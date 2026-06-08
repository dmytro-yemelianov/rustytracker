//! Playback cursor and timing skeleton for RustyTracker.
//!
//! Audio mixing and effect execution will build on this crate. The first slice
//! keeps traversal explicit and testable.

use rustytracker_core::{Module, Pattern};

pub const PLAYBACK_FIRST_ORDER_INDEX: usize = 0;
pub const PLAYBACK_FIRST_ROW: u16 = 0;
pub const PLAYBACK_ORDER_STEP: usize = 1;
pub const PLAYBACK_ROW_STEP: u16 = 1;
pub const PLAYBACK_EMPTY_PATTERN_ROWS: u16 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
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
