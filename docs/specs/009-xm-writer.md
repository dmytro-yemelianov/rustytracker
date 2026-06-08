# XM Writer Spec

## Scope

The XM writer starts with the fixed module header, active order table, pattern
headers/cells, the first MilkyTracker-compatible effect inverse mappings, and
instrument/sample headers with 8-bit and 16-bit delta-coded sample payloads.
Full normalized roundtrip support still needs the remaining symmetric effect
coverage and end-to-end equality tests.

## References

Writer behavior follows MilkyTracker's bundled XM references and save path:

- `resources/reference/xm-form.txt` for pattern header and unpacked cell layout
- `src/milkyplay/ExporterXM.cpp` `convertEffect`, `convertToVolume`,
  `convertEffects`, and sample-data writing for core-to-XM effect conversion,
  column placement, and delta sample payload encoding
- `src/milkyplay/LoaderXM.cpp` for the inverse parser behavior that writer
  tests must roundtrip through

## Header Block

`write_xm_header` writes the 336-byte XM header prefix used by version `0x0104`:

- `Extended Module: ` signature
- marker byte
- module title
- RustyTracker tracker name
- XM version
- header size
- active song length
- restart position
- channel count
- pattern count
- instrument count
- frequency-table flag
- default tick speed and BPM
- 256-byte order table

The writer rejects modules whose active order list cannot fit in the XM order
table. Pattern and instrument counts are also checked against the `u16` fields
used by the XM header.

## Tests

The writer tests verify:

- an empty core module emits a header that `parse_xm_header` can read
- all bundled XM fixtures can be parsed into core and have their header/order
  metadata emitted back into a readable XM header
- overlong order tables fail before bytes are emitted
- empty patterns are written with zero payload bytes
- simple note/instrument cells roundtrip through unpacked XM pattern data
- empty instruments are emitted as short XM instrument headers
- active instruments are emitted with extension metadata and sample headers
- 8-bit and 16-bit sample payloads are emitted with XM delta encoding
- 16-bit sample lengths and loop fields are written as byte counts
- sample header fields that cannot fit XM `u32` fields fail before bytes are
  returned

## Pattern Blocks

`write_xm_patterns` writes one pattern header per core pattern:

- 9-byte XM pattern header
- packing type `0`
- core row count
- packed data length

Empty patterns are encoded with a zero-length payload. Non-empty patterns are
currently emitted as unpacked five-byte XM cells:

```text
note, instrument, volume_column, effect, operand
```

The current writer preserves notes and instrument numbers and writes one effect
column plus the XM volume column using the same placement rule as
MilkyTracker's XM exporter:

- a single core effect slot is written to the XM effect column
- when multiple core effect slots exist, slot 0 tries the XM volume column first
- later slots use the effect column first, then the volume column if the effect
  column is already occupied
- tone portamento operands with a low nibble stay in the effect column because
  the XM volume column can only preserve the high nibble
- effects are only relocated into the XM volume column when the current parser
  can recover the same normalized core effect; zero-operand volume/panning
  slides and low-nibble tone portamento are left in the effect column when space
  exists

Implemented inverse mappings:

- internal non-zero arpeggio `0x20` -> XM effect `0`
- internal extended commands `0x30..=0x3f` -> XM `E` command operands
- internal extra-fine portamento `0x41..=0x42` -> XM `0x21`
- `Cxx` and global volume operands from core `0..=255` back to XM `0..=64`
- volume, volume slide, vibrato, panning, panning slide, and tone portamento
  relocation into the XM volume column where the current parser can roundtrip
  the normalized core effect
- internal fine volume slide up/down `EAx` / `EBx` relocate to volume-column
  `9x` / `8x` when the operand is non-zero; zero-operand commands stay in the
  effect column because the XM volume column cannot distinguish absent data from
  zero-value fine slides

## Instrument Blocks

`write_xm_instruments` writes one XM instrument block per core instrument:

- instruments with no active core samples use the short 29-byte XM header
- instruments with active sample slots use the 263-byte XM instrument header
- sample header size is `40`
- the note sample map is translated from core sample indexes back to XM-local
  sample slots
- envelope point values are scaled from core values back to XM values
- vibrato depth and volume fadeout are scaled back to their XM stored values
- sample header lengths are derived from core sample data
- empty samples keep zero-byte length and zero loop-byte fields
- 16-bit sample length, loop start, and loop length fields are stored as byte
  counts

## Sample Payloads

Sample payloads are written immediately after all sample headers for the active
instrument, matching the XM layout parsed by `parse_xm_instruments`.

The writer supports mono core sample data:

- `SampleData::Pcm8` writes one delta byte per frame
- `SampleData::Pcm16` writes one little-endian delta word per frame and sets
  the XM 16-bit sample-type flag
- payload deltas start from zero and use wrapping subtraction, matching
  MilkyTracker's XM save path and the parser's wrapping accumulator
- loop-type bits come from `SampleLoopKind`; parsed stereo samples are already
  normalized to mono core data and are not re-emitted with the stereo flag

The writer returns `SampleFieldTooLarge` when a sample length or loop byte field
does not fit XM's `u32` storage.
