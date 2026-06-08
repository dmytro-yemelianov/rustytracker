# XM Parser Milestone

## Scope

`rustytracker-xm` is the first file-format crate because MilkyTracker's own
resource directory already contains representative XM files. The crate starts
read-only and grows only through fixture-backed tests.

MilkyTracker reference:

- `src/milkyplay/LoaderXM.cpp`
- `src/milkyplay/XModule.h`

## Header Contract

The current parser reads the XM module header and checks:

- signature `Extended Module: `
- marker byte `0x1a`
- XM versions accepted by MilkyTracker: `0x0102`, `0x0103`, `0x0104`
- module title and tracker name as fixed-width text
- song length, restart position, channel count, pattern count, instrument count
- flags and frequency-table mode
- default tick speed and BPM
- active order table entries

The bundled MilkyTracker fixtures all use XM `0x0104`, a 276-byte header, and
linear frequency tables.

## Pattern Header Contract

For XM `0x0103` and `0x0104`, MilkyTracker reads each pattern as:

```text
u32 pattern_header_length
u8  packing_type
u16 row_count
u16 packed_pattern_data_length
```

For XM `0x0102`, MilkyTracker reads:

```text
u32 pattern_header_length
u8  packing_type
u8  row_count_minus_one
u16 packed_pattern_data_length
```

MilkyTracker then allocates an expanded internal pattern with:

- `effnum = 2`
- `channum = module channel count`
- `row_count * channel_count * 6` bytes

RustyTracker exposes this parsed metadata as `XmPatternHeader` and uses it to
decode into `rustytracker-core::Pattern`.

## Packed Cell Contract

XM pattern cells are five packed fields:

```text
note, instrument, volume_column, effect, operand
```

If the first byte has bit `0x80` set, its low five bits select which fields are
present. Otherwise the byte is the note and the following four bytes complete an
unpacked cell.

RustyTracker expands each XM cell to the MilkyTracker six-byte layout:

```text
note, instrument, volume_effect, volume_operand, effect, operand
```

Compatibility rules ported from `LoaderXM.cpp` and `XModule.cpp`:

- invalid XM effects are cleared before any operand rewrite
- effects `Cxx` and `Gxx` use MilkyTracker's 0..64 to 0..255 volume mapping
- effect `0xx` with a non-zero operand becomes internal effect `0x20`
- `Exx` becomes internal `0x30..0x3f`
- `Xxx`/`0x21` becomes internal `0x40..0x4f`
- XM note `97` becomes RustyTracker note-off value `121`
- XM volume-column commands are converted into the first internal effect slot
- volume-column fine volume slides `8x` / `9x` become internal `EBx` / `EAx`
- the XM effect column is stored in the second internal effect slot
- a pattern with `packed_pattern_data_length == 0` decodes as an allocated empty
  core pattern

## Fixture Assertions

The first XM fixture tests assert:

- every bundled file parses its header
- every declared pattern has a readable pattern header
- fixture row counts and packed-byte totals match the original files
- packed pattern cells decode for every bundled fixture
- decoded cell counts, non-empty counts, first non-empty cells, and expanded
  pattern checksums match the MilkyTracker-compatible layout
- truncated pattern tables fail before producing partial metadata
- packed cells that end mid-field fail with row/channel context

This gives the later instrument/sample parser an exact byte offset to continue
from without depending on playback code.

## Instrument And Sample Header Contract

For XM `0x0104`, RustyTracker now parses the instrument section after pattern
data. The parser mirrors MilkyTracker's loader behavior:

- every declared instrument starts with `u32 instrument_size`
- short instruments in the `4..29` byte range are read through MilkyTracker's
  padded 29-byte compatibility buffer
- instruments with `instrument_size <= 29` have no extension/sample-header data
- instruments with `instrument_size > 29` consume the declared extension bytes
  even when `sample_count == 0`
- the 96-byte note-to-sample map is preserved
- volume and panning envelopes are read as 12 points each
- envelope values, vibrato depth, and volume fadeout are scaled the way
  MilkyTracker scales them during load
- each sample header is read as the XM 40-byte sample header
- sample loop flags are normalized to core `SampleLoopKind`
- XM's undefined loop flag combination `0x03` is treated as ping-pong loop,
  matching MilkyTracker's load-time normalization
- ModPlug stereo samples are averaged to mono after delta decoding; normalized
  sample frame counts and loop frame positions are halved
- ADPCM-packed XM samples return `UnsupportedAdpcmSample` with instrument/sample
  context until ADPCM decoding is ported
- sample payload bytes are bounds checked and decoded as XM delta-coded PCM
- 8-bit sample data is decoded into signed `i8` frames
- 16-bit sample data is decoded little-endian into signed `i16` frames
- sample frame counts and loop frame positions are normalized for 16-bit sample
  headers while the original XM byte lengths are retained

The parser exposes:

- `XmInstrumentSection`
- `XmInstrument`
- `XmEnvelope`
- `XmEnvelopePoint`
- `XmSampleHeader`
- `XmSampleData`
- `parse_xm_module`

Current tests assert instrument counts, empty-instrument counts, sample counts,
sample-data byte totals, first instrument names, first sample header fields,
instrument-section end offsets, decoded sample frame totals, decoded sample
checksums, and truncated instrument/sample-data failures for all bundled
MilkyTracker XM fixtures. A synthetic test covers 16-bit delta decoding because
the bundled fixtures currently use 8-bit samples.

## Core Module Conversion

`parse_xm_module` composes the tested parser stages into a
`rustytracker-core::Module`:

- module title, channel count, frequency table, restart position, tick speed,
  BPM, main volume, and active order table are copied from the XM header
- packed XM patterns become typed core `Pattern` values
- order entries that reference patterns past the declared pattern count append
  MilkyTracker-compatible empty 64-row patterns
- XM instruments become core `Instrument` values
- sample slots use MilkyTracker's 16-slot-per-instrument pool layout
- note-to-sample maps are converted to absolute core sample indexes where the
  mapped sample exists, otherwise `None`
- volume envelopes, panning envelopes, vibrato metadata, and volume fadeout are
  copied into core instrument fields
- XM sample headers become core `Sample` metadata
- normalized sample loop kinds are copied into core samples
- decoded sample payloads become core `SampleData::Pcm8` or
  `SampleData::Pcm16`

Fixture tests load every bundled MilkyTracker XM file into the core model and
assert headers, orders, pattern counts, instrument counts, sample-pool layout,
first instrument envelope/vibrato/fadeout metadata, first sample metadata,
sample loop kind, decoded data prefixes, and decoded sample checksums.

Next step: continue XM writing with pattern, instrument, and sample payload
serialization.

## Header Writer Contract

`write_xm_header` emits the fixed 336-byte XM header prefix:

- signature, marker, module title, tracker name, version, and header size
- active order count and 256-byte order table
- restart position, channel count, pattern count, and instrument count
- linear/amiga frequency-table flag
- default tick speed and BPM

`write_xm_patterns` emits pattern headers after the module header:

- empty patterns use zero packed payload bytes
- non-empty patterns are emitted as unpacked XM cells
- note-off values are converted back from the core note-off value to XM `97`

Effect inverse mapping, instrument metadata writing, and sample payload writing
now exist in the XM writer. Full normalized roundtrip equality remains a
separate milestone.
