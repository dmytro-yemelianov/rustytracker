# RustyTracker Architecture Spec

## Goal

Rewrite MilkyTracker's core in Rust while preserving the tracker semantics that
matter for XM/MOD composition, playback, and editing.

MilkyTracker C++ is the reference implementation. RustyTracker is not a
line-by-line port. It is a typed reimplementation with compatibility tests.

## Non-Goals For The First Phase

- No GUI rewrite.
- No obscure legacy import parity before XM/MOD is stable.
- No unsafe Rust unless a measured playback bottleneck forces it.
- No file-format behavior without fixtures and golden tests.

## Crate Boundaries

| Crate | Responsibility | MilkyTracker Reference |
| --- | --- | --- |
| `rustytracker-core` | Module, pattern, instrument, sample, envelope model | `XModule.h`, `ModuleEditor.h` |
| `rustytracker-xm` | XM parse/write and packed pattern conversion | `LoaderXM.cpp`, `ExporterXM.cpp` |
| `rustytracker-mod` | MOD parse/write and compatibility checks | `LoaderMOD.cpp`, `ExporterXM.cpp` |
| `rustytracker-play` | Effects, mixer, resampling, offline PCM render | `PlayerSTD.cpp`, `PlayerIT.cpp`, `ChannelMixer.cpp` |
| `rustytracker-edit` | Editing commands, undo, transforms | `PatternEditor.cpp`, `PatternEditorTools.cpp`, `SampleEditor.cpp` |
| `rustytracker-cli` | Golden-test runner, inspection, render tools | `milkycli.cpp`, `WAVExporter.cpp` |
| `rustytracker-ui` | Desktop UI after headless parity | `Tracker.cpp`, `ppui/*` |

## Core Design Rules

1. Use typed indices and bounded constructors instead of raw integers.
2. Store normalized pattern cells in the core; convert to packed file bytes only
   in format crates.
3. Make editor-internal capacity explicit. MilkyTracker logically starts with 8
   song channels but expands allocated editor patterns to 32 channels.
4. Keep playback state separate from module data.
5. Keep editing commands separate from raw data mutation so undo and tests can
   reason about each action.

## Migration Order

1. `rustytracker-core`
2. XM read-only parser
3. XM writer and structural roundtrip tests
4. Offline PCM render for a tiny effect subset
5. Expand playback effects through golden tests
6. Editing commands and undo
7. MOD support
8. UI

## Current Core Contracts

The first core contracts cover:

- empty-song defaults
- typed note encoding
- pattern shape and bounds checks
- fixed-width visible names
- order-list mutation behavior
- empty instrument/sample pool defaults
