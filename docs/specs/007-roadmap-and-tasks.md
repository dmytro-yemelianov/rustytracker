# Roadmap And Tasks

## Current State

RustyTracker has a test-first Rust workspace with:

- `rustytracker-core`: typed module, pattern, instrument, sample, note, and order
  model
- `rustytracker-xm`: read-only XM header parsing, pattern decoding, instrument
  metadata parsing, delta-coded sample payload decoding, and end-to-end bundled
  XM loading into `rustytracker-core::Module`; XM header/order/pattern writing
  with the first MilkyTracker-compatible effect inverse mappings, instrument
  metadata writing, and delta-coded sample payload writing
- `rustytracker-play`: initial playback cursor/order traversal, tick timing,
  current-row channel snapshots, mutable per-channel trigger state, and raw
  decoded sample stepping plus fixed-count raw mono PCM rendering without
  interpolation
- `rustytracker-cli`: normalized JSON structural dumps with golden fixture
  comparisons, plus a runnable `play-state` JSON trace for the playback
  skeleton
- CodeRabbit review rules requiring compatibility values and format constants to
  live behind named constants/enums
- reference source map in `docs/specs/010-reference-specs.md`

Current fixture base:

- `milky.xm`
- `slumberjack.xm`
- `sv_ttt.xm`
- `theday.xm`
- `universalnetwork2_real.xm`

## Rules

- Every compatibility behavior starts as a failing test.
- Magic numbers from MilkyTracker, XM/MOD, playback, byte layouts, effect IDs,
  note values, and conversion factors must be named constants or enums.
- Parser crates may expose intermediate format structs, but long-lived app data
  belongs in `rustytracker-core`.
- UI work waits until headless import and playback are credible.

## Milestone 1: XM Structural Import

Goal: bundled XM files load into a core `Module` without playback.

Status: complete and merged.

Done:

- XM module header parse
- XM pattern-header parse
- XM packed pattern-cell decode
- XM instrument section parse
- XM sample header parse
- core instrument envelope metadata
- core instrument vibrato metadata
- core sample loop kind metadata
- undefined XM loop type `0x03` normalized as ping-pong
- empty packed XM patterns
- ModPlug stereo samples mixed to mono
- explicit unsupported errors for ADPCM-packed XM samples
- empty patterns appended for order references past the declared pattern count
- XM 8-bit and 16-bit delta sample decode
- end-to-end `parse_xm_module`
- fixture tests against bundled XM files

Remaining: none.

Acceptance:

- all bundled XM fixtures load into core with stable structural checks
- malformed input returns contextual errors
- no parser-owned buffers are needed after conversion into core

## Milestone 2: Golden Structural CLI

Goal: produce stable JSON dumps from loaded modules.

Status: complete and merged.

Done:

- add `rustytracker-cli`
- implement `rustytracker dump path/to/module.xm --format json`
- add JSON schema for normalized module dumps
- generate golden JSON for bundled fixtures
- compare fixture dumps in tests

Remaining: none.

Acceptance:

- `cargo test --workspace` verifies golden structural dumps
- JSON diffs make import regressions easy to inspect

## Milestone 3: XM Roundtrip

Goal: load and write enough XM to prove structural roundtrip.

Status: in progress.

Done:

- implement XM writer for header/order table
- write empty pattern headers and simple unpacked pattern cells
- add reference-spec map for XM compatibility sources
- add MilkyTracker-compatible inverse mappings for current core pattern effects
- relocate compatible first-slot effects into the XM volume column
- write XM instrument metadata and zero-length sample headers
- write 8-bit and 16-bit sample payloads using XM delta encoding
- add symmetric parser/writer coverage for fine volume-slide volume-column
  commands
- add `XM -> core -> XM -> core` normalized equality tests for bundled fixtures
- add synthetic full-module roundtrip coverage for supported effect-column and
  volume-column inverse mappings
- add synthetic full-module roundtrip coverage for instrument note-map remapping
  from nonzero core sample indexes into XM-local sample slots

Remaining: none identified before playback skeleton work.

Acceptance:

- bundled fixtures roundtrip to equivalent normalized structures
- writer does not depend on playback code

## Milestone 4: Playback Skeleton

Goal: play rows/ticks and raw samples without full effect parity.

Status: complete.

Done:

- add `rustytracker-play`
- implement validated playback cursor start position
- implement row advance within a pattern
- implement order advance after the final pattern row
- report song end without moving the cursor into an invalid row
- reject orders that reference missing patterns
- implement row/tick timing from speed/BPM
- expose current-row channel cell state for active module channels
- add `rustytracker play-state <module.xm> --rows <count>` for runnable
  playback-state inspection
- implement mutable per-channel note, instrument, sample, volume, panning, and
  note-off trigger state
- add raw decoded PCM8/PCM16 sample stepping without interpolation
- add deterministic raw mono PCM render tests on top of sample stepping
- connect raw mono rendering to tick/row progression and sample-rate timing

Tasks: none.

Acceptance:

- simple synthetic modules render deterministic raw mono PCM
- playback state is testable without UI

## Milestone 5: XM Effect Parity

Goal: implement XM effects incrementally with PCM/golden state tests.

Order:

- [x] 1. speed/BPM (complete)
- [x] 2. volume column (complete)
- [x] 3. set volume/panning (complete)
- [x] 4. pattern break and position jump (complete)
- [x] 5. arpeggio (complete)
- [x] 6. portamento (complete)
- [x] 7. vibrato (complete)
- [x] 8. sample offset (complete)
- [x] 9. loop and ping-pong loop behavior (complete)
- [x] 10. envelopes and fadeout (complete)

Acceptance:

- each effect family has focused fixtures
- regressions show either state diffs or PCM diffs

## Milestone 6: Editing Core

Goal: port tracker editing behavior after import/playback types stabilize.

Status: complete.

Tasks:

- [x] add `rustytracker-edit`
- [x] model edit commands
- [x] implement undo/redo snapshots or diffs
- [x] port order operations
- [x] port pattern edits and transformations
- [x] add tests from MilkyTracker `ModuleEditor` and `PatternTools` behavior

Acceptance:

- editing commands are deterministic, reversible where expected, and independent
  of UI state

## Milestone 7: UI

Goal: desktop tracker UI after headless engine is credible.

Status: in progress.

Tasks:

- [x] add `rustytracker-ui` crate
- [x] implement responsive egui (eframe) desktop layout (menu bar, control bar, side panels, and central grid)
- [x] implement interactive pattern editor grid supporting visual cursor highlights and channel scrolling
- [x] implement keyboard navigation (arrow keys, PageUp/PageDown, Space to record)
- [x] implement keyboard piano note entry and hex digit shifts for instrument/effect values in edit mode
- [x] implement desktop file loader dialogue using `rfd` and signature-based XM/MOD auto-detection
- [x] implement tick-accumulator simulated playback playhead tracing linked to `rustytracker-play` engine
- [x] replace generic egui pattern grid with custom tracker-painted pattern surface
- [x] add semantic tracker theme layer for custom-painted tracker surfaces
- [x] add selectable tracker palette settings
- [x] add bitmap-font-backed tracker text renderer
- [x] replace generic transport, view, and side-panel action controls with compact tracker-styled controls
- [x] replace generic instrument/sample editor containers and waveform with tracker-painted surfaces
- [x] replace generic instrument/sample editor toggles and loop selector with tracker-styled controls
- [ ] replace generic instrument/sample editor controls with compact tracker-styled controls
- [x] split CPAL output engine and callback into a focused audio module
- [ ] split remaining `rustytracker-ui` app/input/editor monolith into focused modules

Acceptance:

- UI consumes `core`, `edit`, `play`, and format crates instead of owning engine
  behavior
- pattern/editor surfaces use fixed tracker metrics, semantic colors, and
  deterministic hit testing

## Milestone 8: iOS Native App & Database Integration

Goal: Create a native iOS SwiftUI client powered by the Rust core engine and a persistent database.

Status: Proposed.

Tasks:

- [ ] Add UniFFI bridge crate to generate Swift bindings for `rustytracker-core` and `rustytracker-play`.
- [ ] Configure database schema using GRDB.swift or SwiftData to catalog imported modules, playlists, and history.
- [ ] Create real-time audio thread implementation using `AVAudioSourceNode` feeding from Rust's playback engine.
- [ ] Implement iOS file document picker integration to copy modules from files/iCloud into sandbox.
- [ ] Wire up background audio task handling with `AVAudioSession` and `MPRemoteCommandCenter`.

Acceptance:

- Modules parse their metadata through Rust FFI on import and populate the iOS SQLite database.
- The iOS audio callback runs lock-free and plays `.xm` and `.mod` files smoothly in the background.

## Immediate Backlog

1. Connect the UI to a real-time CPAL sound card output thread using `rustytracker-play`'s PCM frame generation.
2. Keep adding focused XM/MOD edge cases and writer features when playback or future compatibility work exposes a concrete gap.
3. Implement the iOS platform bridge and SQLite/SwiftData integration as described in [013-ios-database-implementation.md](file:///Users/dmytro/Documents/github/rustytracker/docs/specs/013-ios-database-implementation.md).

