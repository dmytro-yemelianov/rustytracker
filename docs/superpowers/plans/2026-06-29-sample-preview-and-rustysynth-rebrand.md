# Sample Preview & RustySynth Rebrand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the project-owned `MilkyTracker` mixer mode to `RustySynth`, and add live monophonic sample preview (keyboard jam + click) to the desktop UI.

**Architecture:** Add a `PreviewVoice` type to `rustytracker-play` that reuses the existing `Mixer`/`MixerVoice`/`PlaybackChannelState` engine so preview honors the selected mixer mode. The desktop audio thread mixes a preview frame on top of the module frame (preview works whether or not the song is playing). UI note keys and instrument-list clicks drive the preview through new `AudioPlaybackEngine` commands.

**Tech Stack:** Rust (workspace, edition 2021), `eframe`/`egui` (desktop UI), `cpal` + `rtrb` (audio output), existing `rustytracker-play` playback engine.

## Global Constraints

- No new external dependencies; reuse the existing engine and crates.
- HiFi playback/export output must remain byte-identical (the rebrand and preview must not alter HiFi rendering).
- The rebrand renames ONLY the project-owned mixer mode. Do NOT rename anything that refers to the real MilkyTracker program: `rustytracker-test-support` (`MILKYTRACKER_ROOT`, `milkytracker_fixture_*`), test names like `export_wav_uses_milkytracker_mod_pitch_clock` / `render_to_wav_uses_milkytracker_amiga_tick_clock` / `parses_milkytracker_bundled_xm_headers`, `tracker_name == "MilkyTracker"` assertions, the `// Correct loops like MilkyTracker does:` comment, and all README references.
- Preview is monophonic, sustain-while-held; key release stops the note.
- Click-preview pitch is C-4.
- No back-compat `milkytracker` alias is kept for the renamed mode.
- Exact note value for C-4 = `49` (`Note::key(4, NoteName::C)` → `Note::Key(49)`); instrument numbers are 1-based.

---

### Task 1: Rebrand MilkyTracker mixer mode → RustySynth

**Files:**
- Modify: `crates/rustytracker-play/src/lib.rs` (lines 52-104: enum, `ALL`, `label`, `cli_name`, `from_name`, `uses_linear_interpolation`)
- Modify: `crates/rustytracker-cli/src/lib.rs:35` (usage string)
- Test: `crates/rustytracker-play/tests/mixer_mode.rs` (new)

**Interfaces:**
- Produces: `PlaybackMixerMode::RustySynth` variant; `RustySynth.cli_name() == "rustysynth"`; `RustySynth.label() == "RustySynth"`; `PlaybackMixerMode::from_name("rustysynth" | "rusty" | "rs") == Some(RustySynth)`; `from_name("milkytracker") == None`.

- [ ] **Step 1: Write the failing test**

Create `crates/rustytracker-play/tests/mixer_mode.rs`:

```rust
use rustytracker_play::PlaybackMixerMode;

#[test]
fn rustysynth_replaces_milkytracker_mode() {
    let mode = PlaybackMixerMode::from_name("rustysynth").unwrap();
    assert_eq!(mode, PlaybackMixerMode::RustySynth);
    assert_eq!(mode.cli_name(), "rustysynth");
    assert_eq!(mode.label(), "RustySynth");

    // Short aliases still resolve.
    assert_eq!(
        PlaybackMixerMode::from_name("rusty"),
        Some(PlaybackMixerMode::RustySynth)
    );
    assert_eq!(
        PlaybackMixerMode::from_name("rs"),
        Some(PlaybackMixerMode::RustySynth)
    );

    // The old program name no longer maps to the project synth mode.
    assert_eq!(PlaybackMixerMode::from_name("milkytracker"), None);

    // RustySynth is part of the selectable set; HiFi stays the default.
    assert!(PlaybackMixerMode::ALL.contains(&PlaybackMixerMode::RustySynth));
    assert_eq!(PlaybackMixerMode::default(), PlaybackMixerMode::HiFi);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustytracker-play --test mixer_mode`
Expected: FAIL to compile — `no variant named RustySynth found for enum PlaybackMixerMode`.

- [ ] **Step 3: Apply the rename in `rustytracker-play/src/lib.rs`**

In the `PlaybackMixerMode` enum (around line 53-59), rename the variant:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackMixerMode {
    #[default]
    HiFi,
    RustySynth,
    Amiga,
    ProTracker,
}
```

In `ALL` (around line 62-67):

```rust
    pub const ALL: [Self; 4] = [
        Self::HiFi,
        Self::RustySynth,
        Self::Amiga,
        Self::ProTracker,
    ];
```

In `label()` (around line 69-76) change the arm:

```rust
            Self::RustySynth => "RustySynth",
```

In `cli_name()` (around line 78-85) change the arm:

```rust
            Self::RustySynth => "rustysynth",
```

In `from_name()` (around line 87-95) change the arm:

```rust
            "rustysynth" | "rusty" | "rs" => Some(Self::RustySynth),
```

In `uses_linear_interpolation()` (around line 101-103):

```rust
    fn uses_linear_interpolation(self) -> bool {
        matches!(self, Self::HiFi | Self::RustySynth)
    }
}
```

- [ ] **Step 4: Update the CLI usage string in `rustytracker-cli/src/lib.rs:35`**

```rust
    "[--sample-rate <rate>] [--mixer <hifi|rustysynth|amiga|protracker>]"
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p rustytracker-play --test mixer_mode`
Expected: PASS.

Run: `cargo test -p rustytracker-cli`
Expected: PASS (CLI smoke / round-trip still green; no test referenced the old mode name).

- [ ] **Step 6: Commit**

```bash
git add crates/rustytracker-play/src/lib.rs crates/rustytracker-cli/src/lib.rs crates/rustytracker-play/tests/mixer_mode.rs
git commit -m "Rename MilkyTracker mixer mode to RustySynth"
```

---

### Task 2: `PreviewVoice` in rustytracker-play

**Files:**
- Create: `crates/rustytracker-play/src/preview.rs`
- Modify: `crates/rustytracker-play/src/lib.rs` (add `mod preview;` near line 10; add `pub use preview::PreviewVoice;`)
- Test: `crates/rustytracker-play/tests/preview.rs` (new)

**Interfaces:**
- Consumes (crate-internal): `PlaybackChannelState::empty(u16)` and `PlaybackChannelState::apply_cell(&Module, &PatternCell)` (both `pub(crate)`); `Mixer::new(usize)`, `Mixer::handle_commands(&[SequencerCommand])`, `Mixer::render_stereo_frame(&Module, u32, &mut [PlaybackChannelState], PlaybackMixerMode)`; `SequencerCommand::{Trigger, Stop}`; `MixerVoice.active` (pub field via `Mixer.voices`).
- Produces:
  ```rust
  pub struct PreviewVoice { /* private */ }
  impl PreviewVoice {
      pub fn new() -> Self;
      pub fn note_on(&mut self, module: &Module, instrument: u8, note: u8, settings: PlaybackSettings) -> PlaybackResult<()>;
      pub fn note_off(&mut self);
      pub fn is_active(&self) -> bool;
      pub fn render_stereo_frame(&mut self, module: &Module, sample_rate: u32) -> PlaybackResult<RawStereoPcmFrame>;
  }
  impl Default for PreviewVoice { fn default() -> Self; }
  ```

- [ ] **Step 1: Write the failing tests**

Create `crates/rustytracker-play/tests/preview.rs`:

```rust
use rustytracker_core::{FrequencyTable, Module, SampleData};
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PreviewVoice};

const PREVIEW_TEST_INSTRUMENT: u8 = 1;
const PREVIEW_TEST_NOTE: u8 = 49; // C-4
const PREVIEW_TEST_SAMPLE_RATE: u32 = 44_100;

fn module_with_preview_sample(data: SampleData) -> Module {
    let mut module = Module::empty_with_channels(2).unwrap();
    module.header.frequency_table = FrequencyTable::Linear;
    let map_len = module.instruments[0].note_sample_map.len().max(96);
    module.instruments[0].note_sample_map = vec![Some(0); map_len];
    module.samples[0].volume = 255;
    module.samples[0].data = data;
    module
}

#[test]
fn preview_voice_is_silent_before_note_on() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![1000; 64]));
    let mut voice = PreviewVoice::new();
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_note_on_produces_output() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    assert!(voice.is_active());
    let (l, r) = voice
        .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
        .unwrap();
    assert!(l != 0 || r != 0, "expected audible preview output");
}

#[test]
fn preview_voice_note_off_stops_voice() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    voice.note_off();
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_missing_instrument_stays_inactive() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    let missing = (module.instruments.len() as u8) + 1;
    let result = voice.note_on(
        &module,
        missing,
        PREVIEW_TEST_NOTE,
        PlaybackSettings::default(),
    );
    assert!(result.is_err());
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_non_looping_sample_stops_after_end() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000, 9_000, 8_000, 7_000]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    assert!(voice.is_active());
    for _ in 0..200 {
        let _ = voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap();
    }
    assert!(
        !voice.is_active(),
        "non-looping preview should stop after the sample ends"
    );
}

#[test]
fn preview_voice_honors_mixer_mode() {
    let data: Vec<i16> = (0..256).map(|i| (i * 100) as i16).collect();
    let module = module_with_preview_sample(SampleData::pcm16(data));

    let render_n = |mode: PlaybackMixerMode| -> Vec<(i32, i32)> {
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

    let hifi = render_n(PlaybackMixerMode::HiFi);
    let amiga = render_n(PlaybackMixerMode::Amiga);
    assert_ne!(
        hifi, amiga,
        "HiFi (interpolated) and Amiga (stepped) fetch should differ on a ramp sample"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p rustytracker-play --test preview`
Expected: FAIL to compile — `no PreviewVoice in the root` / unresolved import.

- [ ] **Step 3: Create `crates/rustytracker-play/src/preview.rs`**

```rust
use crate::channel::PlaybackChannelState;
use crate::error::PlaybackResult;
use crate::{
    Mixer, PlaybackSettings, RawStereoPcmFrame, SequencerCommand, PLAYBACK_STEREO_SILENCE,
};
use rustytracker_core::{Module, Note, PatternCell};

const PREVIEW_CHANNEL: u16 = 0;

/// A single, monophonic preview voice for auditioning an instrument/sample
/// outside of song playback. Reuses the shared mixer engine so preview honors
/// the selected mixer mode.
#[derive(Debug, Clone)]
pub struct PreviewVoice {
    mixer: Mixer,
    channels: Vec<PlaybackChannelState>,
    settings: PlaybackSettings,
}

impl Default for PreviewVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewVoice {
    pub fn new() -> Self {
        Self {
            mixer: Mixer::new(1),
            channels: vec![PlaybackChannelState::empty(PREVIEW_CHANNEL)],
            settings: PlaybackSettings::default(),
        }
    }

    /// Trigger a note for the given instrument. Monophonic: any currently
    /// sounding preview note is cut first. On a missing instrument/sample the
    /// voice stays silent and the error is returned.
    pub fn note_on(
        &mut self,
        module: &Module,
        instrument: u8,
        note: u8,
        settings: PlaybackSettings,
    ) -> PlaybackResult<()> {
        self.settings = settings;

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

        if self.channels[0].active {
            if let (Some(sample_index), Some(instrument_index)) =
                (self.channels[0].sample_index, self.channels[0].instrument_index)
            {
                self.mixer.handle_commands(&[SequencerCommand::Trigger {
                    channel: PREVIEW_CHANNEL,
                    sample_index,
                    instrument_index,
                    note: self.channels[0].note,
                    instrument: self.channels[0].instrument,
                    volume: self.channels[0].volume,
                    panning: self.channels[0].panning,
                    period: self.channels[0].period,
                    offset: None,
                }]);
            }
        }

        Ok(())
    }

    pub fn note_off(&mut self) {
        self.mixer.handle_commands(&[SequencerCommand::Stop {
            channel: PREVIEW_CHANNEL,
        }]);
    }

    pub fn is_active(&self) -> bool {
        self.mixer
            .voices
            .first()
            .map(|voice| voice.active)
            .unwrap_or(false)
    }

    pub fn render_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawStereoPcmFrame> {
        if !self.is_active() {
            return Ok(PLAYBACK_STEREO_SILENCE);
        }
        self.mixer.render_stereo_frame(
            module,
            sample_rate,
            &mut self.channels,
            self.settings.mixer_mode,
        )
    }
}
```

- [ ] **Step 4: Wire the module into `crates/rustytracker-play/src/lib.rs`**

Add the module declaration next to the other `mod` lines (after `mod envelope;` / near line 8-11):

```rust
mod preview;
```

Add the public re-export next to the other `pub use` blocks (e.g. after the `pub use envelope::PlaybackEnvelopeState;` line ~30):

```rust
pub use preview::PreviewVoice;
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p rustytracker-play --test preview`
Expected: PASS (all six tests).

Run: `cargo test -p rustytracker-play`
Expected: PASS (existing engine tests unaffected; fixture-gated tests skip if `MILKYTRACKER_ROOT` is unset).

- [ ] **Step 6: Commit**

```bash
git add crates/rustytracker-play/src/preview.rs crates/rustytracker-play/src/lib.rs crates/rustytracker-play/tests/preview.rs
git commit -m "Add PreviewVoice for monophonic sample auditioning"
```

---

### Task 3: Audio-thread preview integration

**Files:**
- Modify: `crates/rustytracker-ui/src/audio.rs` (imports; `AudioCommand`; `AudioThreadState`; command handling + `write_audio` rewrite; `AudioPlaybackEngine` methods; test module)

**Interfaces:**
- Consumes: `PreviewVoice` (Task 2); `PlaybackMixerMode`, `PlaybackSettings` from `rustytracker_play`.
- Produces: `AudioPlaybackEngine::preview_note_on(&self, instrument: u8, note: u8, mixer_mode: PlaybackMixerMode)` and `AudioPlaybackEngine::preview_note_off(&self)`; `AudioCommand::{PreviewNoteOn { instrument, note, mixer_mode }, PreviewNoteOff}`.

- [ ] **Step 1: Write the failing test**

Add this test module at the bottom of `crates/rustytracker-ui/src/audio.rs`, replacing the existing `#[cfg(test)] mod tests { ... }` block by appending the new test inside it (keep `normalize_pcm16_sample_clamps_to_output_range`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rustytracker_core::{FrequencyTable, Module, SampleData};
    use rustytracker_play::PlaybackSettings;

    #[test]
    fn normalize_pcm16_sample_clamps_to_output_range() {
        assert_eq!(normalize_pcm16_sample(0), 0.0);
        assert_eq!(normalize_pcm16_sample(PCM16_MIN), -1.0);
        assert_eq!(
            normalize_pcm16_sample(PCM16_MAX + 1),
            PCM16_MAX as f32 / PCM16_NORMALIZATION
        );
    }

    fn module_with_preview_sample() -> Module {
        let mut module = Module::empty_with_channels(2).unwrap();
        module.header.frequency_table = FrequencyTable::Linear;
        let map_len = module.instruments[0].note_sample_map.len().max(96);
        module.instruments[0].note_sample_map = vec![Some(0); map_len];
        module.samples[0].volume = 255;
        module.samples[0].data = SampleData::pcm16(vec![12_000; 64]);
        module
    }

    #[test]
    fn write_audio_mixes_preview_while_module_stopped() {
        let module = module_with_preview_sample();
        let mut local_state = AudioThreadState {
            playback: None,
            module: Some(module.clone()),
            is_playing: false,
            sample_rate: 44_100,
            preview: PreviewVoice::new(),
        };
        local_state
            .preview
            .note_on(&module, 1, 49, PlaybackSettings::default())
            .unwrap();

        let status = Arc::new(AudioStatus::new());
        let (_producer, mut consumer) = rtrb::RingBuffer::<AudioCommand>::new(4);
        let mut output = vec![0.0f32; 64];

        write_audio(&mut output, &mut consumer, &status, &mut local_state);

        assert!(
            output.iter().any(|&sample| sample != 0.0),
            "preview voice should be mixed into output even when the song is stopped"
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p rustytracker-ui write_audio_mixes_preview_while_module_stopped`
Expected: FAIL to compile — `AudioThreadState` has no field `preview`; `PreviewVoice` not imported.

- [ ] **Step 3: Update imports at the top of `audio.rs`**

Change the `rustytracker_play` import (line 6) to:

```rust
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState, PreviewVoice};
```

- [ ] **Step 4: Extend `AudioCommand` (around line 12-18)**

```rust
pub(crate) enum AudioCommand {
    Play,
    Pause,
    Stop,
    UpdateModule(Module),
    SetPlayback(Option<PlaybackState>),
    PreviewNoteOn {
        instrument: u8,
        note: u8,
        mixer_mode: PlaybackMixerMode,
    },
    PreviewNoteOff,
}
```

- [ ] **Step 5: Add the `preview` field to `AudioThreadState` (around line 36-41)**

```rust
struct AudioThreadState {
    playback: Option<PlaybackState>,
    module: Option<Module>,
    is_playing: bool,
    sample_rate: u32,
    preview: PreviewVoice,
}
```

And initialize it where `local_state_opt` is built (around line 86-91):

```rust
        let mut local_state_opt = Some(AudioThreadState {
            playback: None,
            module: None,
            is_playing: false,
            sample_rate,
            preview: PreviewVoice::new(),
        });
```

- [ ] **Step 6: Add `preview_note_on` / `preview_note_off` to `AudioPlaybackEngine`**

Add these methods inside `impl AudioPlaybackEngine` (next to `stop`/`set_playback`, around line 173-183):

```rust
    pub(crate) fn preview_note_on(&self, instrument: u8, note: u8, mixer_mode: PlaybackMixerMode) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::PreviewNoteOn {
                instrument,
                note,
                mixer_mode,
            });
        }
    }

    pub(crate) fn preview_note_off(&self) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::PreviewNoteOff);
        }
    }
```

- [ ] **Step 7: Rewrite `write_audio` (replace the whole function body, lines ~196-296)**

```rust
fn write_audio<T>(
    output: &mut [T],
    consumer: &mut rtrb::Consumer<AudioCommand>,
    status: &Arc<AudioStatus>,
    local_state: &mut AudioThreadState,
) where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    while let Ok(cmd) = consumer.pop() {
        match cmd {
            AudioCommand::Play => {
                local_state.is_playing = true;
            }
            AudioCommand::Pause => {
                local_state.is_playing = false;
            }
            AudioCommand::Stop => {
                local_state.is_playing = false;
                local_state.playback = None;
            }
            AudioCommand::UpdateModule(module) => {
                local_state.module = Some(module);
            }
            AudioCommand::SetPlayback(playback) => {
                local_state.playback = playback;
            }
            AudioCommand::PreviewNoteOn {
                instrument,
                note,
                mixer_mode,
            } => {
                if let Some(module) = &local_state.module {
                    let _ = local_state.preview.note_on(
                        module,
                        instrument,
                        note,
                        PlaybackSettings::with_mixer_mode(mixer_mode),
                    );
                }
            }
            AudioCommand::PreviewNoteOff => {
                local_state.preview.note_off();
            }
        }
    }

    let sample_rate = local_state.sample_rate;

    // A module is required to render either song playback or preview.
    let module = match local_state.module.as_ref() {
        Some(m) => m,
        None => {
            write_silence(output);
            status.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    let is_playing = local_state.is_playing;
    let playback_opt = &mut local_state.playback;
    let preview = &mut local_state.preview;
    let mut song_ended = false;

    for frame in output.chunks_mut(2) {
        let (module_l, module_r) = if is_playing && !song_ended {
            match playback_opt.as_mut() {
                Some(pb) => match pb.render_raw_stereo_frame(module, sample_rate) {
                    Ok((raw_l, raw_r)) => {
                        if pb.song_ended() {
                            song_ended = true;
                            (0.0, 0.0)
                        } else {
                            (normalize_pcm16_sample(raw_l), normalize_pcm16_sample(raw_r))
                        }
                    }
                    Err(_) => {
                        song_ended = true;
                        (0.0, 0.0)
                    }
                },
                None => (0.0, 0.0),
            }
        } else {
            (0.0, 0.0)
        };

        let (preview_l, preview_r) = match preview.render_stereo_frame(module, sample_rate) {
            Ok((raw_l, raw_r)) => (normalize_pcm16_sample(raw_l), normalize_pcm16_sample(raw_r)),
            Err(_) => {
                preview.note_off();
                (0.0, 0.0)
            }
        };

        let left = (module_l + preview_l).clamp(-1.0, 1.0);
        let right = (module_r + preview_r).clamp(-1.0, 1.0);

        if frame.len() >= 2 {
            frame[0] = T::from_sample(left);
            frame[1] = T::from_sample(right);
        } else if !frame.is_empty() {
            frame[0] = T::from_sample(left);
        }
    }

    if song_ended {
        local_state.is_playing = false;
        local_state.playback = None;
    }

    status
        .is_playing
        .store(local_state.is_playing, Ordering::Relaxed);

    if local_state.is_playing {
        if let (Some(pb), Some(module)) = (&local_state.playback, &local_state.module) {
            if let Ok(pos) = pb.clock().position(module) {
                status.order_index.store(pos.order_index, Ordering::Relaxed);
                status.row.store(pos.row as u32, Ordering::Relaxed);
            }
        }
    }
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p rustytracker-ui`
Expected: PASS (both `normalize_pcm16_sample_clamps_to_output_range` and `write_audio_mixes_preview_while_module_stopped`).

- [ ] **Step 9: Commit**

```bash
git add crates/rustytracker-ui/src/audio.rs
git commit -m "Mix a preview voice into the desktop audio thread"
```

---

### Task 4: UI keyboard jam (note keys audition the selected instrument)

**Files:**
- Modify: `crates/rustytracker-ui/src/app.rs` (struct field + initializer)
- Modify: `crates/rustytracker-ui/src/input.rs` (NOTE_KEYS const; `note_value_for_key`; preview trigger/release block; reuse NOTE_KEYS in the edit block)

**Interfaces:**
- Consumes: `AudioPlaybackEngine::preview_note_on/off` (Task 3); `self.mixer_mode`, `self.selected_instrument`, `self.octave`.
- Produces: `RustyTrackerApp.preview_key: Option<egui::Key>`; `RustyTrackerApp::note_value_for_key(&self, egui::Key) -> Option<u8>`.

This task has no unit test — egui `InputState` is not constructible in isolation — so it is gated on a clean build, clippy, and a manual smoke test.

- [ ] **Step 1: Add the `preview_key` field to `RustyTrackerApp` in `app.rs`**

In the struct (after `view_mode: ViewMode,`, around line 119):

```rust
    pub(crate) view_mode: ViewMode,
    pub(crate) preview_key: Option<egui::Key>,
}
```

In `RustyTrackerApp::new` (after `view_mode: ViewMode::PatternEditor,`, around line 145):

```rust
            view_mode: ViewMode::PatternEditor,
            preview_key: None,
        }
```

- [ ] **Step 2: Add the NOTE_KEYS const and `note_value_for_key` helper in `input.rs`**

At module scope in `input.rs` (e.g. just above `fn key_to_note_and_octave_offset`):

```rust
const NOTE_KEYS: [Key; 29] = [
    Key::Z,
    Key::S,
    Key::X,
    Key::D,
    Key::C,
    Key::V,
    Key::G,
    Key::B,
    Key::H,
    Key::N,
    Key::J,
    Key::M,
    Key::Q,
    Key::Num2,
    Key::W,
    Key::Num3,
    Key::E,
    Key::R,
    Key::Num5,
    Key::T,
    Key::Num6,
    Key::Y,
    Key::Num7,
    Key::U,
    Key::I,
    Key::Num9,
    Key::O,
    Key::Num0,
    Key::P,
];
```

Note: this literal must match the inline key array currently in the edit block (`input.rs:102-131`) exactly — same keys, same order (29 keys). If the source literal differs, copy it verbatim and set `[Key; N]` to its real element count. Do not add or drop keys.

Add the helper inside `impl RustyTrackerApp` in `input.rs` (e.g. after `get_active_pattern_rows`):

```rust
    pub(crate) fn note_value_for_key(&self, key: Key) -> Option<u8> {
        let (name, octave_offset) = key_to_note_and_octave_offset(key)?;
        let final_octave = (self.octave as i8 + octave_offset).clamp(0, 8) as u8;
        match Note::key(final_octave, name) {
            Ok(Note::Key(value)) => Some(value),
            _ => None,
        }
    }
```

- [ ] **Step 3: Replace the inline key array in the edit block with `NOTE_KEYS`**

In `handle_keyboard_input`, the edit-mode note-input loop currently reads
`for key in [ Key::Z, …, Key::P ] {` (input.rs:101-132). Replace the inline
array with the const:

```rust
                if self.active_field == ActiveField::Note {
                    for key in NOTE_KEYS {
                        if input.key_pressed(key) {
```

(Leave the body of that loop unchanged.)

- [ ] **Step 4: Add the always-on preview trigger/release block**

Inside `ctx.input(|input| { … })`, after the navigation keys and before the
`if self.edit_mode {` block (around input.rs:42), add:

```rust
            // Live sample preview (jam) — runs in both edit and non-edit mode.
            for key in NOTE_KEYS {
                if input.key_pressed(key) {
                    if let Some(value) = self.note_value_for_key(key) {
                        self.audio_engine
                            .preview_note_on(self.selected_instrument, value, self.mixer_mode);
                        self.preview_key = Some(key);
                    }
                }
            }
            if let Some(active) = self.preview_key {
                if input.key_released(active) {
                    self.audio_engine.preview_note_off();
                    self.preview_key = None;
                }
            }
```

- [ ] **Step 5: Build and lint**

Run: `cargo build -p rustytracker-ui`
Expected: builds with no errors.

Run: `cargo clippy -p rustytracker-ui --all-targets`
Expected: no new warnings introduced by these changes.

- [ ] **Step 6: Manual smoke test**

Run: `cargo run -p rustytracker-ui` (load a `.mod`/`.xm` with samples).
Verify: with the song stopped and EDIT OFF, pressing Z/S/X/… sounds the selected instrument and releasing the key stops it; pressing a new key retriggers; in EDIT ON, the same keys both audition AND write the note to the pattern. Confirm jamming also works while the song is playing.

- [ ] **Step 7: Commit**

```bash
git add crates/rustytracker-ui/src/app.rs crates/rustytracker-ui/src/input.rs
git commit -m "Audition instruments from the keyboard in the desktop UI"
```

---

### Task 5: UI click-to-preview + stop/view cutoff

**Files:**
- Modify: `crates/rustytracker-ui/src/panels.rs` (core import; instrument-list click; STOP button; PATTERN/INSTR view buttons)

**Interfaces:**
- Consumes: `AudioPlaybackEngine::preview_note_on/off` (Task 3); C-4 = `Note::key(4, NoteName::C)`.

No unit test (UI interaction); gated on build, clippy, and manual smoke.

- [ ] **Step 1: Import `Note` and `NoteName` in `panels.rs`**

Update the core import (panels.rs line ~5) to:

```rust
use rustytracker_core::{InstrumentName, Note, NoteName, SampleLoopKind, SampleName};
```

- [ ] **Step 2: Fire a C-4 preview when an instrument row is clicked**

In `render_instrument_list`, replace the click handler (panels.rs:367-369):

```rust
                if response.clicked() {
                    self.selected_instrument = ins_num;
                    if let Ok(Note::Key(value)) = Note::key(4, NoteName::C) {
                        self.audio_engine
                            .preview_note_on(ins_num, value, self.mixer_mode);
                    }
                }
```

- [ ] **Step 3: Stop the preview when transport STOP is pressed**

In `render_controls_bar`, the STOP button handler (panels.rs:124-136) gains a
preview cutoff:

```rust
            {
                self.audio_engine.stop();
                self.audio_engine.preview_note_off();
                self.active_row = 0;
                self.active_order_index = 0;
            }
```

- [ ] **Step 4: Stop the preview when switching editor views**

In the PATTERN button handler (panels.rs:228-231):

```rust
            {
                self.audio_engine.preview_note_off();
                self.view_mode = ViewMode::PatternEditor;
            }
```

In the INSTR button handler (panels.rs:239-242):

```rust
            {
                self.audio_engine.preview_note_off();
                self.view_mode = ViewMode::InstrumentEditor;
            }
```

- [ ] **Step 5: Build and lint**

Run: `cargo build -p rustytracker-ui`
Expected: builds clean.

Run: `cargo clippy -p rustytracker-ui --all-targets`
Expected: no new warnings.

- [ ] **Step 6: Manual smoke test**

Run: `cargo run -p rustytracker-ui`.
Verify: clicking an instrument in the right-hand INSTRUMENTS list plays a C-4 of that instrument; pressing STOP silences a ringing preview; switching PATTERN/INSTR views also cuts it.

- [ ] **Step 7: Commit**

```bash
git add crates/rustytracker-ui/src/panels.rs
git commit -m "Preview instruments on click; cut preview on stop/view change"
```

---

### Task 6: Workspace verification

**Files:** none (verification + final gate).

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`
Expected: no diff. If it reports formatting, run `cargo fmt --all` and include the changes in the final commit.

- [ ] **Step 2: Clippy across the workspace**

Run: `cargo clippy --workspace --all-targets`
Expected: no new warnings from the changed crates.

- [ ] **Step 3: Full test suite**

Run: `cargo test --workspace`
Expected: PASS. Fixture-gated tests (those needing `MILKYTRACKER_ROOT`) skip when the fixtures are absent; that is expected and not a failure.

- [ ] **Step 4: Final commit (only if fmt produced changes)**

```bash
git add -A
git commit -m "Apply rustfmt after sample-preview + rebrand work"
```

---

## Self-Review

**Spec coverage:**
- Rebrand (enum/ALL/label/cli_name/from_name/uses_linear_interpolation + CLI usage string; fixtures untouched) → Task 1. ✓
- `PreviewVoice` public API reusing the engine + mixer-mode fidelity → Task 2. ✓
- Audio-thread `PreviewNoteOn/Off`, `preview` field, always-mix `write_audio`, engine methods → Task 3. ✓
- Keyboard jam in both modes + `preview_key` + release handling → Task 4. ✓
- Click-to-preview at C-4 + stop/view cutoff → Task 5. ✓
- Decisions (mono sustain-while-held, keyboard+click, C-4 click, no `milkytracker` alias) → encoded in Global Constraints and Tasks 1/4/5. ✓
- Testing (PreviewVoice unit tests incl. mixer-mode/missing-instrument/non-looping-stop; rebrand round-trip; UI mix-while-stopped; existing normalize test) → Tasks 1/2/3/6. ✓
- Known caveat (click-preview of a looped sample rings until next trigger/stop) → inherent in the mono design; cut by STOP/view-change/next note per Task 5. ✓

**Placeholder scan:** No TBD/TODO. The one explicit verification note (NOTE_KEYS array length must match the existing literal) is an instruction to copy verbatim, with the source line range given — not a deferred decision.

**Type consistency:** `PreviewVoice::{new,note_on,note_off,is_active,render_stereo_frame}` signatures are identical across Task 2's definition and their uses in Tasks 3/4/5. `AudioCommand::PreviewNoteOn { instrument, note, mixer_mode }` and `PreviewNoteOff` match between the enum (Task 3 Step 4), the engine methods (Step 6), and the handler (Step 7). `preview_key: Option<egui::Key>` is defined in Task 4 Step 1 and used in Step 4. C-4 = note value `49` is consistent across the plan.
