# Playback Skeleton Spec

## Scope

`rustytracker-play` owns headless playback state and future offline rendering.
The first slices deliberately stop before effects, sample interpolation, and
PCM mixing. They establish tested cursor, clock, current-row channel snapshots,
mutable per-channel trigger state, raw decoded sample stepping, and fixed-count
raw mono PCM rendering that walk the core module order list, pattern rows,
ticks, and active module channels.

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

## Row State Contract

`PlaybackCursor::row_state(&Module)` and `PlaybackClock::row_state(&Module)`
return a `PlaybackRowState` for the current position:

- the resolved playback position
- one `ChannelRowState` per active module channel
- each channel's cloned `PatternCell` for the current row

The row state is a read-only snapshot. It does not carry effect memory, update
envelopes, or advance sample playback.

## Playback State Contract

`PlaybackState::start(&Module)` combines a `PlaybackClock` with one mutable
`PlaybackChannelState` per active module channel. It applies triggers from the
first row before returning.

Each channel state tracks:

- channel index
- whether a sample is active
- current note and instrument number
- resolved instrument index
- resolved core sample index
- sample frame, starting at `0` when a note triggers
- sample volume and panning copied from the resolved sample

Trigger behavior:

- non-empty instrument numbers are one-based XM instrument numbers and update
  instrument memory
- key notes trigger the current or newly supplied instrument
- note-only rows reuse prior instrument memory
- empty cells preserve current channel state
- note-off cells mark the channel inactive and clear the active sample
- missing instruments and out-of-range sample references return explicit
  playback errors

## Sample Step Contract

`PlaybackState::step_samples(&Module)` emits one decoded sample frame per
active channel that currently points at readable sample data, in channel order.

The raw sample frame output includes:

- channel index
- resolved core sample index
- sample frame index before advancing
- decoded sample value as either `Pcm8(i8)` or `Pcm16(i16)`

After emitting a frame, the channel's sample frame advances by one decoded
frame. When the frame just emitted is the final decoded frame, the channel stops
its active sample and resets the sample frame to `0`. Empty sample data stops
the active sample without emitting a frame.

This stepper deliberately does not yet apply pitch, interpolation, loop modes,
volume, panning, effect memory, envelopes, or cross-channel mixing.

## Raw Mono Render Contract

`PlaybackState::render_raw_mono_pcm(&Module, frame_count)` renders a fixed
number of deterministic mono PCM frames by repeatedly calling
`step_samples(&Module)`.

For each output frame:

- active channel sample frames are read in channel order
- PCM8 values are widened to signed 16-bit scale by shifting left by 8 bits
- PCM16 values are used at their decoded signed 16-bit scale
- channel values are summed into an `i32` mono frame without clipping
- if no channel emits a sample frame, the mono output frame is `0`

The raw mono renderer deliberately does not yet apply pitch, interpolation, loop
modes, volume, panning, sample-rate timing, row/tick advancement, effect memory,
or envelopes.

## CLI Trace Contract

`rustytracker play-state <module.xm> --rows <count>` loads an XM file and emits
a deterministic JSON trace of the first playback rows:

- schema version and `play_state` format tag
- requested row count and whether song end was reached
- initial BPM, ticks per row, tick duration, and row duration
- one row entry per visited row
- each row entry includes order index, pattern index, row, tick, and active
  channel cells
- each channel entry includes the raw row cell plus the current mutable playback
  channel state after that row's triggers have been applied

The trace is intentionally not audio output. It gives contributors something
concrete to compile and run while the playback crate is still below sample
mixing.

## Error Contract

The cursor reports structural playback errors explicitly:

- zero tick speed
- zero BPM
- empty order list
- order index beyond the active order list
- order entry that references a missing pattern
- empty pattern
- cursor row outside the resolved pattern's row count
- pattern channel count smaller than the active module channel count
- missing instrument references in trigger rows
- note sample maps that point outside the module sample table

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
- current row state returns one cell per active module channel
- row state follows tick-driven row advancement
- playback state triggers initial note/instrument/sample state
- empty rows preserve active channel state
- note-only rows reuse prior instrument memory
- note-off rows release active channel state
- missing instruments and missing samples are rejected explicitly
- sample stepping emits decoded PCM8 and PCM16 frames without interpolation
- sample stepping advances sample frame positions and stops after the final frame
- empty sample data stops the active sample without emitting a frame
- raw mono rendering widens PCM8, preserves PCM16 scale, sums active channels,
  and emits silence for the requested frames after samples end
- patterns with too few channels for the module are rejected
- `play-state` rejects missing, non-numeric, or zero row counts
- empty order lists are rejected
- empty patterns are rejected
- missing pattern references are rejected before playback starts

## Next Steps

- connect raw mono rendering to tick/row progression and future sample-rate
  timing
