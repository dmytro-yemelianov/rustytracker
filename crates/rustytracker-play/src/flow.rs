use crate::cursor::{PlaybackClock, PlaybackPosition, PlaybackRowState};
use crate::error::PlaybackResult;
use rustytracker_core::Module;

pub const EFFECT_SET_SPEED_BPM: u8 = 0x0f;
pub const SPEED_BPM_THRESHOLD: u8 = 32;
pub const EFFECT_POSITION_JUMP: u8 = 0x0b;
pub const EFFECT_PATTERN_BREAK: u8 = 0x0d;

pub(crate) fn apply_row_flow(
    clock: &mut PlaybackClock,
    module: &Module,
    row_state: &PlaybackRowState,
) -> PlaybackResult<bool> {
    let mut requested_order = None;
    let mut requested_row = None;
    let mut song_ended = false;

    for channel in &row_state.channels {
        for effect in &channel.cell.effects {
            if effect.effect == EFFECT_SET_SPEED_BPM {
                if effect.operand == 0 {
                    song_ended = true;
                } else if effect.operand < SPEED_BPM_THRESHOLD {
                    clock.set_tick_speed(u16::from(effect.operand))?;
                } else {
                    clock.set_bpm(u16::from(effect.operand))?;
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
        let current_pos = clock.position(module)?;

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

        clock.set_jump_target(PlaybackPosition {
            order_index: target_order,
            pattern_index: 0,
            row: target_row,
        });
    }

    Ok(song_ended)
}
