//! Playback cursor and timing skeleton for RustyTracker.
//!
//! Audio mixing and effect execution will build on this crate. The first slice
//! keeps traversal explicit and testable.

use rustytracker_core::{
    Module, Note, Pattern, PatternCell, DEFAULT_INSTRUMENT_NUMBER, FIRST_XM_NOTE_VALUE,
    SAMPLE_DEFAULT_PANNING,
};

pub const PLAYBACK_FIRST_CHANNEL: u16 = 0;
pub const PLAYBACK_FIRST_ORDER_INDEX: usize = 0;
pub const PLAYBACK_FIRST_ROW: u16 = 0;
pub const PLAYBACK_FIRST_TICK: u16 = 0;
pub const PLAYBACK_ORDER_STEP: usize = 1;
pub const PLAYBACK_ROW_STEP: u16 = 1;
pub const PLAYBACK_TICK_STEP: u16 = 1;
pub const PLAYBACK_EMPTY_PATTERN_ROWS: u16 = 0;
pub const PLAYBACK_MIN_TICK_SPEED: u16 = 1;
pub const PLAYBACK_MIN_BPM: u16 = 1;
pub const PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM: u64 = 2_500_000_000;
pub const PLAYBACK_INSTRUMENT_NUMBER_BASE: u8 = 1;
pub const PLAYBACK_SAMPLE_START_FRAME: usize = 0;
pub const PLAYBACK_EMPTY_VOLUME: u8 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
    InvalidTickSpeed {
        tick_speed: u16,
    },
    InvalidBpm {
        bpm: u16,
    },
    EmptyOrderList,
    OrderIndexOutOfRange {
        order_index: usize,
        order_count: usize,
    },
    MissingPattern {
        order_index: usize,
        pattern_index: usize,
    },
    EmptyPattern {
        pattern_index: usize,
    },
    RowOutOfRange {
        pattern_index: usize,
        row: u16,
        rows: u16,
    },
    PatternChannelOutOfRange {
        pattern_index: usize,
        module_channels: u16,
        pattern_channels: u16,
    },
    MissingInstrument {
        channel: u16,
        instrument: u8,
    },
    MissingSample {
        channel: u16,
        instrument_index: usize,
        sample_index: usize,
    },
}

pub type PlaybackResult<T> = Result<T, PlaybackError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackPosition {
    pub order_index: usize,
    pub pattern_index: usize,
    pub row: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelRowState {
    pub channel: u16,
    pub cell: PatternCell,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackRowState {
    pub position: PlaybackPosition,
    pub channels: Vec<ChannelRowState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackChannelState {
    pub channel: u16,
    pub active: bool,
    pub note: Note,
    pub instrument: u8,
    pub instrument_index: Option<usize>,
    pub sample_index: Option<usize>,
    pub sample_frame: usize,
    pub volume: u8,
    pub panning: u8,
}

impl PlaybackChannelState {
    fn empty(channel: u16) -> Self {
        Self {
            channel,
            active: false,
            note: Note::Empty,
            instrument: DEFAULT_INSTRUMENT_NUMBER,
            instrument_index: None,
            sample_index: None,
            sample_frame: PLAYBACK_SAMPLE_START_FRAME,
            volume: PLAYBACK_EMPTY_VOLUME,
            panning: SAMPLE_DEFAULT_PANNING,
        }
    }

    fn apply_cell(&mut self, module: &Module, cell: &PatternCell) -> PlaybackResult<()> {
        if cell.instrument != DEFAULT_INSTRUMENT_NUMBER {
            self.set_instrument(module, cell.instrument)?;
        }

        match cell.note {
            Note::Empty => Ok(()),
            Note::Off => {
                self.release();
                Ok(())
            }
            Note::Key(note) => self.trigger_key(module, note),
        }
    }

    fn set_instrument(&mut self, module: &Module, instrument: u8) -> PlaybackResult<()> {
        let Some(instrument_index) = instrument_index_for_number(instrument) else {
            return Err(PlaybackError::MissingInstrument {
                channel: self.channel,
                instrument,
            });
        };
        if instrument_index >= module.instruments.len() {
            return Err(PlaybackError::MissingInstrument {
                channel: self.channel,
                instrument,
            });
        }

        self.instrument = instrument;
        self.instrument_index = Some(instrument_index);
        Ok(())
    }

    fn trigger_key(&mut self, module: &Module, note: u8) -> PlaybackResult<()> {
        self.note = Note::Key(note);
        self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;

        let Some(instrument_index) = self.instrument_index else {
            self.active = false;
            self.sample_index = None;
            return Ok(());
        };
        let Some(note_index) = note_sample_map_index(note) else {
            self.active = false;
            self.sample_index = None;
            return Ok(());
        };
        let Some(sample_index) = module.instruments[instrument_index]
            .note_sample_map
            .get(note_index)
            .and_then(|sample_index| *sample_index)
        else {
            self.active = false;
            self.sample_index = None;
            return Ok(());
        };
        let Some(sample) = module.samples.get(sample_index) else {
            return Err(PlaybackError::MissingSample {
                channel: self.channel,
                instrument_index,
                sample_index,
            });
        };

        self.active = true;
        self.sample_index = Some(sample_index);
        self.volume = sample.volume;
        self.panning = sample.panning;
        Ok(())
    }

    fn release(&mut self) {
        self.active = false;
        self.note = Note::Off;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAdvance {
    SameOrder,
    NextOrder,
    SongEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickAdvance {
    SameRow,
    NextRow,
    NextOrder,
    SongEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackTiming {
    pub tick_speed: u16,
    pub bpm: u16,
    pub tick_duration_nanos: u64,
}

impl PlaybackTiming {
    pub fn from_module(module: &Module) -> PlaybackResult<Self> {
        let tick_speed = module.header.tick_speed;
        if tick_speed < PLAYBACK_MIN_TICK_SPEED {
            return Err(PlaybackError::InvalidTickSpeed { tick_speed });
        }

        let bpm = module.header.bpm;
        if bpm < PLAYBACK_MIN_BPM {
            return Err(PlaybackError::InvalidBpm { bpm });
        }

        Ok(Self {
            tick_speed,
            bpm,
            tick_duration_nanos: PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM / u64::from(bpm),
        })
    }

    pub fn ticks_per_row(&self) -> u16 {
        self.tick_speed
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn tick_duration_nanos(&self) -> u64 {
        self.tick_duration_nanos
    }

    pub fn row_duration_nanos(&self) -> u64 {
        self.tick_duration_nanos
            .saturating_mul(u64::from(self.tick_speed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackCursor {
    order_index: usize,
    row: u16,
}

impl PlaybackCursor {
    pub fn start(module: &Module) -> PlaybackResult<Self> {
        let cursor = Self {
            order_index: PLAYBACK_FIRST_ORDER_INDEX,
            row: PLAYBACK_FIRST_ROW,
        };
        cursor.position(module)?;
        Ok(cursor)
    }

    pub fn position(&self, module: &Module) -> PlaybackResult<PlaybackPosition> {
        let pattern_index = pattern_index_for_order(module, self.order_index)?;
        pattern_for_row(module, pattern_index, self.row)?;

        Ok(PlaybackPosition {
            order_index: self.order_index,
            pattern_index,
            row: self.row,
        })
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        row_state_for_position(module, self.position(module)?)
    }

    pub fn advance_row(&mut self, module: &Module) -> PlaybackResult<RowAdvance> {
        let position = self.position(module)?;
        let pattern = pattern_for_row(module, position.pattern_index, position.row)?;
        let next_row = position.row.saturating_add(PLAYBACK_ROW_STEP);

        if next_row < pattern.rows() {
            self.row = next_row;
            return Ok(RowAdvance::SameOrder);
        }

        let next_order_index = position.order_index + PLAYBACK_ORDER_STEP;
        if next_order_index < module.orders.len() {
            let next_pattern_index = pattern_index_for_order(module, next_order_index)?;
            pattern_for_row(module, next_pattern_index, PLAYBACK_FIRST_ROW)?;
            self.order_index = next_order_index;
            self.row = PLAYBACK_FIRST_ROW;
            return Ok(RowAdvance::NextOrder);
        }

        Ok(RowAdvance::SongEnd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackClock {
    cursor: PlaybackCursor,
    timing: PlaybackTiming,
    tick: u16,
}

impl PlaybackClock {
    pub fn start(module: &Module) -> PlaybackResult<Self> {
        Ok(Self {
            cursor: PlaybackCursor::start(module)?,
            timing: PlaybackTiming::from_module(module)?,
            tick: PLAYBACK_FIRST_TICK,
        })
    }

    pub fn cursor(&self) -> PlaybackCursor {
        self.cursor
    }

    pub fn timing(&self) -> PlaybackTiming {
        self.timing
    }

    pub fn tick(&self) -> u16 {
        self.tick
    }

    pub fn position(&self, module: &Module) -> PlaybackResult<PlaybackPosition> {
        self.cursor.position(module)
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        self.cursor.row_state(module)
    }

    pub fn advance_tick(&mut self, module: &Module) -> PlaybackResult<TickAdvance> {
        let next_tick = self.tick.saturating_add(PLAYBACK_TICK_STEP);
        if next_tick < self.timing.tick_speed {
            self.tick = next_tick;
            return Ok(TickAdvance::SameRow);
        }

        match self.cursor.advance_row(module)? {
            RowAdvance::SameOrder => {
                self.tick = PLAYBACK_FIRST_TICK;
                Ok(TickAdvance::NextRow)
            }
            RowAdvance::NextOrder => {
                self.tick = PLAYBACK_FIRST_TICK;
                Ok(TickAdvance::NextOrder)
            }
            RowAdvance::SongEnd => Ok(TickAdvance::SongEnd),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackState {
    clock: PlaybackClock,
    channels: Vec<PlaybackChannelState>,
}

impl PlaybackState {
    pub fn start(module: &Module) -> PlaybackResult<Self> {
        let clock = PlaybackClock::start(module)?;
        let row_state = clock.row_state(module)?;
        let channels = row_state
            .channels
            .iter()
            .map(|channel| PlaybackChannelState::empty(channel.channel))
            .collect();
        let mut state = Self { clock, channels };
        state.apply_row_state(module, &row_state)?;
        Ok(state)
    }

    pub fn clock(&self) -> PlaybackClock {
        self.clock
    }

    pub fn channels(&self) -> &[PlaybackChannelState] {
        &self.channels
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        self.clock.row_state(module)
    }

    pub fn advance_tick(&mut self, module: &Module) -> PlaybackResult<TickAdvance> {
        let advance = self.clock.advance_tick(module)?;
        match advance {
            TickAdvance::NextRow | TickAdvance::NextOrder => self.trigger_current_row(module)?,
            TickAdvance::SameRow | TickAdvance::SongEnd => {}
        }
        Ok(advance)
    }

    fn trigger_current_row(&mut self, module: &Module) -> PlaybackResult<()> {
        let row_state = self.clock.row_state(module)?;
        self.apply_row_state(module, &row_state)
    }

    fn apply_row_state(
        &mut self,
        module: &Module,
        row_state: &PlaybackRowState,
    ) -> PlaybackResult<()> {
        for channel in &row_state.channels {
            self.channels[usize::from(channel.channel)].apply_cell(module, &channel.cell)?;
        }

        Ok(())
    }
}

fn pattern_index_for_order(module: &Module, order_index: usize) -> PlaybackResult<usize> {
    if module.orders.is_empty() {
        return Err(PlaybackError::EmptyOrderList);
    }

    let pattern_index = usize::from(*module.orders.get(order_index).ok_or(
        PlaybackError::OrderIndexOutOfRange {
            order_index,
            order_count: module.orders.len(),
        },
    )?);

    if pattern_index >= module.patterns.len() {
        return Err(PlaybackError::MissingPattern {
            order_index,
            pattern_index,
        });
    }

    Ok(pattern_index)
}

fn pattern_for_row(module: &Module, pattern_index: usize, row: u16) -> PlaybackResult<&Pattern> {
    let pattern = &module.patterns[pattern_index];

    if pattern.rows() == PLAYBACK_EMPTY_PATTERN_ROWS {
        return Err(PlaybackError::EmptyPattern { pattern_index });
    }

    if row >= pattern.rows() {
        return Err(PlaybackError::RowOutOfRange {
            pattern_index,
            row,
            rows: pattern.rows(),
        });
    }

    Ok(pattern)
}

fn row_state_for_position(
    module: &Module,
    position: PlaybackPosition,
) -> PlaybackResult<PlaybackRowState> {
    let pattern = &module.patterns[position.pattern_index];
    let module_channels = module.header.channel_count;
    let pattern_channels = pattern.channels();

    if module_channels > pattern_channels {
        return Err(PlaybackError::PatternChannelOutOfRange {
            pattern_index: position.pattern_index,
            module_channels,
            pattern_channels,
        });
    }

    let channels = (PLAYBACK_FIRST_CHANNEL..module_channels)
        .map(|channel| ChannelRowState {
            channel,
            cell: pattern
                .cell(channel, position.row)
                .expect("row state validates channel and row bounds before reading")
                .clone(),
        })
        .collect();

    Ok(PlaybackRowState { position, channels })
}

fn instrument_index_for_number(instrument: u8) -> Option<usize> {
    instrument
        .checked_sub(PLAYBACK_INSTRUMENT_NUMBER_BASE)
        .map(usize::from)
}

fn note_sample_map_index(note: u8) -> Option<usize> {
    note.checked_sub(FIRST_XM_NOTE_VALUE).map(usize::from)
}
