# XM Writer Spec

## Scope

The XM writer starts with the fixed module header, active order table, and
pattern headers/cells. Full roundtrip support will add effect inverse mapping,
instrument writing, sample-header writing, and sample-payload writing in later
slices.

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
column from the non-empty core effect slots. Full inverse mapping for
MilkyTracker's normalized volume/effect columns is tracked as the next pattern
writer step.
