# Coding Standards

## Constants

RustyTracker keeps compatibility values out of algorithm bodies.

Production Rust code must not scatter magic numbers. Values from MilkyTracker,
XM/MOD file formats, tracker limits, byte offsets, bit masks, effect IDs, note
values, sample defaults, conversion factors, and fixed lengths must be expressed
as named constants, associated constants, or enums.

Good constants are domain-specific:

- `XM_PATTERN_HEADER_LEN`
- `XM_CELL_PACKED_FLAG`
- `INTERNAL_EFFECT_VOLUME_SLIDE`
- `DEFAULT_PATTERN_ROWS`

Avoid vague names such as `TWO`, `MASK`, or `OFFSET` unless the surrounding type
or module already supplies the domain.

Tests may keep fixture values inline when the fixture struct or assertion name
already identifies the source file and behavior being pinned. Golden fixture data
should still be grouped in one table instead of duplicated across test bodies.

## Compatibility

Port behavior from MilkyTracker in small, tested slices. Every
compatibility-sensitive parser, editor, or playback behavior starts as a failing
test. Tests should identify whether they pin:

- MilkyTracker empty-song defaults
- XM/MOD file-format structure
- malformed input handling
- packed pattern decoding
- future playback/render output

## Scope

Keep crates focused:

- `rustytracker-core` owns typed domain invariants.
- `rustytracker-xm` owns XM parsing/writing.
- Future playback and editing crates should depend on core types instead of
  embedding parser-specific byte layouts.
