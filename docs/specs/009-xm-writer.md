# XM Writer Spec

## Scope

The XM writer starts with the fixed module header, active order table, pattern
headers/cells, the first MilkyTracker-compatible effect inverse mappings, and
instrument/sample headers without payload encoding. Full roundtrip support will
add sample-payload writing in a later slice.

## References

Writer behavior follows MilkyTracker's bundled XM references and save path:

- `resources/reference/xm-form.txt` for pattern header and unpacked cell layout
- `src/milkyplay/ExporterXM.cpp` `convertEffect`, `convertToVolume`, and
  `convertEffects` for core-to-XM effect conversion and column placement
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
- active instruments are emitted with extension metadata and zero-length sample
  headers
- non-empty sample payloads are rejected until delta encoding is implemented

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

Deferred mappings:

- `EAx` / `EBx` relocation into volume-column `9x` / `8x` waits until parser and
  core semantics for fine volume slides are represented symmetrically.

## Instrument Blocks

`write_xm_instruments` writes one XM instrument block per core instrument:

- instruments with no active core samples use the short 29-byte XM header
- instruments with active sample slots use the 263-byte XM instrument header
- sample header size is `40`
- the note sample map is translated from core sample indexes back to XM-local
  sample slots
- envelope point values are scaled from core values back to XM values
- vibrato depth and volume fadeout are scaled back to their XM stored values
- sample headers are emitted with zero byte lengths until the sample payload
  encoder exists

The writer rejects non-empty sample data with
`SampleDataEncodingNotImplemented` so it cannot silently emit invalid or fake
audio payloads.
