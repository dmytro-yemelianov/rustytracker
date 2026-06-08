//! Playback cursor and timing skeleton for RustyTracker.
//!
//! Audio mixing and effect execution will build on this crate. The first slice
//! keeps traversal explicit and testable.

use rustytracker_core::{Module, Pattern};

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
}

pub type PlaybackResult<T> = Result<T, PlaybackError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackPosition {
    pub order_index: usize,
    pub pattern_index: usize,
    pub row: u16,
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
