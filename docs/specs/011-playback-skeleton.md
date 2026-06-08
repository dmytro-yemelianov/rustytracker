# Playback Skeleton Spec

## Scope

`rustytracker-play` owns headless playback state and future offline rendering.
The first slices deliberately stop before effects, channel state, sample
interpolation, and PCM mixing. They establish tested cursor and clock state that
walk the core module order list, pattern rows, and ticks.

## References

Playback behavior will be ported from:

- `src/milkyplay/PlayerSTD.cpp` for order, row, tick, and effect progression
- `src/milkyplay/PlayerIT.cpp` where XM/IT-compatible state behavior diverges
- `src/milkyplay/ChannelMixer.cpp` for later sample mixing

## Cursor Contract

`PlaybackCursor::start(&Module)` validates the first order before returning a
cursor. The cursor starts at order `0`, row `0`, and resolves the pattern index
from the module order table.

`PlaybackCursor::position(&Module)` returns:

- current order index
- resolved pattern index
- current row

`PlaybackCursor::advance_row(&Module)`:

- advances to the next row while the current pattern has remaining rows
- advances to row `0` of the next order after the current pattern's last row
- returns `SongEnd` and keeps the cursor unchanged after the final row of the
  final order
- validates the target order and pattern before mutating the cursor

## Timing Contract

`PlaybackTiming::from_module(&Module)` derives initial XM timing from the core
module header:

- `tick_speed` is ticks per row
- `bpm` controls tick duration
- tick duration in nanoseconds is `2_500_000_000 / bpm`
- row duration is tick duration multiplied by ticks per row

`PlaybackClock::start(&Module)` combines a validated `PlaybackCursor`,
validated `PlaybackTiming`, and tick `0`.

`PlaybackClock::advance_tick(&Module)`:

- advances ticks within the current row until the next tick would equal
  `tick_speed`
- advances to the next row or order and resets the tick to `0` after the final
  tick of a non-final row
- returns `SongEnd` and keeps the clock on the final tick of the final row

## Error Contract

The cursor reports structural playback errors explicitly:

- zero tick speed
- zero BPM
- empty order list
- order index beyond the active order list
- order entry that references a missing pattern
- empty pattern
- cursor row outside the resolved pattern's row count

These checks keep future tick/effect code from silently walking invalid module
state.

## Tests

The initial `rustytracker-play` tests verify:

- start position is the first order and first row
- row advance stays in the current order while the pattern has more rows
- row advance moves to the next order when the current pattern ends
- song end is reported after the last row of the last order
- tick and row durations are derived from speed/BPM
- zero speed/BPM timing fields are rejected
- tick advance stays in the row until the row's final tick has elapsed
- tick advance reports song end without moving past the final tick
- empty order lists are rejected
- empty patterns are rejected
- missing pattern references are rejected before playback starts

## Next Steps

- add per-channel row state
- add raw sample stepping without interpolation
- add deterministic PCM render tests once sample stepping exists
