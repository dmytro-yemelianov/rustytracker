# Core Model Spec

## Reference Defaults

MilkyTracker empty-song defaults:

- song channels: 8
- editor pattern channel capacity: 32
- pattern rows: 64
- effect slots per cell: 2
- frequency table: linear
- main volume: 255
- BPM/speed field: 125
- tick speed/tempo field: 6
- restart position: 0
- order list: one order pointing at pattern 0

RustyTracker normalizes this into an eagerly allocated empty pattern. The C++
model can have null pattern data until `ModuleEditor::getPattern()` allocates it;
RustyTracker avoids nullable pattern storage.

## Note Encoding

The core preserves FastTracker/MilkyTracker numeric note semantics:

- `0`: empty note
- `1..=96`: C-0 through B-7
- `121`: note off

Higher-level APIs should expose `Note`, not raw note bytes.

## Pattern Cell

A pattern cell contains:

- note
- instrument number, `0` for empty
- fixed count of effect commands

XM packing is not part of the core model.

## Invariants

- Module channel count must be `1..=32`.
- Pattern row and channel access must be bounds checked.
- Pattern cells must have the pattern's configured effect slot count.
- Empty modules must be constructible without allocation failure paths in normal
  Rust code.
- Order lists must have `1..=255` active positions.
- Empty module construction allocates the default MilkyTracker instrument/sample
  pool shape: 128 instruments and 2048 samples.
- Instrument note/sample maps use `Option<usize>` so imported files can
  represent MilkyTracker's invalid `-1` sample mapping.
- Samples keep typed payloads as `SampleData::Empty`, `SampleData::Pcm8`, or
  `SampleData::Pcm16`; parser crates should not own long-lived sample buffers.
- Instruments preserve envelope metadata, vibrato metadata, and volume fadeout
  in core types so playback and editing do not need to inspect XM parser
  structs.
- Samples preserve a normalized `SampleLoopKind` independently from the raw
  format-specific sample type byte.
