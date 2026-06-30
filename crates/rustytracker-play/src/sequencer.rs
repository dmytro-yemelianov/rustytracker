use rustytracker_core::{Module, Note};

use crate::channel::PlaybackChannelState;
use crate::error::PlaybackResult;
use crate::{PlaybackClock, PlaybackPosition, PlaybackRowState, TickAdvance};
use crate::{
    EFFECT_PATTERN_BREAK, EFFECT_POSITION_JUMP, EFFECT_SET_SPEED_BPM, SPEED_BPM_THRESHOLD,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerCommand {
    Trigger {
        channel: u16,
        sample_index: usize,
        instrument_index: usize,
        note: Note,
        instrument: u8,
        volume: u8,
        panning: u8,
        period: u32,
        offset: Option<usize>,
    },
    Update {
        channel: u16,
        sample_index: Option<usize>,
        volume: u8,
        panning: u8,
        period: u32,
        volume_envelope_val: u16,
        panning_envelope_val: u16,
        fadeout_volume: u32,
        keyon: bool,
    },
    Stop {
        channel: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequencer {
    pub clock: PlaybackClock,
    pub channels: Vec<PlaybackChannelState>,
    pub song_ended: bool,
}

impl Sequencer {
    pub fn start_with_config(module: &Module) -> PlaybackResult<Self> {
        let clock = PlaybackClock::start(module)?;
        let row_state = clock.row_state(module)?;
        let channels = row_state
            .channels
            .iter()
            .map(|channel| PlaybackChannelState::empty(channel.channel))
            .collect();
        let mut seq = Self {
            clock,
            channels,
            song_ended: false,
        };
        seq.apply_row_state(module, &row_state)?;
        Ok(seq)
    }

    pub fn advance_tick(
        &mut self,
        module: &Module,
    ) -> PlaybackResult<(TickAdvance, Vec<SequencerCommand>)> {
        if self.song_ended {
            return Ok((TickAdvance::SongEnd, Vec::new()));
        }

        // Reset transient flags
        for ch in &mut self.channels {
            ch.triggered = None;
            ch.stopped = false;
        }

        let advance = self.clock.advance_tick(module)?;
        match advance {
            TickAdvance::NextRow | TickAdvance::NextOrder => self.trigger_current_row(module)?,
            TickAdvance::SameRow => {
                let current_tick = self.clock.tick();
                for channel in &mut self.channels {
                    channel.process_tick_effects(module, current_tick);
                }
            }
            TickAdvance::SongEnd => {
                self.song_ended = true;
            }
        }

        // Generate commands
        let mut commands = Vec::new();
        for channel in &self.channels {
            if channel.stopped {
                commands.push(SequencerCommand::Stop {
                    channel: channel.channel,
                });
            } else if let Some(offset) = channel.triggered {
                if let (Some(sample_index), Some(instrument_index)) =
                    (channel.sample_index, channel.instrument_index)
                {
                    commands.push(SequencerCommand::Trigger {
                        channel: channel.channel,
                        sample_index,
                        instrument_index,
                        note: channel.note,
                        instrument: channel.instrument,
                        volume: channel.volume,
                        panning: channel.panning,
                        period: channel.period,
                        offset,
                    });
                }
            } else if channel.active {
                commands.push(SequencerCommand::Update {
                    channel: channel.channel,
                    sample_index: channel.sample_index,
                    volume: channel.volume,
                    panning: channel.panning,
                    period: channel.period,
                    volume_envelope_val: channel.volume_envelope_val,
                    panning_envelope_val: channel.panning_envelope_val,
                    fadeout_volume: channel.fadeout_volume,
                    keyon: channel.keyon,
                });
            }
        }

        Ok((advance, commands))
    }

    pub fn generate_initial_commands(&self) -> Vec<SequencerCommand> {
        let mut commands = Vec::new();
        for channel in &self.channels {
            if channel.active {
                if let (Some(sample_index), Some(instrument_index)) =
                    (channel.sample_index, channel.instrument_index)
                {
                    commands.push(SequencerCommand::Trigger {
                        channel: channel.channel,
                        sample_index,
                        instrument_index,
                        note: channel.note,
                        instrument: channel.instrument,
                        volume: channel.volume,
                        panning: channel.panning,
                        period: channel.period,
                        offset: if channel.sample_frame > 0 {
                            Some(channel.sample_frame)
                        } else {
                            None
                        },
                    });
                }
            }
        }
        commands
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
        let mut requested_order = None;
        let mut requested_row = None;

        for channel in &row_state.channels {
            for effect in &channel.cell.effects {
                if effect.effect == EFFECT_SET_SPEED_BPM {
                    if effect.operand == 0 {
                        self.song_ended = true;
                    } else if effect.operand < SPEED_BPM_THRESHOLD {
                        self.clock.set_tick_speed(u16::from(effect.operand))?;
                    } else {
                        self.clock.set_bpm(u16::from(effect.operand))?;
                    }
                } else if effect.effect == EFFECT_POSITION_JUMP {
                    requested_order = Some(usize::from(effect.operand));
                } else if effect.effect == EFFECT_PATTERN_BREAK {
                    let bcd = effect.operand;
                    let row = u16::from(bcd >> 4) * 10 + u16::from(bcd & 0x0f);
                    requested_row = Some(row);
                }
            }
        }

        if requested_order.is_some() || requested_row.is_some() {
            let current_pos = self.clock.position(module)?;
            let target_order = match requested_order {
                Some(order) => order,
                None => {
                    let next_order = current_pos.order_index + 1;
                    if next_order >= module.orders.len() {
                        usize::from(module.header.restart_position)
                    } else {
                        next_order
                    }
                }
            };
            let target_row = requested_row.unwrap_or_default();
            self.clock.set_jump_target(PlaybackPosition {
                order_index: target_order,
                pattern_index: usize::from(module.orders[target_order]),
                row: target_row,
            });
        }

        for channel in &row_state.channels {
            let ch_state = &mut self.channels[usize::from(channel.channel)];
            ch_state.apply_cell(module, &channel.cell)?;
            ch_state.process_tick_effects(module, 0);
        }

        Ok(())
    }
}
