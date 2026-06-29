# Sample Preview & RustySynth Rebrand — Design

Date: 2026-06-29
Status: Approved (design); ready for implementation plan

## Summary

Two work items:

1. **Rebrand** the `MilkyTracker` mixer mode (added earlier this session) to
   `RustySynth`. This is a clean rename of a project-owned, unreleased mixer
   profile — it does **not** touch the test fixtures that reference the real
   MilkyTracker program as a compatibility oracle.
2. **Sample preview in the desktop UI.** Make instruments/samples auditionable:
   note keys jam the selected instrument live, and clicking an instrument in the
   list plays a test note. Voices are **monophonic, sustain-while-held**.

## Context (current state)

- `rustytracker-play` has a complete, reusable playback engine:
  - `MixerVoice` — a self-contained voice (sample index/frame/fraction, period,
    volume, panning, envelope values, loop state) with `advance_sample_position`.
  - `Mixer::render_stereo_frame(module, sample_rate, channels, mixer_mode)` —
    mixes active voices and is **mixer-mode aware**.
  - Private DSP helpers `period_to_frequency()` and `get_sample_value()` apply the
    selected mixer mode (PAL Amiga pitch, stepped vs linear-interpolated fetch).
  - `PlaybackChannelState::apply_cell()` / `trigger_key()` resolve a note +
    instrument into a triggered voice (sample index, period, volume, panning).
- The desktop UI (`rustytracker-ui`) only plays the **whole module** via
  `audio.rs` → `PlaybackState::render_raw_stereo_frame`. Note keys (Z/S/X…) in
  edit mode only **write** cells to the pattern; nothing auditions a sample.
- Pattern effects ("pattern FX") are already implemented and editable
  (arpeggio, porta up/down/tone, vibrato, vibrato+volslide, panning, sample
  offset, volume, volume slide, fine volume slide, set speed/BPM, position jump,
  pattern break). No change required here — listed only to confirm availability.

## Item 1 — Rebrand: MilkyTracker mixer mode → RustySynth

Scope is intentionally narrow. The enum variant is referenced **only** inside
`rustytracker-play/src/lib.rs`, plus one CLI usage string. `panels.rs` (MIX
selector) and `rustytracker-wasm` consume the mode through `PlaybackMixerMode::ALL`,
`.label()`, and `.cli_name()`, so they pick up the rename automatically.

Changes:

- `crates/rustytracker-play/src/lib.rs`
  - Enum: `PlaybackMixerMode::MilkyTracker` → `RustySynth`.
  - `ALL`: update the variant entry.
  - `label()`: `"MilkyTracker"` → `"RustySynth"`.
  - `cli_name()`: `"milkytracker"` → `"rustysynth"`.
  - `from_name()`: accept `"rustysynth" | "rusty" | "rs"` → `RustySynth`.
    **No** `milkytracker` alias is kept for this mode (clean break; the mode is
    unreleased). The token `milkytracker` should not silently map to a
    project-owned synth.
  - `uses_linear_interpolation()`: `Self::HiFi | Self::MilkyTracker` →
    `Self::HiFi | Self::RustySynth`.
- `crates/rustytracker-cli/src/lib.rs:35` — usage string:
  `<hifi|milkytracker|amiga|protracker>` → `<hifi|rustysynth|amiga|protracker>`.

Explicitly **out of scope** (must NOT be renamed — they refer to the real
MilkyTracker program / its bundled test data):
- `rustytracker-test-support` (`MILKYTRACKER_ROOT`, `milkytracker_fixture_*`).
- Test names like `export_wav_uses_milkytracker_mod_pitch_clock`,
  `render_to_wav_uses_milkytracker_amiga_tick_clock`,
  `parses_milkytracker_bundled_xm_headers`, etc.
- Assertions such as `tracker_name == "MilkyTracker"`.
- `// Correct loops like MilkyTracker does:` comment in `rustytracker-mod`.

## Item 2 — Sample preview

### Approach

Reuse the existing engine by adding a `PreviewVoice` type **inside**
`rustytracker-play`. Because it lives in the crate it can call the existing
`pub(crate)` trigger logic and the private mixer-mode DSP directly, with zero
duplication. Preview therefore sounds identical to playback/export for the
selected mixer mode.

Rejected alternatives:
- A standalone mini-player in the UI's `audio.rs` — duplicates pitch/loop DSP,
  won't track mixer modes, drifts from the engine.
- Synthesizing a 1-channel/1-row `Module` and running a `PlaybackState` —
  heavyweight, allocates a module per note, awkward for retrigger/sustain/stop.

### Component 1 — `PreviewVoice` (new module `rustytracker-play/src/preview.rs`)

Public API:

```rust
pub struct PreviewVoice { /* PlaybackChannelState (ch 0) + 1-voice Mixer + PlaybackSettings */ }

impl PreviewVoice {
    pub fn new() -> Self;
    pub fn note_on(
        &mut self,
        module: &Module,
        instrument: u8,
        note: u8,
        settings: PlaybackSettings,
    ) -> PlaybackResult<()>;
    pub fn note_off(&mut self);
    pub fn is_active(&self) -> bool;
    pub fn render_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawStereoPcmFrame>;
}
```

Behavior:
- Holds a **persistent** `PlaybackChannelState` and a 1-voice `Mixer`, so repeated
  `note_on` calls reuse buffers — no per-keypress heap allocation.
- `note_on`: store `settings`; build a synthetic
  `PatternCell { note: Note::Key(note), instrument, effects: [] }`; run
  `channel.apply_cell(module, &cell)`; then mirror the resolved
  `sample_index / volume / panning / period` into the mixer voice exactly as
  `Sequencer::advance_tick` emits a `SequencerCommand::Trigger`
  (via `Mixer::handle_commands`). Monophonic: each `note_on` retriggers the one
  voice.
- `render_stereo_frame`: if the voice is inactive, return `PLAYBACK_STEREO_SILENCE`;
  otherwise delegate to `Mixer::render_stereo_frame(module, sample_rate,
  &mut channels, settings.mixer_mode)` so mixer-mode pitch and interpolation are
  honored. The voice deactivates itself when a non-looping sample reaches its end
  (existing `MixerVoice` logic).
- `note_off`: issue `SequencerCommand::Stop { channel: 0 }` and mark the channel
  inactive. (MVP: no envelope release — the note stops immediately. Envelope-aware
  release is a possible later enhancement.)
- Export `PreviewVoice` from `lib.rs`. Add `mod preview;`.

### Component 2 — Audio thread (`rustytracker-ui/src/audio.rs`)

- Extend `AudioCommand`:
  - `PreviewNoteOn { instrument: u8, note: u8, mixer_mode: PlaybackMixerMode }`
  - `PreviewNoteOff`
- `AudioThreadState` gains `preview: PreviewVoice` (constructed in the thread).
- Command handling: `PreviewNoteOn` → `preview.note_on(module, instrument, note,
  PlaybackSettings::with_mixer_mode(mixer_mode))` (errors ignored → stays silent);
  `PreviewNoteOff` → `preview.note_off()`. `note_on` requires the module to be
  present; if `local_state.module` is `None`, the command is a no-op.
- Restructure `write_audio` so it **always** produces output as
  `module_frame + preview_frame`:
  - `module_frame` = the current module stereo frame, or `(0, 0)` when not playing
    / no playback / song ended (the existing early-return silence cases become a
    zero module frame instead of an early `return`).
  - `preview_frame` = `preview.render_stereo_frame(module, sample_rate)` when a
    module is present and the voice is active, else `(0, 0)`.
  - Sum, then `normalize_pcm16_sample` + clamp as today.
  - Module position/status bookkeeping is unchanged and still gated on module play
    state. The stream must keep running (not early-return to silence) whenever a
    preview voice could be active, so preview is audible with the song stopped.
- `AudioPlaybackEngine` gains `preview_note_on(instrument, note, mixer_mode)` and
  `preview_note_off()` that push the new commands through the existing producer.

### Component 3 — UI input & clicks

- `app.rs`: `RustyTrackerApp` gains `preview_key: Option<egui::Key>` — the note key
  currently sustaining a preview voice.
- `input.rs` (`handle_keyboard_input`): the note-key loop moves **out** of the
  `if self.edit_mode { … }` block so it runs in both modes:
  - On `key_pressed(note_key)`: resolve the `Note::Key(value)` as today; call
    `self.audio_engine.preview_note_on(self.selected_instrument, value,
    self.mixer_mode)` and set `self.preview_key = Some(key)`. In edit mode, also
    perform the existing write-note + write-instrument + `commit_edit_to_audio` +
    `advance_row_after_edit`. In non-edit mode, only the preview fires.
  - On `key_released(k)` where `Some(k) == self.preview_key`: call
    `self.audio_engine.preview_note_off()` and clear `preview_key`.
  - Edit-mode `Num1` Note-Off behavior is unchanged.
- `panels.rs` (`render_instrument_list`): when an instrument row is clicked it
  already sets `self.selected_instrument = ins_num`; additionally fire a one-shot
  `preview_note_on(ins_num, <C-4 note value>, self.mixer_mode)`.
- Send `preview_note_off()` when playback is stopped and when leaving the
  pattern/instrument view, so a held/ringing preview is cut cleanly.

C-4 note value is obtained the same way keyboard entry builds notes
(`Note::key(4, NoteName::C)` → `Note::Key(u8)`).

### Data flow

```
key press / instrument click
  → input.rs / panels.rs
  → AudioPlaybackEngine::preview_note_on
  → rtrb ring buffer (AudioCommand::PreviewNoteOn)
  → audio thread: PreviewVoice::note_on (resolve note → sample/period)
  → write_audio: module_frame + preview_frame → normalize/clamp → cpal
key release
  → AudioPlaybackEngine::preview_note_off → PreviewNoteOff → PreviewVoice::note_off (Stop)
```

### Error handling & edge cases

- `note_on` on a missing instrument/sample or an empty sample → voice stays
  inactive (silent); never panics.
- Monophonic: a new `note_on` replaces the single voice; the previously sounding
  note is cut.
- **Known MVP caveat:** a *looped* sample previewed via **click** (which has no
  key-release) rings until the next trigger or an explicit stop. Documented; not
  fixed in this MVP.

### Decisions (confirmed)

- Voice model: **monophonic, sustain-while-held** (key release stops the note).
- Trigger: **keyboard jam + click**.
- Click-preview pitch: **C-4** (not the sample's relative/default note).
- No back-compat `milkytracker` alias for the renamed mode.

## Testing

`rustytracker-play` (primary coverage):
- `PreviewVoice::note_on` on a known fixture/synthetic module yields non-silent
  frames; `note_off` → silence; never-triggered voice → silence.
- Mixer mode is honored: HiFi vs Amiga produce different output for the same note
  (e.g. differing frame values / pitch).
- Missing instrument or empty sample → `is_active()` is false and frames are
  silent.
- A non-looping sample stops (`is_active()` false) after it plays past its end.
- Rebrand round-trip: `PlaybackMixerMode::from_name("rustysynth") == RustySynth`,
  `RustySynth.cli_name() == "rustysynth"`, `RustySynth.label() == "RustySynth"`,
  and `from_name("milkytracker")` no longer yields the project synth mode.

`rustytracker-ui`:
- Keep the existing `normalize_pcm16_sample` test.
- A focused test that `write_audio` mixes a preview frame while the module is
  stopped (preview audible with the song not playing), within what is practical
  without a live cpal device.

`rustytracker-cli`:
- The mixer-mode CLI smoke/round-trip continues to pass with `--mixer rustysynth`.

## Files touched

- `crates/rustytracker-play/src/lib.rs` — rebrand; export `PreviewVoice`; `mod preview;`.
- `crates/rustytracker-play/src/preview.rs` — **new** `PreviewVoice`.
- `crates/rustytracker-cli/src/lib.rs` — usage string.
- `crates/rustytracker-ui/src/audio.rs` — preview commands, `PreviewVoice`, mix.
- `crates/rustytracker-ui/src/app.rs` — `preview_key` field.
- `crates/rustytracker-ui/src/input.rs` — jam on note keys + release handling.
- `crates/rustytracker-ui/src/panels.rs` — click-to-preview in instrument list.
- Tests in the crates above.

## Out of scope (YAGNI)

- Polyphonic preview / chords.
- Volume/panning envelope handling on preview (sustain release).
- Preview through the wasm build.
- Pattern-FX changes (already implemented).
- A dedicated "jam mode" toggle separate from edit mode.
```