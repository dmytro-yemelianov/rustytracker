# RustySynth Distinct Character: Cubic Resampling + Warmth — Design

Date: 2026-06-30
Status: Approved (design); ready for implementation plan

## Summary

Give the `RustySynth` mixer mode its own sonic identity instead of rendering
byte-identical to `HiFi`. RustySynth becomes:

1. **Higher-fidelity resampling** — 4-point Catmull-Rom cubic interpolation
   (HiFi stays 2-point linear; Amiga/ProTracker stay stepped/nearest).
2. **Warmth** — a master-bus stage applied to the summed stereo frame:
   tanh soft-clip (harmonic warmth + soft knee replacing hard clip) followed
   by a gentle one-pole low-pass (spectral warmth).

Warmth and cubic are **RustySynth-only**. HiFi, Amiga, and ProTracker are
unchanged; **HiFi output stays byte-identical** to today.

This also folds in two small deferred cleanups from the previous feature's
review (see Cleanups).

## Context (current state)

- `crates/rustytracker-play/src/lib.rs` holds the mixer DSP:
  - `Mixer::render_stereo_frame` / `render_mono_frame` sum active voices and
    return raw PCM `(i32, i32)` / `i32` (f64 accumulators cast at the end).
  - `period_to_frequency(period, table, mixer_mode)` — pitch (mode-aware:
    Amiga/ProTracker use the PAL clock).
  - `get_sample_value(data, frame, fraction, sample, mixer_mode)` — currently
    two paths: `uses_linear_interpolation()` (HiFi | RustySynth) → linear
    interp via `get_sample_value_linear` + `next_frame_index`; else → stepped
    `sample_value_as_f64`.
  - `PlaybackMixerMode` flags: `uses_pal_clock()` (Amiga | ProTracker),
    `uses_linear_interpolation()` (HiFi | RustySynth).
- `MixerVoice::advance_sample_position` already tracks `sample_frame` +
  `sample_frame_fraction` (a `u32` 0..=u32::MAX fraction), so a fractional
  read position is available for cubic.
- Verified today: `--mixer rustysynth` WAV is byte-identical to `--mixer hifi`
  (both linear, no warmth). This design **intentionally** breaks that equality.

## Decisions (confirmed)

- Resampling for RustySynth: **Catmull-Rom cubic** (4-point).
- Warmth: **tanh soft-clip → one-pole low-pass**, applied to the summed master
  frame, RustySynth-only.
- Ship **subtle defaults**, tuned by ear after implementation. The two knobs
  are the soft-clip `drive` and the low-pass `cutoff`.
- HiFi/Amiga/ProTracker behavior must not change; HiFi byte-identical.

## Component 1 — Cubic resampling

`crates/rustytracker-play/src/lib.rs`, alongside the existing interpolation
helpers.

- Introduce `enum Interpolation { Stepped, Linear, Cubic }`.
- Replace `PlaybackMixerMode::uses_linear_interpolation() -> bool` with
  `interpolation(self) -> Interpolation`:
  - `HiFi => Linear`
  - `RustySynth => Cubic`
  - `Amiga | ProTracker => Stepped`
  (`uses_linear_interpolation` is private and only used by `get_sample_value`,
  so removing it has no external impact.)
- `get_sample_value` switches on `mixer_mode.interpolation()`:
  - `Stepped` → `sample_value_as_f64(data, frame)` (unchanged).
  - `Linear` → existing `get_sample_value_linear` (unchanged → HiFi identical).
  - `Cubic` → new `get_sample_value_cubic`.
- `get_sample_value_cubic(data, frame, fraction, sample) -> f64`:
  - `t = fraction / u32::MAX` (same convention as linear).
  - Fetch four points: `p0 = prev`, `p1 = frame`, `p2 = next`, `p3 = next2`,
    via a loop-aware neighbor helper.
  - Catmull-Rom:
    ```
    0.5 * ( 2*p1
          + (-p0 + p2) * t
          + (2*p0 - 5*p1 + 4*p2 - p3) * t^2
          + (-p0 + 3*p1 - 3*p2 + p3) * t^3 )
    ```
  - At `t = 0` this returns exactly `p1` (the current sample).
- Loop-aware neighbor fetch — extend the logic in `next_frame_index`:
  - For a looping sample (forward or ping-pong), wrap neighbor indices within
    the loop as `next_frame_index` already does for `+1`; compute `-1`, `+1`,
    `+2` the same way.
  - For a non-looping sample: an index `>= frame_count` returns `0.0` (matches
    today's linear end-handling, which uses `0.0` past the end); an index `< 0`
    clamps to frame `0`. Helper returns the resolved sample value (`f64`) for
    each of the four taps.
  - The existing `sample_value_as_f64` is reused for the actual PCM→f64 read
    (keeps PCM8/PCM16 handling in one place).

## Component 2 — Master warmth

New module `crates/rustytracker-play/src/warmth.rs`.

```rust
pub struct MasterWarmth {
    lp_l: f64,            // low-pass state (left)
    lp_r: f64,            // low-pass state (right)
    lp_coeff: f64,        // cached one-pole coefficient `a`
    coeff_sample_rate: u32, // sample rate `lp_coeff` was computed for (0 = none yet)
}
```

- `MasterWarmth::new() -> Self` — zeros the state, `lp_coeff = 0.0`,
  `coeff_sample_rate = 0`. Takes no sample rate (the `Mixer` doesn't have one
  at construction); the coefficient is derived lazily on first `process`.
- `drive` and the low-pass `cutoff` are module-level named constants in
  `warmth.rs` (default `DRIVE = 1.0`, `CUTOFF_HZ ≈ 12_000.0`), documented as
  ear-tuned knobs.
- Normalization: warmth runs in the normalized domain `x_n = raw / 32768.0`,
  denormalized by `* 32768.0` on output.
- **Soft-clip** (stateless, per sample): `tanh(DRIVE * x_n)`. Unit slope at the
  origin (near-transparent on quiet signal), monotonic, compresses peaks toward
  ±1 — a soft knee in place of a hard clip. Higher `DRIVE` = more saturation.
- **One-pole low-pass** (stateful, per channel): `y += lp_coeff * (x - y)`,
  with `lp_coeff = 1 - exp(-2π * CUTOFF_HZ / sample_rate)`. `process` recomputes
  and caches `lp_coeff` only when the incoming `sample_rate != coeff_sample_rate`.
- Chain order: **soft-clip → low-pass** (drive into the filter).
- `process(&mut self, l: f64, r: f64, sample_rate: u32) -> (f64, f64)` — stereo.
- `process_mono(&mut self, x: f64, sample_rate: u32) -> f64` — mono path (uses
  `lp_l`), so `render_mono_frame` stays consistent with stereo.

### Wiring into the mixer

- Add `PlaybackMixerMode::uses_warmth(self) -> bool` → `true` only for
  `RustySynth`.
- `Mixer` gains a `warmth: MasterWarmth` field, constructed via
  `MasterWarmth::new()` in `Mixer::new` (no sample-rate argument needed). State
  resets when a new `Mixer` is created (i.e. per `PlaybackState::start` / per
  `PreviewVoice`).
- In `render_stereo_frame`, after summing to `(mixed_l, mixed_r)` and before
  the `as i32` cast:
  ```rust
  let (out_l, out_r) = if mixer_mode.uses_warmth() {
      self.warmth.process(mixed_l, mixed_r, sample_rate)
  } else {
      (mixed_l, mixed_r)
  };
  Ok((out_l as i32, out_r as i32))
  ```
  Same pattern in `render_mono_frame` via `process_mono(mixed, sample_rate)`.
- **HiFi/Amiga/ProTracker** take the `else` branch → identical to today.

**Sample-rate note:** `Mixer::new(channel_count)` takes no sample rate, but both
render functions receive `sample_rate` each call and pass it to `process`.
`MasterWarmth` derives and caches `lp_coeff` only when the incoming
`sample_rate` differs from `coeff_sample_rate` (cheap guard; recompute only on
change), so `Mixer::new`'s signature is unchanged.

## Data flow

```
per voice: get_sample_value(.. Cubic ..) → vol/pan → accumulate (mixed_l, mixed_r)
         → if RustySynth: MasterWarmth::process (tanh soft-clip → one-pole LPF)
         → (i32, i32) → existing downstream clamp (rarely engaged for RustySynth)
```

## Error handling / edge cases

- Cubic neighbor reads past a non-looping sample end return `0.0` (consistent
  with current linear behavior); reads before frame 0 clamp to frame 0. No
  panics — all reads go through `.get()`-based `sample_value_as_f64`.
- Warmth never returns NaN/Inf for finite input (`tanh` is bounded; LPF is a
  convex combination). The downstream `as i32` + existing WAV/audio clamp
  remain as the final safety net.
- An empty/inactive mix yields `(0.0, 0.0)`; warmth of `0` is `0` (tanh(0)=0,
  LPF settles to 0), so silence stays silence.

## Testing

`rustytracker-play`:
- **Cubic correctness:** `get_sample_value_cubic` at `t=0` returns the exact
  `p1`; on a curved (non-linear) sample, cubic differs from linear and from
  stepped at a fractional position.
- **Mode metadata:** `RustySynth.interpolation() == Cubic`,
  `HiFi.interpolation() == Linear`, `Amiga.interpolation() == Stepped`;
  `RustySynth.uses_warmth() == true`, all others `false`.
- **Soft-clip:** `tanh`-based curve is ~unity for small input
  (`f(x) ≈ x` within tolerance for small `x`), strictly compresses larger input
  (`|f(x)| < |x|` for larger `x`), monotonic, `f(0) == 0`.
- **Low-pass:** attenuates a fast-alternating (near-Nyquist) input sequence
  relative to its input amplitude; passes DC unchanged at steady state.
- **HiFi unchanged:** a master frame large enough that RustySynth soft-clips it
  passes through HiFi's render path unaltered (HiFi takes the no-warmth branch
  and the Linear interpolation path).
- **RustySynth differs from HiFi:** end-to-end, RustySynth and HiFi renders of
  the same curved sample now differ (the equality verified earlier is broken on
  purpose).
- **Preview inherits character:** because `PreviewVoice` uses the same `Mixer`,
  a RustySynth preview render reflects cubic + warmth (extend existing preview
  mixer-mode coverage).

`rustytracker-cli` (manual/again at finish): `--mixer hifi` vs `--mixer
rustysynth` WAVs now **differ**; `--mixer hifi` is unchanged vs. its prior
output.

## Cleanups (folded in — deferred Minors from the preview/rebrand review)

- `crates/rustytracker-ui/src/input.rs`: the edit-mode note loop reuses
  `note_value_for_key` instead of recomputing the octave/note inline (only the
  preview path used the helper). Behavior unchanged.
- `crates/rustytracker-play/src/preview.rs`: move `self.settings = settings` in
  `note_on` to after `apply_cell` succeeds, so a failed `note_on` doesn't leave
  the voice's settings reflecting an aborted note. Behavior unchanged.

## Files touched

- `crates/rustytracker-play/src/lib.rs` — `Interpolation` enum;
  `interpolation()` / `uses_warmth()`; cubic in `get_sample_value`; loop-aware
  neighbor helper; `Mixer.warmth` field + wiring in both render fns.
- `crates/rustytracker-play/src/warmth.rs` — **new** `MasterWarmth`.
- `crates/rustytracker-play/tests/` — cubic + warmth + metadata tests
  (extend `mixer_mode.rs` / `preview.rs` or a new `warmth.rs` test file).
- `crates/rustytracker-ui/src/input.rs`, `crates/rustytracker-play/src/preview.rs`
  — folded-in cleanups.

## Out of scope (YAGNI)

- Per-voice filtering or per-voice saturation (warmth is master-bus only).
- User-facing UI controls for drive/cutoff (named constants; tuned by ear).
- Changing HiFi/Amiga/ProTracker character.
- Oversampling / anti-aliased resampling beyond cubic.
- Stereo width / other master effects.
