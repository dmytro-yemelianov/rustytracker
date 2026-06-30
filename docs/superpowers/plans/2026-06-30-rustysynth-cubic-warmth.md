# RustySynth Cubic Resampling + Warmth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the `RustySynth` mixer mode a distinct sound — 4-point Catmull-Rom cubic resampling plus a master-bus warmth stage (tanh soft-clip → one-pole low-pass) — while leaving HiFi/Amiga/ProTracker unchanged.

**Architecture:** Split the mixer's interpolation selector from a bool into a 3-way `Interpolation { Stepped, Linear, Cubic }`; add a cubic path. Add a stateful `MasterWarmth` to `Mixer`, applied to the summed stereo/mono frame only when the mode is RustySynth. The HiFi linear path is left literally untouched to preserve byte-identical output.

**Tech Stack:** Rust (workspace, edition 2021), `rustytracker-play` mixer engine.

## Global Constraints

- HiFi output must remain **byte-identical** to today: HiFi → `Interpolation::Linear` (the existing `get_sample_value_linear` / `next_frame_index` path, untouched) and HiFi takes the no-warmth branch. Amiga/ProTracker (`Stepped`) also unchanged.
- Cubic and warmth apply to **RustySynth only**.
- No new external dependencies.
- Soft-clip default `DRIVE = 1.0` (`tanh(DRIVE·x_n)` on normalized `x_n = raw/32768`); low-pass default `CUTOFF_HZ = 12_000.0`; both are ear-tuned named constants in `warmth.rs`.
- Cubic = Catmull-Rom over `[frame-1, frame, frame+1, frame+2]`; exact sample at `t=0`; non-looping taps past the end → `0.0`, before frame 0 → clamp to frame 0; looping taps use forward-modulo wrap (consistent with the existing linear `next_frame_index`).
- Warmth chain order: soft-clip → low-pass.
- The previously-verified `rustysynth == hifi` WAV equality breaks **on purpose**; HiFi-vs-its-old-self must not.

---

### Task 1: Cubic resampling + `interpolation()` selector

**Files:**
- Modify: `crates/rustytracker-play/src/lib.rs` (mixer-mode impl ~line 94-101; `get_sample_value` ~1101-1113; add helpers after `next_frame_index` ~1151; add a `#[cfg(test)] mod tests` at end of file)
- Test: `crates/rustytracker-play/tests/mixer_mode.rs` (metadata), `crates/rustytracker-play/tests/preview.rs` (behavioral)

**Interfaces:**
- Produces: `pub enum Interpolation { Stepped, Linear, Cubic }`; `PlaybackMixerMode::interpolation(self) -> Interpolation` (HiFi→Linear, RustySynth→Cubic, Amiga|ProTracker→Stepped). Removes private `uses_linear_interpolation`.
- Internal: `fn tap_index(frame: usize, offset: i64, sample: &Sample) -> Option<usize>`, `fn catmull_rom(p0,p1,p2,p3,t: f64) -> f64`, `fn get_sample_value_cubic(data, frame, fraction, sample) -> f64`.

- [ ] **Step 1: Write failing unit tests for the pure cubic math (in lib.rs)**

Append to the END of `crates/rustytracker-play/src/lib.rs`:

```rust
#[cfg(test)]
mod cubic_tests {
    use super::*;
    use rustytracker_core::{Sample, SampleData, SampleLoopKind};

    fn ramp_sample(len: usize, looped: bool) -> Sample {
        let mut s = Sample::default();
        s.data = SampleData::pcm16((0..len as i16).collect());
        if looped {
            s.loop_kind = SampleLoopKind::Forward;
            s.loop_start = 2;
            s.loop_length = (len as u32).saturating_sub(2);
        }
        s
    }

    #[test]
    fn catmull_rom_endpoints() {
        assert!((catmull_rom(0.0, 7.0, 9.0, 3.0, 0.0) - 7.0).abs() < 1e-9);
        assert!((catmull_rom(0.0, 7.0, 9.0, 3.0, 1.0) - 9.0).abs() < 1e-9);
    }

    #[test]
    fn catmull_rom_known_midpoint() {
        // p0=0,p1=1,p2=1,p3=0 at t=0.5 => 1.125 (curvature overshoot)
        assert!((catmull_rom(0.0, 1.0, 1.0, 0.0, 0.5) - 1.125).abs() < 1e-9);
    }

    #[test]
    fn tap_index_non_looping_clamps_and_ends() {
        let s = ramp_sample(8, false);
        assert_eq!(tap_index(0, -1, &s), Some(0)); // before start clamps to 0
        assert_eq!(tap_index(3, 1, &s), Some(4));
        assert_eq!(tap_index(7, 1, &s), None); // past end -> None (caller uses 0.0)
        assert_eq!(tap_index(7, 2, &s), None);
    }

    #[test]
    fn tap_index_forward_loop_wraps() {
        // len 8, loop_start 2, loop_length 6 => loop_end 8
        let s = ramp_sample(8, true);
        assert_eq!(tap_index(7, 1, &s), Some(2)); // wraps to loop_start
        assert_eq!(tap_index(7, 2, &s), Some(3));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustytracker-play cubic_tests`
Expected: FAIL to compile — `catmull_rom`, `tap_index` not found.

- [ ] **Step 3: Add the `Interpolation` enum and `interpolation()` method**

In `crates/rustytracker-play/src/lib.rs`, add the enum just above `impl PlaybackMixerMode` (i.e. right after the `PlaybackMixerMode` enum definition, before line 63 `impl PlaybackMixerMode {`):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Stepped,
    Linear,
    Cubic,
}
```

Replace the `uses_linear_interpolation` method (lines 98-100) with:

```rust
    pub fn interpolation(self) -> Interpolation {
        match self {
            Self::HiFi => Interpolation::Linear,
            Self::RustySynth => Interpolation::Cubic,
            Self::Amiga | Self::ProTracker => Interpolation::Stepped,
        }
    }
```

- [ ] **Step 4: Switch `get_sample_value` on the interpolation kind**

Replace `get_sample_value` (lines 1101-1113) with:

```rust
fn get_sample_value(
    data: &SampleData,
    frame: usize,
    fraction: u32,
    sample: &Sample,
    mixer_mode: PlaybackMixerMode,
) -> f64 {
    match mixer_mode.interpolation() {
        Interpolation::Linear => get_sample_value_linear(data, frame, fraction, sample),
        Interpolation::Cubic => get_sample_value_cubic(data, frame, fraction, sample),
        Interpolation::Stepped => sample_value_as_f64(data, frame),
    }
}
```

(`get_sample_value_linear` and `next_frame_index` are left UNTOUCHED — this preserves HiFi byte-identical.)

- [ ] **Step 5: Add the cubic helpers**

Insert AFTER `next_frame_index` (after its closing brace, ~line 1151) and before `fn sample_value_as_f64`:

```rust
/// Loop-aware sample index `offset` frames from `frame`.
/// Generalizes `next_frame_index` (which is `tap_index(frame, 1, _)`) to
/// arbitrary offsets for cubic interpolation. Kept separate so the linear
/// path stays byte-identical. `None` means "past the end of a non-looping
/// sample" (caller treats it as 0.0); negative targets clamp to frame 0.
fn tap_index(frame: usize, offset: i64, sample: &Sample) -> Option<usize> {
    let frame_count = sample.data.frame_count();
    if frame_count == 0 {
        return None;
    }
    let target = frame as i64 + offset;
    if target < 0 {
        return Some(0);
    }
    let target = target as usize;
    let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
    if has_loop {
        let loop_start = sample.loop_start as usize;
        let loop_length = sample.loop_length as usize;
        let loop_end = loop_start + loop_length;
        if target >= loop_end {
            Some(loop_start + (target - loop_end) % loop_length)
        } else {
            Some(target)
        }
    } else if target >= frame_count {
        None
    } else {
        Some(target)
    }
}

fn tap_value(data: &SampleData, frame: usize, offset: i64, sample: &Sample) -> f64 {
    match tap_index(frame, offset, sample) {
        Some(index) => sample_value_as_f64(data, index),
        None => 0.0,
    }
}

fn catmull_rom(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
}

fn get_sample_value_cubic(data: &SampleData, frame: usize, fraction: u32, sample: &Sample) -> f64 {
    let t = fraction as f64 / u32::MAX as f64;
    let p0 = tap_value(data, frame, -1, sample);
    let p1 = sample_value_as_f64(data, frame);
    let p2 = tap_value(data, frame, 1, sample);
    let p3 = tap_value(data, frame, 2, sample);
    catmull_rom(p0, p1, p2, p3, t)
}
```

- [ ] **Step 6: Run the unit tests to verify they pass**

Run: `cargo test -p rustytracker-play cubic_tests`
Expected: PASS (4 tests).

- [ ] **Step 7: Add metadata + behavioral tests**

Append to `crates/rustytracker-play/tests/mixer_mode.rs`:

```rust
#[test]
fn mixer_modes_report_interpolation_kind() {
    use rustytracker_play::Interpolation;
    assert_eq!(PlaybackMixerMode::HiFi.interpolation(), Interpolation::Linear);
    assert_eq!(
        PlaybackMixerMode::RustySynth.interpolation(),
        Interpolation::Cubic
    );
    assert_eq!(
        PlaybackMixerMode::Amiga.interpolation(),
        Interpolation::Stepped
    );
    assert_eq!(
        PlaybackMixerMode::ProTracker.interpolation(),
        Interpolation::Stepped
    );
}
```

Append to `crates/rustytracker-play/tests/preview.rs` (reuses `module_with_preview_sample`):

```rust
#[test]
fn rustysynth_cubic_differs_from_hifi_linear_on_a_curved_sample() {
    // A parabola is non-linear, so cubic interpolation diverges from linear.
    let data: Vec<i16> = (0..256).map(|i| ((i * i) / 8) as i16).collect();
    let module = module_with_preview_sample(SampleData::pcm16(data));

    let render = |mode: PlaybackMixerMode| -> Vec<(i32, i32)> {
        let mut voice = PreviewVoice::new();
        voice
            .note_on(
                &module,
                PREVIEW_TEST_INSTRUMENT,
                PREVIEW_TEST_NOTE,
                PlaybackSettings::with_mixer_mode(mode),
            )
            .unwrap();
        (0..8)
            .map(|_| {
                voice
                    .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
                    .unwrap()
            })
            .collect()
    };

    assert_ne!(
        render(PlaybackMixerMode::HiFi),
        render(PlaybackMixerMode::RustySynth),
        "RustySynth cubic should differ from HiFi linear on a curved sample"
    );
}
```

- [ ] **Step 8: Run the play test suite**

Run: `cargo test -p rustytracker-play`
Expected: PASS (existing tests, the 4 cubic unit tests, the metadata test, and the new behavioral test). The existing `preview_voice_honors_mixer_mode` (HiFi vs Amiga) still passes.

- [ ] **Step 9: Commit**

```bash
git add crates/rustytracker-play/src/lib.rs crates/rustytracker-play/tests/mixer_mode.rs crates/rustytracker-play/tests/preview.rs
git commit -m "Add cubic resampling for RustySynth via interpolation() selector"
```

---

### Task 2: Master warmth (soft-clip + low-pass), RustySynth-only

**Files:**
- Create: `crates/rustytracker-play/src/warmth.rs`
- Modify: `crates/rustytracker-play/src/lib.rs` (`mod warmth;`; `uses_warmth()` on `PlaybackMixerMode`; `Mixer` struct + `new` + both render fns; drop `Eq` from `Mixer` and `PlaybackState` derives)
- Test: warmth.rs unit tests; `crates/rustytracker-play/tests/mixer_mode.rs`; `crates/rustytracker-play/tests/preview.rs`

**Interfaces:**
- Consumes: nothing from Task 1 directly (parallel concern).
- Produces: `MasterWarmth::new()`, `process(&mut self, l: f64, r: f64, sample_rate: u32) -> (f64, f64)`, `process_mono(&mut self, x: f64, sample_rate: u32) -> f64`; `PlaybackMixerMode::uses_warmth(self) -> bool` (true only for RustySynth).

- [ ] **Step 1: Write `warmth.rs` with failing unit tests**

Create `crates/rustytracker-play/src/warmth.rs`:

```rust
//! Master-bus "warmth" for RustySynth: tanh soft-clip into a one-pole low-pass.
//! Applied to the summed stereo/mono frame; RustySynth-only.

const DRIVE: f64 = 1.0; // soft-clip drive (ear-tuned)
const CUTOFF_HZ: f64 = 12_000.0; // one-pole low-pass cutoff (ear-tuned)
const PCM_SCALE: f64 = 32_768.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MasterWarmth {
    lp_l: f64,
    lp_r: f64,
    lp_coeff: f64,
    coeff_sample_rate: u32,
}

impl Default for MasterWarmth {
    fn default() -> Self {
        Self::new()
    }
}

impl MasterWarmth {
    pub fn new() -> Self {
        Self {
            lp_l: 0.0,
            lp_r: 0.0,
            lp_coeff: 0.0,
            coeff_sample_rate: 0,
        }
    }

    fn update_coeff(&mut self, sample_rate: u32) {
        if sample_rate != self.coeff_sample_rate && sample_rate > 0 {
            self.lp_coeff =
                1.0 - (-2.0 * std::f64::consts::PI * CUTOFF_HZ / sample_rate as f64).exp();
            self.coeff_sample_rate = sample_rate;
        }
    }

    pub fn process(&mut self, l: f64, r: f64, sample_rate: u32) -> (f64, f64) {
        self.update_coeff(sample_rate);
        let sl = soft_clip(l);
        let sr = soft_clip(r);
        self.lp_l += self.lp_coeff * (sl - self.lp_l);
        self.lp_r += self.lp_coeff * (sr - self.lp_r);
        (self.lp_l, self.lp_r)
    }

    pub fn process_mono(&mut self, x: f64, sample_rate: u32) -> f64 {
        self.update_coeff(sample_rate);
        let s = soft_clip(x);
        self.lp_l += self.lp_coeff * (s - self.lp_l);
        self.lp_l
    }
}

fn soft_clip(raw: f64) -> f64 {
    let x = raw / PCM_SCALE;
    (DRIVE * x).tanh() * PCM_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_clip_is_odd_and_zero_at_origin() {
        assert_eq!(soft_clip(0.0), 0.0);
        assert!((soft_clip(-5000.0) + soft_clip(5000.0)).abs() < 1e-9);
    }

    #[test]
    fn soft_clip_near_unity_for_small_and_compresses_large() {
        // small input ~ passes through
        assert!((soft_clip(100.0) - 100.0).abs() < 1.0);
        // full-scale input is compressed below itself
        assert!(soft_clip(32_768.0) < 32_768.0);
        assert!(soft_clip(32_768.0) > 20_000.0);
    }

    #[test]
    fn low_pass_passes_dc_and_attenuates_alternation() {
        let sr = 44_100;
        // DC: feed a small constant; output converges to it.
        let mut w = MasterWarmth::new();
        let mut out = 0.0;
        for _ in 0..200 {
            out = w.process_mono(100.0, sr);
        }
        assert!((out - soft_clip(100.0)).abs() < 1.0);

        // Alternation: fast +/- swings come out attenuated in amplitude.
        let mut w2 = MasterWarmth::new();
        let mut peak = 0.0_f64;
        for n in 0..200 {
            let x = if n % 2 == 0 { 5000.0 } else { -5000.0 };
            let y = w2.process_mono(x, sr);
            if n > 100 {
                peak = peak.max(y.abs());
            }
        }
        assert!(peak < 5000.0, "alternation should be attenuated, got {peak}");
    }
}
```

- [ ] **Step 2: Run the warmth unit tests to verify they fail**

Run: `cargo test -p rustytracker-play warmth`
Expected: FAIL to compile — `warmth` module not declared in the crate yet.

- [ ] **Step 3: Declare the module and add `uses_warmth()`**

In `crates/rustytracker-play/src/lib.rs`, add to the module declarations (next to `mod preview;`, near the top):

```rust
mod warmth;
```

Add an internal import near the top of `lib.rs` (with the other `use` lines) so `Mixer` can construct it — `MasterWarmth` stays crate-internal (it never leaks through `Mixer`'s public API, since the field is private), so do NOT `pub use` it:

```rust
use warmth::MasterWarmth;
```

Add a method to `impl PlaybackMixerMode` (right after `interpolation`):

```rust
    pub fn uses_warmth(self) -> bool {
        matches!(self, Self::RustySynth)
    }
```

- [ ] **Step 4: Run the warmth unit tests to verify they pass**

Run: `cargo test -p rustytracker-play warmth`
Expected: PASS (3 tests).

- [ ] **Step 5: Add `MasterWarmth` to `Mixer` and drop `Eq` where f64 lands**

In `crates/rustytracker-play/src/lib.rs`:

Change the `Mixer` derive + struct (line ~513-516) from:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mixer {
    pub voices: Vec<MixerVoice>,
}
```

to:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Mixer {
    pub voices: Vec<MixerVoice>,
    warmth: MasterWarmth,
}
```

Change `Mixer::new` (line ~519-524) to initialize the field:

```rust
    pub fn new(channel_count: usize) -> Self {
        let voices = (0..channel_count)
            .map(|ch| MixerVoice::empty(ch as u16))
            .collect();
        Self {
            voices,
            warmth: MasterWarmth::new(),
        }
    }
```

Change the `PlaybackState` derive (line ~767) from `#[derive(Debug, Clone, PartialEq, Eq)]` to `#[derive(Debug, Clone, PartialEq)]` (it contains `Mixer`, which is no longer `Eq`). Leave `Sequencer` and `MixerVoice` derives as-is (they contain no `f64`).

- [ ] **Step 6: Apply warmth in both render functions**

In `render_stereo_frame`, replace the final `Ok((mixed_l as i32, mixed_r as i32))` (line ~662) with:

```rust
        let (out_l, out_r) = if mixer_mode.uses_warmth() {
            self.warmth.process(mixed_l, mixed_r, sample_rate)
        } else {
            (mixed_l, mixed_r)
        };

        Ok((out_l as i32, out_r as i32))
```

In `render_mono_frame`, replace the final `Ok(mixed)` (line ~720) with:

```rust
        let out = if mixer_mode.uses_warmth() {
            self.warmth.process_mono(mixed as f64, sample_rate)
        } else {
            mixed as f64
        };

        Ok(out as i32)
```

- [ ] **Step 7: Add `uses_warmth` metadata + behavioral warmth tests**

Append to `crates/rustytracker-play/tests/mixer_mode.rs`:

```rust
#[test]
fn only_rustysynth_uses_warmth() {
    assert!(PlaybackMixerMode::RustySynth.uses_warmth());
    assert!(!PlaybackMixerMode::HiFi.uses_warmth());
    assert!(!PlaybackMixerMode::Amiga.uses_warmth());
    assert!(!PlaybackMixerMode::ProTracker.uses_warmth());
}
```

Append to `crates/rustytracker-play/tests/preview.rs`:

```rust
#[test]
fn rustysynth_warmth_compresses_a_loud_frame_hifi_does_not() {
    // A loud, steady full-scale sample: HiFi passes the peak; RustySynth
    // soft-clips it below full scale.
    let module = module_with_preview_sample(SampleData::pcm16(vec![32_000; 64]));

    let first_left = |mode: PlaybackMixerMode| -> i32 {
        let mut voice = PreviewVoice::new();
        voice
            .note_on(
                &module,
                PREVIEW_TEST_INSTRUMENT,
                PREVIEW_TEST_NOTE,
                PlaybackSettings::with_mixer_mode(mode),
            )
            .unwrap();
        // A few frames so the warmth low-pass settles toward the level.
        let mut l = 0;
        for _ in 0..32 {
            l = voice
                .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
                .unwrap()
                .0;
        }
        l
    };

    let hifi = first_left(PlaybackMixerMode::HiFi).abs();
    let rusty = first_left(PlaybackMixerMode::RustySynth).abs();
    assert!(
        rusty < hifi,
        "RustySynth warmth should compress the loud frame below HiFi (hifi={hifi}, rusty={rusty})"
    );
}
```

- [ ] **Step 8: Run the play suite**

Run: `cargo test -p rustytracker-play`
Expected: PASS (all prior tests + warmth unit tests + the two new mode/behavioral tests). Existing HiFi-default preview tests are unaffected (HiFi takes the no-warmth branch).

- [ ] **Step 9: Commit**

```bash
git add crates/rustytracker-play/src/warmth.rs crates/rustytracker-play/src/lib.rs crates/rustytracker-play/tests/mixer_mode.rs crates/rustytracker-play/tests/preview.rs
git commit -m "Add RustySynth master warmth: tanh soft-clip into one-pole low-pass"
```

---

### Task 3: Folded cleanups (deferred review Minors)

**Files:**
- Modify: `crates/rustytracker-ui/src/input.rs` (edit-mode note loop)
- Modify: `crates/rustytracker-play/src/preview.rs` (`note_on` settings ordering)

**Interfaces:** none new.

These are behavior-preserving. Verify with build + the play suite; no new tests.

- [ ] **Step 1: Reuse `note_value_for_key` in the edit-mode write loop**

In `crates/rustytracker-ui/src/input.rs`, the edit-mode note loop currently recomputes the note inline:

```rust
                        if input.key_pressed(key) {
                            if let Some((note_name, octave_offset)) =
                                key_to_note_and_octave_offset(key)
                            {
                                let final_octave =
                                    (self.octave as i8 + octave_offset).clamp(0, 8) as u8;
                                if let Ok(note) = Note::key(final_octave, note_name) {
                                    let active_pattern_idx = self.get_active_pattern_index();

                                    // Write note
                                    let _ = self.editor.set_note(
                                        active_pattern_idx,
                                        self.active_channel,
                                        self.active_row,
                                        note,
                                    );
```

Replace the inline note computation so it reuses the helper (which returns the `u8` value), constructing the `Note` from it:

```rust
                        if input.key_pressed(key) {
                            if let Some(value) = self.note_value_for_key(key) {
                                let note = Note::Key(value);
                                let active_pattern_idx = self.get_active_pattern_index();

                                // Write note
                                let _ = self.editor.set_note(
                                    active_pattern_idx,
                                    self.active_channel,
                                    self.active_row,
                                    note,
                                );
```

Keep the rest of the loop body (set_instrument, commit_edit_to_audio, advance_row_after_edit, and the closing braces) intact — only the note-resolution head of the `if input.key_pressed(key)` block changes. After this, the `key_to_note_and_octave_offset` import/use inside this loop is gone; the function is still used by `note_value_for_key`, so no dead-code warning.

Re-indent the inner block to match (one fewer nesting level). Run `cargo fmt` after editing.

- [ ] **Step 2: Move the settings assignment in `preview.rs::note_on`**

In `crates/rustytracker-play/src/preview.rs`, `note_on` currently sets `self.settings = settings;` before `apply_cell`. Move it to AFTER `apply_cell` succeeds. The body becomes:

```rust
    pub fn note_on(
        &mut self,
        module: &Module,
        instrument: u8,
        note: u8,
        settings: PlaybackSettings,
    ) -> PlaybackResult<()> {
        // Mono: stop whatever was playing before resolving the new note.
        self.mixer.handle_commands(&[SequencerCommand::Stop {
            channel: PREVIEW_CHANNEL,
        }]);

        let cell = PatternCell {
            note: Note::Key(note),
            instrument,
            ..PatternCell::default()
        };
        self.channels[0].apply_cell(module, &cell)?;
        self.settings = settings;

        if self.channels[0].active {
```

(Keep the remainder of `note_on` unchanged. Now a failed `apply_cell` returns early via `?` without having mutated `self.settings`.)

- [ ] **Step 3: Build and run the play suite**

Run: `cargo build -p rustytracker-ui && cargo test -p rustytracker-play`
Expected: both clean; all play tests still pass (the settings move is behavior-preserving — the existing `preview_voice_missing_instrument_stays_inactive` test still passes).

- [ ] **Step 4: Commit**

```bash
git add crates/rustytracker-ui/src/input.rs crates/rustytracker-play/src/preview.rs
git commit -m "Cleanup: reuse note_value_for_key; set preview settings after apply_cell"
```

---

### Task 4: Workspace verification + CLI character check

**Files:** none (verification).

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`
Expected: no diff. If it reports changes, run `cargo fmt --all` and stage them into a follow-up commit.

- [ ] **Step 2: Clippy**

Run: `cargo clippy --workspace --all-targets`
Expected: 0 errors; no NEW warnings from `rustytracker-play` (warmth.rs / lib.rs) or the touched UI file. (Note: `MasterWarmth` needs its `Default` impl — already included — to avoid `clippy::new_without_default`.)

- [ ] **Step 3: Full test suite**

Run: `cargo test --workspace`
Expected: PASS. Fixture-gated tests skip without `MILKYTRACKER_ROOT` (expected).

- [ ] **Step 4: CLI character check — RustySynth now differs from HiFi, HiFi unchanged**

Run (uses a sibling fixture module):

```bash
BIN=target/debug/rustytracker
IN=../MilkyTracker/resources/music/milky.xm
OUT=$(mktemp -d)
cargo build -p rustytracker-cli
"$BIN" export-wav "$IN" "$OUT/hifi.wav" --mixer hifi
"$BIN" export-wav "$IN" "$OUT/rusty.wav" --mixer rustysynth
cmp "$OUT/hifi.wav" "$OUT/rusty.wav" && echo "IDENTICAL (BUG)" || echo "DIFFER (expected ✅)"
```

Expected: `DIFFER (expected ✅)` — RustySynth now has its own character.
(If `../MilkyTracker` is unavailable, substitute any `.mod`/`.xm` path; the check is the `DIFFER` result.)

- [ ] **Step 5: Final commit (only if fmt produced changes)**

```bash
git add -A
git commit -m "Apply rustfmt after RustySynth cubic + warmth"
```

---

## Self-Review

**Spec coverage:**
- Cubic resampling (Catmull-Rom, loop-aware taps, `t=0` exact, end/edge handling) → Task 1. ✓
- `interpolation()` 3-way replacing `uses_linear_interpolation`; HiFi=Linear untouched → Task 1. ✓
- `MasterWarmth` (tanh soft-clip → one-pole LPF, RustySynth-only, `process`/`process_mono`, lazy coeff) → Task 2. ✓
- `uses_warmth()`; wiring into both render fns; `Eq` dropped from `Mixer`/`PlaybackState` → Task 2. ✓
- HiFi byte-identical (linear path untouched + no-warmth branch) → guaranteed by Task 1 Step 4/5 and Task 2 Step 6; checked end-to-end in Task 4 Step 4. ✓
- Tunable defaults (`DRIVE`, `CUTOFF_HZ` named constants) → Task 2 Step 1. ✓
- Folded cleanups → Task 3. ✓
- Tests: cubic math, tap_index, interpolation metadata, RustySynth≠HiFi on a curve, soft-clip/LPF unit tests, uses_warmth metadata, warmth compresses loud frame, preview inherits character (via PreviewVoice render) → Tasks 1-2. ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code. The CLI check names an exact fallback if the fixture is absent.

**Type consistency:** `Interpolation` variants (`Stepped`/`Linear`/`Cubic`) consistent between the enum (Task 1 Step 3), `get_sample_value` (Step 4), and the metadata test (Step 7). `MasterWarmth::{new,process,process_mono}` signatures consistent between `warmth.rs` (Task 2 Step 1) and the render wiring (Step 6). `tap_index`/`tap_value`/`catmull_rom`/`get_sample_value_cubic` signatures consistent between definition (Task 1 Step 5) and the unit tests (Step 1). `uses_warmth()` consistent between definition (Task 2 Step 3) and use (Step 6) and tests (Step 7).
