# Orders, Names, Instruments, And Samples

## Fixed Text

MilkyTracker stores names in fixed byte arrays and UI setters copy at most the
visible FT2 lengths:

- module title: 20 bytes
- instrument name: 22 bytes
- sample name: 22 bytes

RustyTracker stores valid Rust strings and truncates at construction time. File
format crates are responsible for fixed-width byte padding when writing XM/MOD.

## Order List

MilkyTracker has 256 bytes of order-list storage but clamps active song length
to `1..=255` in editor operations.

RustyTracker represents this as `OrderList`:

- always contains at least one order
- maximum active length is 255
- `insert_duplicate_after(index)` duplicates the selected pattern number
- `sequence_after(index)` inserts the next pattern number after the highest
  pattern number currently used by the order list
- `delete(index)` never removes the final remaining order

## Instruments

MilkyTracker creates an empty XM-style song with 128 instruments and 16 sample
slots per instrument. RustyTracker keeps that pool shape in the core model for
compatibility.

Each empty instrument has:

- an empty 22-character name
- 16 sample slots mapped to its contiguous sample pool region
- a 96-entry note-to-sample map initialized to sample slot 0

## Samples

Each empty sample has:

- length 0
- loop start 0
- loop length 0
- volume `0xff`
- panning `0x80`
- flags `3`
- volume fadeout `65535`

Raw sample bytes are intentionally not in `rustytracker-core` yet. They will be
introduced with explicit 8-bit/16-bit signed sample buffer types and loop
post-processing tests.

