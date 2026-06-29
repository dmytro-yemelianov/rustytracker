use crate::error::{PlaybackError, PlaybackResult};
use crate::timing::PlaybackTiming;
use rustytracker_core::{Module, Pattern, PatternCell};

pub const PLAYBACK_FIRST_CHANNEL: u16 = 0;
pub const PLAYBACK_FIRST_ORDER_INDEX: usize = 0;
pub const PLAYBACK_FIRST_ROW: u16 = 0;
pub const PLAYBACK_FIRST_TICK: u16 = 0;
pub const PLAYBACK_ORDER_STEP: usize = 1;
pub const PLAYBACK_ROW_STEP: u16 = 1;
pub const PLAYBACK_TICK_STEP: u16 = 1;
pub const PLAYBACK_EMPTY_PATTERN_ROWS: u16 = 0;

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
pub struct PlaybackCursor {
    order_index: usize,
    row: u16,
    jump_target: Option<PlaybackPosition>,
}

impl PlaybackCursor {
    pub fn order_index(&self) -> usize {
        self.order_index
    }

    pub fn row(&self) -> u16 {
        self.row
    }

    pub fn start(module: &Module) -> PlaybackResult<Self> {
        let cursor = Self {
            order_index: PLAYBACK_FIRST_ORDER_INDEX,
            row: PLAYBACK_FIRST_ROW,
            jump_target: None,
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
        if let Some(target) = self.jump_target {
            self.jump_target = None;

            if target.order_index >= module.orders.len() {
                return Err(PlaybackError::OrderIndexOutOfRange {
                    order_index: target.order_index,
                    order_count: module.orders.len(),
                });
            }
            let pattern_index = pattern_index_for_order(module, target.order_index)?;
            let pattern = &module.patterns[pattern_index];

            let target_row = if target.row >= pattern.rows() {
                PLAYBACK_FIRST_ROW
            } else {
                target.row
            };

            let old_order = self.order_index;
            self.order_index = target.order_index;
            self.row = target_row;

            if self.order_index != old_order {
                Ok(RowAdvance::NextOrder)
            } else {
                Ok(RowAdvance::SameOrder)
            }
        } else {
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

    pub fn set_jump_target(&mut self, target: PlaybackPosition) {
        self.jump_target = Some(target);
    }

    pub fn jump_target(&self) -> Option<PlaybackPosition> {
        self.jump_target
    }

    pub fn clear_jump_target(&mut self) {
        self.jump_target = None;
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

    pub fn set_bpm(&mut self, bpm: u16) -> PlaybackResult<()> {
        self.timing.set_bpm(bpm)
    }

    pub fn set_tick_speed(&mut self, tick_speed: u16) -> PlaybackResult<()> {
        self.timing.set_tick_speed(tick_speed)
    }

    pub fn set_jump_target(&mut self, target: PlaybackPosition) {
        self.cursor.set_jump_target(target);
    }

    pub fn jump_target(&self) -> Option<PlaybackPosition> {
        self.cursor.jump_target()
    }

    pub fn clear_jump_target(&mut self) {
        self.cursor.clear_jump_target();
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
