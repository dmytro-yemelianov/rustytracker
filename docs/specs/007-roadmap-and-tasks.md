# Roadmap And Tasks

## Current State

RustyTracker has a test-first Rust workspace with:

- `rustytracker-core`: typed module, pattern, instrument, sample, note, and order
  model
- `rustytracker-xm`: read-only XM header parsing, pattern decoding, instrument
  metadata parsing, delta-coded sample payload decoding, and end-to-end bundled
  XM loading into `rustytracker-core::Module`
- CodeRabbit review rules requiring compatibility values and format constants to
  live behind named constants/enums

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

Status: mostly complete in PR #1.

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
- XM 8-bit and 16-bit delta sample decode
- end-to-end `parse_xm_module`
- fixture tests against bundled XM files

Remaining:

- merge PR #1 after review
- handle ModPlug stereo sample data
- handle ADPCM-packed XM samples or return an explicit unsupported error
- handle order entries that reference patterns past the declared pattern count

Acceptance:

- all bundled XM fixtures load into core with stable structural checks
- malformed input returns contextual errors
- no parser-owned buffers are needed after conversion into core

## Milestone 2: Golden Structural CLI

Goal: produce stable JSON dumps from loaded modules.

Tasks:

- add `rustytracker-cli`
- implement `rustytracker dump path/to/module.xm --format json`
- add JSON schema for normalized module dumps
- generate golden JSON for bundled fixtures
- compare fixture dumps in tests

Acceptance:

- `cargo test --workspace` verifies golden structural dumps
- JSON diffs make import regressions easy to inspect

## Milestone 3: XM Roundtrip

Goal: load and write enough XM to prove structural roundtrip.

Tasks:

- implement XM writer for header/order table
- write patterns from core cells into XM packed pattern data
- write instruments and sample headers
- write sample payloads using XM delta encoding
- add `XM -> core -> XM -> core` normalized equality tests

Acceptance:

- bundled fixtures roundtrip to equivalent normalized structures
- writer does not depend on playback code

## Milestone 4: Playback Skeleton

Goal: play rows/ticks and raw samples without full effect parity.

Tasks:

- add `rustytracker-play`
- implement order traversal
- implement row/tick timing from speed/BPM
- implement channel state
- mix decoded sample data without interpolation first
- add short deterministic PCM fixture tests

Acceptance:

- simple fixture modules render deterministic PCM
- playback state is testable without UI

## Milestone 5: XM Effect Parity

Goal: implement XM effects incrementally with PCM/golden state tests.

Order:

1. speed/BPM
2. volume column
3. set volume/panning
4. pattern break and position jump
5. arpeggio
6. portamento
7. vibrato
8. sample offset
9. loop and ping-pong loop behavior
10. envelopes and fadeout

Acceptance:

- each effect family has focused fixtures
- regressions show either state diffs or PCM diffs

## Milestone 6: Editing Core

Goal: port tracker editing behavior after import/playback types stabilize.

Tasks:

- add `rustytracker-edit`
- model edit commands
- implement undo/redo snapshots or diffs
- port order operations
- port pattern edits and transformations
- add tests from MilkyTracker `ModuleEditor` and `PatternTools` behavior

Acceptance:

- editing commands are deterministic, reversible where expected, and independent
  of UI state

## Milestone 7: UI

Goal: desktop tracker UI after headless engine is credible.

Candidate stack:

- `winit` + `pixels`/`wgpu` for a faithful tracker surface
- `egui` only if speed matters more than exact MilkyTracker feel

Acceptance:

- UI consumes `core`, `edit`, `play`, and format crates instead of owning engine
  behavior

## Immediate Backlog

1. Merge or continue PR #1 depending on review status.
2. Add explicit unsupported path for XM ADPCM samples.
3. Add tests for out-of-range order pattern references.
4. Handle ModPlug stereo sample data.
5. Start `rustytracker-cli` structural dump once import semantics are stable.
