# Core File Migration Map

## Phase 1: Domain Model

| MilkyTracker File | RustyTracker Target | Notes |
| --- | --- | --- |
| `src/milkyplay/XModule.h` | `rustytracker-core` | Split header, module, pattern, sample, instrument, envelope types. |
| `src/milkyplay/XModule.cpp` | `rustytracker-core`, `rustytracker-xm`, `rustytracker-mod` | Move cleanup/defaults to constructors; file parsing belongs outside core. |
| `src/tracker/ModuleEditor.h` | `rustytracker-edit` | Editing facade, song operations, sample/instrument helpers. |
| `src/tracker/ModuleEditor.cpp` | `rustytracker-edit` | Port only after core and XM fixtures exist. |

## Phase 2: Pattern Editing

| MilkyTracker File | RustyTracker Target | Notes |
| --- | --- | --- |
| `src/tracker/PatternTools.*` | `rustytracker-core` or `rustytracker-edit` | Typed cell accessor replaces offset arithmetic. |
| `src/tracker/PatternEditor.*` | `rustytracker-edit` | Cursor/selection/edit command behavior. |
| `src/tracker/PatternEditorTools.*` | `rustytracker-edit` | Transform commands: transpose, remap, scale volume, split tracks. |
| `src/tracker/Undo.*` | `rustytracker-edit` | Command history over typed snapshots/diffs. |

## Phase 3: Playback

| MilkyTracker File | RustyTracker Target | Notes |
| --- | --- | --- |
| `src/milkyplay/PlayerSTD.*` | `rustytracker-play` | Main XM/MOD effect engine. |
| `src/milkyplay/PlayerIT.*` | Later | IT compatibility can wait unless import parity is required. |
| `src/milkyplay/ChannelMixer.*` | `rustytracker-play` | Start safe, optimize after golden PCM tests. |
| `src/milkyplay/Resampler*.h` | `rustytracker-play` | Unit-test each resampler separately. |
| `src/tracker/ModuleServices.*` | `rustytracker-play`, `rustytracker-cli` | Offline render and length estimation. |

## Phase 4: File Formats

| MilkyTracker File | RustyTracker Target | Notes |
| --- | --- | --- |
| `src/milkyplay/LoaderXM.cpp` | `rustytracker-xm` | First parser target. |
| `src/milkyplay/ExporterXM.cpp` | `rustytracker-xm`, `rustytracker-mod` | Writer and MOD exporter. |
| `src/milkyplay/LoaderMOD.cpp` | `rustytracker-mod` | Second parser target. |
| other loaders | Later/import plugin crates | Do not block core rewrite. |

## Phase 5: UI

UI migration starts only after the headless engine is credible. Candidate stack:

- `winit` + `pixels`/`wgpu` for a faithful pixel tracker.
- `egui` for speed if exact look is less important.

