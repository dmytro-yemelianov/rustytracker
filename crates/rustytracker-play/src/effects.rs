use crate::channel::{PlaybackChannelState, sample_period_for_note};
use crate::envelope::{
    PLAYBACK_ENVELOPE_DEFAULT_PANNING, PLAYBACK_ENVELOPE_DEFAULT_VOLUME, XM_ENVELOPE_ENABLED_FLAG,
};
use rustytracker_core::Module;

pub const EFFECT_VIBRATO: u8 = 0x04;
pub const EFFECT_VIBRATO_VOLSLIDE: u8 = 0x06;
pub const EFFECT_VOLUME: u8 = 0x0c;
pub const EFFECT_PANNING: u8 = 0x08;
pub const EFFECT_VOLUME_SLIDE: u8 = 0x0a;
pub const EFFECT_FINE_VOLUME_SLIDE_UP: u8 = 0x3a;
pub const EFFECT_FINE_VOLUME_SLIDE_DOWN: u8 = 0x3b;
pub const EFFECT_NOTE_CUT: u8 = 0x3c;
pub const EFFECT_ARPEGGIO_ZERO: u8 = 0x00;
pub const EFFECT_PORTAMENTO_UP: u8 = 0x01;
pub const EFFECT_PORTAMENTO_DOWN: u8 = 0x02;
pub const EFFECT_TONE_PORTAMENTO: u8 = 0x03;
pub const EFFECT_SAMPLE_OFFSET: u8 = 0x09;
pub const EFFECT_ARPEGGIO_NONZERO: u8 = 0x20;

pub const EFFECT_GLISSANDO_CONTROL: u8 = 0x33;
pub const EFFECT_VIBRATO_CONTROL: u8 = 0x34;
pub const EFFECT_TREMOLO: u8 = 0x07;
pub const EFFECT_TREMOLO_CONTROL: u8 = 0x37;

pub(crate) const EFFECT_MEMORY_REUSE_OPERAND: u8 = 0;
const EFFECT_LOW_NIBBLE_MASK: u8 = 0x0f;
const EFFECT_HIGH_NIBBLE_SHIFT: u32 = 4;
const EFFECT_VOLUME_SCALE: u8 = 4;
const EFFECT_PORTAMENTO_PERIOD_SCALE: u32 = 4;
pub(crate) const EFFECT_SAMPLE_OFFSET_FRAME_SCALE: usize = 256;
const ARPEGGIO_TICK_CYCLE: u16 = 3;
const ARPEGGIO_BASE_TICK: u16 = 0;
const ARPEGGIO_FIRST_OFFSET_TICK: u16 = 1;
const ARPEGGIO_SECOND_OFFSET_TICK: u16 = 2;
const ARPEGGIO_PERIOD_STEP: i32 = 64;
pub(crate) const PLAYBACK_EMPTY_PERIOD: u32 = 0;
const VIBRATO_TABLE_INDEX_MASK: usize = 31;
const VIBRATO_PHASE_MASK: usize = 63;
const VIBRATO_NEGATIVE_PHASE_START: usize = 31;
const VIBRATO_SCALE_SHIFT: u32 = 5;

pub const VIB_TAB: [i32; 32] = [
    0, 24, 49, 74, 97, 120, 141, 161, 180, 197, 212, 224, 235, 244, 250, 253, 255, 253, 250, 244,
    235, 224, 212, 197, 180, 161, 141, 120, 97, 74, 49, 24,
];

fn should_apply_arpeggio(effect: u8, operand: u8) -> bool {
    effect == EFFECT_ARPEGGIO_NONZERO
        || (effect == EFFECT_ARPEGGIO_ZERO && operand != EFFECT_MEMORY_REUSE_OPERAND)
}

impl PlaybackChannelState {
    pub(crate) fn process_tick_effects(&mut self, module: &Module, tick: u16) {
        self.ensure_effect_memory_slots();

        let mut active_arpeggio_op = None;
        let mut active_vibrato_slot = None;
        let mut active_tremolo_slot = None;

        for (slot_idx, effect) in self.active_effects.iter().enumerate() {
            match effect.effect {
                EFFECT_VOLUME if tick == 0 => {
                    self.base_volume = effect.operand;
                }
                EFFECT_PANNING if tick == 0 => {
                    self.panning = effect.operand;
                }
                EFFECT_FINE_VOLUME_SLIDE_UP if tick == 0 => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.fine_volume_slide_memory;
                    } else {
                        self.fine_volume_slide_memory = op;
                    }
                    self.base_volume = self
                        .base_volume
                        .saturating_add(op.saturating_mul(EFFECT_VOLUME_SCALE));
                }
                EFFECT_FINE_VOLUME_SLIDE_DOWN if tick == 0 => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.fine_volume_slide_memory;
                    } else {
                        self.fine_volume_slide_memory = op;
                    }
                    self.base_volume = self
                        .base_volume
                        .saturating_sub(op.saturating_mul(EFFECT_VOLUME_SCALE));
                }
                EFFECT_NOTE_CUT if tick == u16::from(effect.operand) => {
                    self.base_volume = 0;
                }
                EFFECT_VOLUME_SLIDE if tick > 0 => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.volume_slide_memory;
                    } else {
                        self.volume_slide_memory = op;
                    }
                    let x = op >> EFFECT_HIGH_NIBBLE_SHIFT;
                    let y = op & EFFECT_LOW_NIBBLE_MASK;
                    if x > 0 {
                        self.base_volume = self
                            .base_volume
                            .saturating_add(x.saturating_mul(EFFECT_VOLUME_SCALE));
                    } else if y > 0 {
                        self.base_volume = self
                            .base_volume
                            .saturating_sub(y.saturating_mul(EFFECT_VOLUME_SCALE));
                    }
                }
                EFFECT_GLISSANDO_CONTROL if tick == 0 => {
                    self.glissando = effect.operand != 0;
                }
                EFFECT_VIBRATO_CONTROL if tick == 0 => {
                    self.vibrato_waveform.fill(effect.operand);
                }
                EFFECT_TREMOLO_CONTROL if tick == 0 => {
                    self.tremolo_waveform.fill(effect.operand);
                }
                EFFECT_TREMOLO => {
                    let x = effect.operand >> EFFECT_HIGH_NIBBLE_SHIFT;
                    let y = effect.operand & EFFECT_LOW_NIBBLE_MASK;
                    if x > EFFECT_MEMORY_REUSE_OPERAND {
                        self.tremolo_speed[slot_idx] = x;
                    }
                    if y > EFFECT_MEMORY_REUSE_OPERAND {
                        self.tremolo_depth[slot_idx] = y;
                    }
                    active_tremolo_slot = Some(slot_idx);
                }
                effect_id if should_apply_arpeggio(effect_id, effect.operand) => {
                    let mut op = effect.operand;
                    if effect_id == EFFECT_ARPEGGIO_NONZERO && op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.arpeggio_memory;
                    } else {
                        self.arpeggio_memory = op;
                    }
                    active_arpeggio_op = Some(op);
                }
                EFFECT_PORTAMENTO_UP => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.portamento_up_speed;
                    } else {
                        self.portamento_up_speed = op;
                        self.portamento_speed = op;
                    }
                    if tick > 0 {
                        self.base_period = self
                            .base_period
                            .saturating_sub(u32::from(op) * EFFECT_PORTAMENTO_PERIOD_SCALE);
                    }
                }
                EFFECT_PORTAMENTO_DOWN => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.portamento_down_speed;
                    } else {
                        self.portamento_down_speed = op;
                        self.portamento_speed = op;
                    }
                    if tick > 0 {
                        self.base_period = self
                            .base_period
                            .saturating_add(u32::from(op) * EFFECT_PORTAMENTO_PERIOD_SCALE);
                    }
                }
                EFFECT_TONE_PORTAMENTO => {
                    let mut op = effect.operand;
                    if op == EFFECT_MEMORY_REUSE_OPERAND {
                        op = self.tone_portamento_speed;
                    } else {
                        self.tone_portamento_speed = op;
                    }
                    if tick > 0 && self.target_period > PLAYBACK_EMPTY_PERIOD {
                        if self.base_period < self.target_period {
                            self.base_period = self
                                .base_period
                                .saturating_add(u32::from(op) * EFFECT_PORTAMENTO_PERIOD_SCALE);
                            if self.base_period > self.target_period {
                                self.base_period = self.target_period;
                            }
                        } else if self.base_period > self.target_period {
                            self.base_period = self
                                .base_period
                                .saturating_sub(u32::from(op) * EFFECT_PORTAMENTO_PERIOD_SCALE);
                            if self.base_period < self.target_period {
                                self.base_period = self.target_period;
                            }
                        }
                    }
                }
                EFFECT_VIBRATO => {
                    let x = effect.operand >> EFFECT_HIGH_NIBBLE_SHIFT;
                    let y = effect.operand & EFFECT_LOW_NIBBLE_MASK;
                    if x > EFFECT_MEMORY_REUSE_OPERAND {
                        self.vibrato_speed[slot_idx] = x;
                    }
                    if y > EFFECT_MEMORY_REUSE_OPERAND {
                        self.vibrato_depth[slot_idx] = y;
                    }
                    active_vibrato_slot = Some(slot_idx);
                }
                EFFECT_VIBRATO_VOLSLIDE => {
                    active_vibrato_slot = Some(slot_idx);
                    if tick > 0 {
                        let mut op = effect.operand;
                        if op == EFFECT_MEMORY_REUSE_OPERAND {
                            op = self.volume_slide_memory;
                        } else {
                            self.volume_slide_memory = op;
                        }
                        let x = op >> EFFECT_HIGH_NIBBLE_SHIFT;
                        let y = op & EFFECT_LOW_NIBBLE_MASK;
                        if x > EFFECT_MEMORY_REUSE_OPERAND {
                            self.base_volume = self
                                .base_volume
                                .saturating_add(x.saturating_mul(EFFECT_VOLUME_SCALE));
                        } else if y > EFFECT_MEMORY_REUSE_OPERAND {
                            self.base_volume = self
                                .base_volume
                                .saturating_sub(y.saturating_mul(EFFECT_VOLUME_SCALE));
                        }
                    }
                }
                _ => {}
            }
        }

        let mut pitch_offset = 0;
        let mut volume_offset = 0;

        if let Some(slot) = active_vibrato_slot {
            let vm = self.calc_vibrato(slot);
            if tick > 0 {
                self.vibrato_pos[slot] =
                    self.vibrato_pos[slot].wrapping_add(self.vibrato_speed[slot]);
            }
            pitch_offset += vm;
        }

        if let Some(slot) = active_tremolo_slot {
            let tm = self.calc_tremolo(slot);
            if tick > 0 {
                self.tremolo_pos[slot] =
                    self.tremolo_pos[slot].wrapping_add(self.tremolo_speed[slot]);
            }
            volume_offset += tm;
        }

        if let Some(arpeg_op) = active_arpeggio_op {
            let x = arpeg_op >> EFFECT_HIGH_NIBBLE_SHIFT;
            let y = arpeg_op & EFFECT_LOW_NIBBLE_MASK;
            let offset = match tick % ARPEGGIO_TICK_CYCLE {
                ARPEGGIO_BASE_TICK => 0,
                ARPEGGIO_FIRST_OFFSET_TICK => x,
                ARPEGGIO_SECOND_OFFSET_TICK => y,
                _ => unreachable!(),
            };
            pitch_offset -= i32::from(offset) * ARPEGGIO_PERIOD_STEP;
        }

        let mut effective_base_period = self.base_period;
        if self.glissando {
            if let Some(sample_index) = self.sample_index {
                if let Some(sample) = module.samples.get(sample_index) {
                    let mut min_diff = i32::MAX;
                    let mut best_period = self.base_period;
                    for note in 1..=120 {
                        let p = sample_period_for_note(note, sample, module.header.frequency_table);
                        let diff = (p as i32 - self.base_period as i32).abs();
                        if diff < min_diff {
                            min_diff = diff;
                            best_period = p;
                        }
                    }
                    effective_base_period = best_period;
                }
            }
        }

        self.period = (effective_base_period as i32 + pitch_offset).max(0) as u32;
        self.volume = (self.base_volume as i32 + volume_offset).clamp(0, 255) as u8;

        let mut volume_envelope_val = PLAYBACK_ENVELOPE_DEFAULT_VOLUME;
        let mut panning_envelope_val = PLAYBACK_ENVELOPE_DEFAULT_PANNING;

        if self.active {
            if let Some(ins_idx) = self.instrument_index {
                if ins_idx < module.instruments.len() {
                    let ins = &module.instruments[ins_idx];

                    volume_envelope_val = self
                        .volume_envelope_state
                        .get_value(&ins.volume_envelope, PLAYBACK_ENVELOPE_DEFAULT_VOLUME);
                    panning_envelope_val = self
                        .panning_envelope_state
                        .get_value(&ins.panning_envelope, PLAYBACK_ENVELOPE_DEFAULT_PANNING);

                    if !self.keyon {
                        self.fadeout_volume = self
                            .fadeout_volume
                            .saturating_sub(u32::from(ins.volume_fadeout));
                    }

                    self.volume_envelope_state
                        .advance(&ins.volume_envelope, self.keyon);
                    self.panning_envelope_state
                        .advance(&ins.panning_envelope, self.keyon);

                    if ((ins.volume_envelope.flags & XM_ENVELOPE_ENABLED_FLAG) != 0
                        && volume_envelope_val == 0)
                        || self.fadeout_volume == 0
                    {
                        self.stop_sample();
                    }
                }
            }
        }

        self.volume_envelope_val = volume_envelope_val;
        self.panning_envelope_val = panning_envelope_val;
    }

    pub(crate) fn ensure_effect_memory_slots(&mut self) {
        let len = self.active_effects.len();
        self.vibrato_speed.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
        self.vibrato_depth.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
        self.vibrato_pos.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
        self.vibrato_waveform.resize(len, 0);
        self.tremolo_waveform.resize(len, 0);
        self.tremolo_speed.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
        self.tremolo_depth.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
        self.tremolo_pos.resize(len, EFFECT_MEMORY_REUSE_OPERAND);
    }

    fn calc_vibrato(&self, slot: usize) -> i32 {
        let vp = self.vibrato_pos[slot] as usize;
        let vd = self.vibrato_depth[slot] as i32;
        let wf = self.vibrato_waveform.get(slot).copied().unwrap_or(0) & 3;

        match wf {
            0 => { // Sine
                let tab_val = VIB_TAB[vp & VIBRATO_TABLE_INDEX_MASK];
                let mut vm = (tab_val * vd) >> VIBRATO_SCALE_SHIFT;
                if (vp & VIBRATO_PHASE_MASK) > VIBRATO_NEGATIVE_PHASE_START {
                    vm = -vm;
                }
                vm
            }
            1 => { // Ramp Down
                let p = vp & VIBRATO_PHASE_MASK;
                let tab_val = 255 - (p as i32 * 8);
                (tab_val * vd) >> VIBRATO_SCALE_SHIFT
            }
            2 | 3 => { // Square
                let p = vp & VIBRATO_PHASE_MASK;
                let tab_val = if p < 32 { 255 } else { -255 };
                (tab_val * vd) >> VIBRATO_SCALE_SHIFT
            }
            _ => unreachable!(),
        }
    }

    fn calc_tremolo(&self, slot: usize) -> i32 {
        let vp = self.tremolo_pos[slot] as usize;
        let vd = self.tremolo_depth[slot] as i32;
        let wf = self.tremolo_waveform.get(slot).copied().unwrap_or(0) & 3;

        match wf {
            0 => { // Sine
                let tab_val = VIB_TAB[vp & VIBRATO_TABLE_INDEX_MASK];
                let mut vm = (tab_val * vd) >> VIBRATO_SCALE_SHIFT;
                if (vp & VIBRATO_PHASE_MASK) > VIBRATO_NEGATIVE_PHASE_START {
                    vm = -vm;
                }
                vm
            }
            1 => { // Ramp Down
                let p = vp & VIBRATO_PHASE_MASK;
                let tab_val = 255 - (p as i32 * 8);
                (tab_val * vd) >> VIBRATO_SCALE_SHIFT
            }
            2 | 3 => { // Square
                let p = vp & VIBRATO_PHASE_MASK;
                let tab_val = if p < 32 { 255 } else { -255 };
                (tab_val * vd) >> VIBRATO_SCALE_SHIFT
            }
            _ => unreachable!(),
        }
    }
}
