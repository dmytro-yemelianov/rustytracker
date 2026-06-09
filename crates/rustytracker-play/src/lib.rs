//! Playback cursor and timing skeleton for RustyTracker.
//!
//! Audio mixing and effect execution will build on this crate. The first slice
//! keeps traversal explicit and testable.

use rustytracker_core::{
    EffectCommand, FrequencyTable, Module, Note, Pattern, PatternCell, Sample, SampleData,
    SampleLoopKind, DEFAULT_EFFECT_SLOTS, DEFAULT_INSTRUMENT_NUMBER, FIRST_XM_NOTE_VALUE,
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
pub const PLAYBACK_SAMPLE_FRAME_STEP: usize = 1;
pub const PLAYBACK_EMPTY_VOLUME: u8 = 0;
pub const PLAYBACK_PCM8_TO_I16_SHIFT: u32 = 8;
pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const EFFECT_SET_SPEED_BPM: u8 = 0x0f;
pub const SPEED_BPM_THRESHOLD: u8 = 32;

pub const EFFECT_VIBRATO: u8 = 0x04;
pub const EFFECT_VIBRATO_VOLSLIDE: u8 = 0x06;
pub const EFFECT_VOLUME: u8 = 0x0c;
pub const EFFECT_PANNING: u8 = 0x08;
pub const EFFECT_VOLUME_SLIDE: u8 = 0x0a;
pub const EFFECT_FINE_VOLUME_SLIDE_UP: u8 = 0x3a;
pub const EFFECT_FINE_VOLUME_SLIDE_DOWN: u8 = 0x3b;
pub const EFFECT_POSITION_JUMP: u8 = 0x0b;
pub const EFFECT_PATTERN_BREAK: u8 = 0x0d;

pub const EFFECT_ARPEGGIO_ZERO: u8 = 0x00;
pub const EFFECT_PORTAMENTO_UP: u8 = 0x01;
pub const EFFECT_PORTAMENTO_DOWN: u8 = 0x02;
pub const EFFECT_TONE_PORTAMENTO: u8 = 0x03;
pub const EFFECT_SAMPLE_OFFSET: u8 = 0x09;
pub const EFFECT_ARPEGGIO_NONZERO: u8 = 0x20;

pub const VIB_TAB: [i32; 32] = [
    0, 24, 49, 74, 97, 120, 141, 161, 180, 197, 212, 224, 235, 244, 250, 253, 255, 253, 250, 244,
    235, 224, 212, 197, 180, 161, 141, 120, 97, 74, 49, 24,
];

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackSampleValue {
    Pcm8(i8),
    Pcm16(i16),
}

impl PlaybackSampleValue {
    pub fn raw_mono_pcm(self) -> RawMonoPcmFrame {
        match self {
            Self::Pcm8(value) => RawMonoPcmFrame::from(value) << PLAYBACK_PCM8_TO_I16_SHIFT,
            Self::Pcm16(value) => RawMonoPcmFrame::from(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSampleFrame {
    pub channel: u16,
    pub sample_index: usize,
    pub sample_frame: usize,
    pub value: PlaybackSampleValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackEnvelopeState {
    pub a: usize,
    pub b: usize,
    pub step: u16,
}

impl PlaybackEnvelopeState {
    pub fn new() -> Self {
        Self {
            a: 0,
            b: 1,
            step: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.b = 1;
        self.step = 0;
    }

    pub fn advance(&mut self, env: &rustytracker_core::Envelope, keyon: bool) {
        if env.points.is_empty() || (env.flags & 0x01) == 0 {
            return;
        }

        let num = env.points.len();

        // if we're sitting on a sustain point and key is on, we don't advance further
        if (env.flags & 0x02) != 0
            && self.a == env.sustain_point as usize
            && self.a < num
            && self.step == env.points[self.a].frame
            && keyon
        {
            return;
        }

        if self.b < num && self.step != env.points[self.b].frame {
            self.step += 1;
        }

        if self.b < num && self.step == env.points[self.b].frame {
            // Check loop
            if (env.flags & 0x04) != 0 {
                let break_loop =
                    !keyon && (env.flags & 0x02) != 0 && env.sustain_point == env.loop_end_point;

                if !break_loop && self.b == env.loop_end_point as usize {
                    self.a = env.loop_start_point as usize;
                    self.b = (env.loop_start_point + 1) as usize;
                    if self.a < num {
                        self.step = env.points[self.a].frame;
                    }
                    return;
                }
            }

            // Increase envelope position if there are more points to come
            if self.b < num - 1 {
                self.a += 1;
                self.b += 1;
            }
        }
    }

    pub fn get_value(&self, env: &rustytracker_core::Envelope, default_val: u16) -> u16 {
        if env.points.is_empty() || (env.flags & 0x01) == 0 {
            return default_val;
        }

        let num = env.points.len();
        let idx_a = self.a.min(num - 1);
        let idx_b = self.b.min(num - 1);

        if idx_a == idx_b {
            return env.points[idx_a].value;
        }

        let p_a = env.points[idx_a];
        let p_b = env.points[idx_b];

        let mut dx = p_b.frame as i32 - p_a.frame as i32;
        if dx == 0 {
            dx = 1;
        }

        let t = (p_b.frame as i32 - self.step as i32) * 65536 / dx;
        let y0 = p_a.value as i32;
        let y1 = p_b.value as i32;

        let y = (y0 * t) + (y1 * (65536 - t));
        (y >> 16) as u16
    }
}

impl Default for PlaybackEnvelopeState {
    fn default() -> Self {
        Self::new()
    }
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
    pub sample_frame_fraction: u32,
    pub volume: u8,
    pub panning: u8,
    pub active_effects: Vec<EffectCommand>,
    pub volume_slide_memory: u8,
    pub fine_volume_slide_memory: u8,
    pub period: u32,
    pub base_period: u32,
    pub target_period: u32,
    pub portamento_speed: u8,
    pub portamento_up_speed: u8,
    pub portamento_down_speed: u8,
    pub tone_portamento_speed: u8,
    pub arpeggio_memory: u8,
    pub vibrato_speed: [u8; 2],
    pub vibrato_depth: [u8; 2],
    pub vibrato_pos: [u8; 2],
    pub sample_offset_memory: u8,
    pub sample_backward: bool,
    pub keyon: bool,
    pub fadeout_volume: u32,
    pub volume_envelope_state: PlaybackEnvelopeState,
    pub panning_envelope_state: PlaybackEnvelopeState,
    pub volume_envelope_val: u16,
    pub panning_envelope_val: u16,
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
            sample_frame_fraction: 0,
            volume: PLAYBACK_EMPTY_VOLUME,
            panning: SAMPLE_DEFAULT_PANNING,
            active_effects: vec![EffectCommand::default(); DEFAULT_EFFECT_SLOTS as usize],
            volume_slide_memory: 0,
            fine_volume_slide_memory: 0,
            period: 0,
            base_period: 0,
            target_period: 0,
            portamento_speed: 0,
            portamento_up_speed: 0,
            portamento_down_speed: 0,
            tone_portamento_speed: 0,
            arpeggio_memory: 0,
            vibrato_speed: [0; 2],
            vibrato_depth: [0; 2],
            vibrato_pos: [0; 2],
            sample_offset_memory: 0,
            sample_backward: false,
            keyon: true,
            fadeout_volume: 65536,
            volume_envelope_state: PlaybackEnvelopeState::new(),
            panning_envelope_state: PlaybackEnvelopeState::new(),
            volume_envelope_val: 256,
            panning_envelope_val: 128,
        }
    }

    fn apply_cell(&mut self, module: &Module, cell: &PatternCell) -> PlaybackResult<()> {
        self.active_effects = cell.effects.clone();
        if cell.instrument != DEFAULT_INSTRUMENT_NUMBER {
            self.set_instrument(module, cell.instrument)?;
        }

        let mut offset_to_apply = None;
        for effect in &cell.effects {
            if effect.effect == EFFECT_SAMPLE_OFFSET {
                let mut op = effect.operand;
                if op == 0 {
                    op = self.sample_offset_memory;
                } else {
                    self.sample_offset_memory = op;
                }
                offset_to_apply = Some(usize::from(op) * 256);
            }
        }

        let tone_porta = cell
            .effects
            .iter()
            .any(|eff| eff.effect == EFFECT_TONE_PORTAMENTO);

        match cell.note {
            Note::Empty => Ok(()),
            Note::Off => {
                if !tone_porta {
                    let volume_envelope_enabled = if let Some(ins_idx) = self.instrument_index {
                        if ins_idx < module.instruments.len() {
                            let ins = &module.instruments[ins_idx];
                            (ins.volume_envelope.flags & 0x01) != 0
                                && !ins.volume_envelope.points.is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if volume_envelope_enabled {
                        self.keyon = false;
                        self.note = Note::Off;
                    } else {
                        self.release();
                    }
                }
                Ok(())
            }
            Note::Key(note) => {
                if tone_porta {
                    if let Some(target) = self.calculate_note_period(module, note)? {
                        self.target_period = target;
                    }
                    Ok(())
                } else {
                    self.trigger_key(module, note, offset_to_apply)
                }
            }
        }
    }

    fn calculate_note_period(&self, module: &Module, note: u8) -> PlaybackResult<Option<u32>> {
        let Some(instrument_index) = self.instrument_index else {
            return Ok(None);
        };
        let Some(note_index) = note_sample_map_index(note) else {
            return Ok(None);
        };
        let Some(sample_index) = module.instruments[instrument_index]
            .note_sample_map
            .get(note_index)
            .and_then(|sample_index| *sample_index)
        else {
            return Ok(None);
        };
        let Some(sample) = module.samples.get(sample_index) else {
            return Err(PlaybackError::MissingSample {
                channel: self.channel,
                instrument_index,
                sample_index,
            });
        };

        let note_val = note as i32 + i32::from(sample.relative_note);
        let period = 7680 - ((note_val - 1) << 6) - (i32::from(sample.finetune) / 2);
        let period = period.max(0) as u32;
        Ok(Some(period))
    }

    fn calc_vibrato(&self, slot: usize) -> i32 {
        let vp = self.vibrato_pos[slot] as usize;
        let vd = self.vibrato_depth[slot] as i32;
        let tab_val = VIB_TAB[vp & 31];
        let mut vm = (tab_val * vd) >> (7 - 2);
        if (vp & 63) > 31 {
            vm = -vm;
        }
        vm
    }

    fn process_tick_effects(&mut self, module: &Module, tick: u16) {
        let mut active_arpeggio_op = None;
        let mut active_vibrato_slot = None;

        for (slot_idx, effect) in self.active_effects.iter().enumerate() {
            match effect.effect {
                EFFECT_VOLUME => {
                    if tick == 0 {
                        self.volume = effect.operand;
                    }
                }
                EFFECT_PANNING => {
                    if tick == 0 {
                        self.panning = effect.operand;
                    }
                }
                EFFECT_FINE_VOLUME_SLIDE_UP => {
                    if tick == 0 {
                        let mut op = effect.operand;
                        if op == 0 {
                            op = self.fine_volume_slide_memory;
                        } else {
                            self.fine_volume_slide_memory = op;
                        }
                        self.volume = self.volume.saturating_add(op.saturating_mul(4));
                    }
                }
                EFFECT_FINE_VOLUME_SLIDE_DOWN => {
                    if tick == 0 {
                        let mut op = effect.operand;
                        if op == 0 {
                            op = self.fine_volume_slide_memory;
                        } else {
                            self.fine_volume_slide_memory = op;
                        }
                        self.volume = self.volume.saturating_sub(op.saturating_mul(4));
                    }
                }
                EFFECT_VOLUME_SLIDE => {
                    if tick > 0 {
                        let mut op = effect.operand;
                        if op == 0 {
                            op = self.volume_slide_memory;
                        } else {
                            self.volume_slide_memory = op;
                        }
                        let x = op >> 4;
                        let y = op & 0x0f;
                        if x > 0 {
                            self.volume = self.volume.saturating_add(x.saturating_mul(4));
                        } else if y > 0 {
                            self.volume = self.volume.saturating_sub(y.saturating_mul(4));
                        }
                    }
                }
                EFFECT_ARPEGGIO_NONZERO | EFFECT_ARPEGGIO_ZERO => {
                    let mut op = effect.operand;
                    if op == 0 {
                        op = self.arpeggio_memory;
                    } else {
                        self.arpeggio_memory = op;
                    }
                    active_arpeggio_op = Some(op);
                }
                EFFECT_PORTAMENTO_UP => {
                    let mut op = effect.operand;
                    if op == 0 {
                        op = self.portamento_up_speed;
                    } else {
                        self.portamento_up_speed = op;
                        self.portamento_speed = op;
                    }
                    if tick > 0 {
                        self.base_period = self.base_period.saturating_sub(u32::from(op) * 4);
                    }
                }
                EFFECT_PORTAMENTO_DOWN => {
                    let mut op = effect.operand;
                    if op == 0 {
                        op = self.portamento_down_speed;
                    } else {
                        self.portamento_down_speed = op;
                        self.portamento_speed = op;
                    }
                    if tick > 0 {
                        self.base_period = self.base_period.saturating_add(u32::from(op) * 4);
                    }
                }
                EFFECT_TONE_PORTAMENTO => {
                    let mut op = effect.operand;
                    if op == 0 {
                        op = self.tone_portamento_speed;
                    } else {
                        self.tone_portamento_speed = op;
                    }
                    if tick > 0 && self.target_period > 0 {
                        if self.base_period < self.target_period {
                            self.base_period = self.base_period.saturating_add(u32::from(op) * 4);
                            if self.base_period > self.target_period {
                                self.base_period = self.target_period;
                            }
                        } else if self.base_period > self.target_period {
                            self.base_period = self.base_period.saturating_sub(u32::from(op) * 4);
                            if self.base_period < self.target_period {
                                self.base_period = self.target_period;
                            }
                        }
                    }
                }
                EFFECT_VIBRATO => {
                    let x = effect.operand >> 4;
                    let y = effect.operand & 0x0f;
                    if x > 0 {
                        self.vibrato_speed[slot_idx] = x;
                    }
                    if y > 0 {
                        self.vibrato_depth[slot_idx] = y;
                    }
                    active_vibrato_slot = Some(slot_idx);
                }
                EFFECT_VIBRATO_VOLSLIDE => {
                    active_vibrato_slot = Some(slot_idx);
                    if tick > 0 {
                        let mut op = effect.operand;
                        if op == 0 {
                            op = self.volume_slide_memory;
                        } else {
                            self.volume_slide_memory = op;
                        }
                        let x = op >> 4;
                        let y = op & 0x0f;
                        if x > 0 {
                            self.volume = self.volume.saturating_add(x.saturating_mul(4));
                        } else if y > 0 {
                            self.volume = self.volume.saturating_sub(y.saturating_mul(4));
                        }
                    }
                }
                _ => {}
            }
        }

        let mut pitch_offset = 0;

        if let Some(slot) = active_vibrato_slot {
            let vm = self.calc_vibrato(slot);
            if tick > 0 {
                self.vibrato_pos[slot] =
                    self.vibrato_pos[slot].wrapping_add(self.vibrato_speed[slot]);
            }
            pitch_offset += vm;
        }

        if let Some(arpeg_op) = active_arpeggio_op {
            let x = arpeg_op >> 4;
            let y = arpeg_op & 0x0f;
            let offset = match tick % 3 {
                0 => 0,
                1 => x,
                2 => y,
                _ => unreachable!(),
            };
            pitch_offset -= (offset as i32) * 64;
        }

        self.period = (self.base_period as i32 + pitch_offset).max(0) as u32;

        // Advance envelopes and fadeout
        let mut volume_envelope_val = 256;
        let mut panning_envelope_val = 128;

        if self.active {
            if let Some(ins_idx) = self.instrument_index {
                if ins_idx < module.instruments.len() {
                    let ins = &module.instruments[ins_idx];

                    // 1. Get values first
                    volume_envelope_val = self
                        .volume_envelope_state
                        .get_value(&ins.volume_envelope, 256);
                    panning_envelope_val = self
                        .panning_envelope_state
                        .get_value(&ins.panning_envelope, 128);

                    // 2. Update fadeout
                    if !self.keyon {
                        self.fadeout_volume = self
                            .fadeout_volume
                            .saturating_sub(u32::from(ins.volume_fadeout));
                    }

                    // 3. Advance envelopes for next tick
                    self.volume_envelope_state
                        .advance(&ins.volume_envelope, self.keyon);
                    self.panning_envelope_state
                        .advance(&ins.panning_envelope, self.keyon);

                    // 4. Deactivate channel if volume envelope or fadeout reaches 0
                    if ((ins.volume_envelope.flags & 0x01) != 0 && volume_envelope_val == 0)
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

    fn trigger_key(
        &mut self,
        module: &Module,
        note: u8,
        start_offset: Option<usize>,
    ) -> PlaybackResult<()> {
        self.note = Note::Key(note);
        self.vibrato_pos = [0; 2];
        self.sample_backward = false;
        self.keyon = true;
        self.fadeout_volume = 65536;
        self.volume_envelope_state.reset();
        self.panning_envelope_state.reset();
        self.volume_envelope_val = 256;
        self.panning_envelope_val = 128;

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

        let note_val = note as i32 + i32::from(sample.relative_note);
        let period = 7680 - ((note_val - 1) << 6) - (i32::from(sample.finetune) / 2);
        let period = period.max(0) as u32;
        self.base_period = period;
        self.period = period;

        if let Some(offset) = start_offset {
            self.sample_frame = offset;
            self.sample_frame_fraction = 0;
            if self.sample_frame >= sample.data.frame_count() {
                self.stop_sample();
            }
        } else {
            self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
            self.sample_frame_fraction = 0;
        }

        Ok(())
    }

    fn release(&mut self) {
        self.active = false;
        self.note = Note::Off;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
        self.sample_frame_fraction = 0;
    }

    fn stop_sample(&mut self) {
        self.active = false;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
        self.sample_frame_fraction = 0;
    }

    fn step_sample(&mut self, module: &Module) -> PlaybackResult<Option<ChannelSampleFrame>> {
        if !self.active {
            return Ok(None);
        }

        let Some(sample_index) = self.sample_index else {
            self.stop_sample();
            return Ok(None);
        };
        let Some(instrument_index) = self.instrument_index else {
            return Err(PlaybackError::MissingInstrument {
                channel: self.channel,
                instrument: self.instrument,
            });
        };
        let Some(sample) = module.samples.get(sample_index) else {
            return Err(PlaybackError::MissingSample {
                channel: self.channel,
                instrument_index,
                sample_index,
            });
        };
        let Some(value) = sample_value_at_frame(&sample.data, self.sample_frame) else {
            self.stop_sample();
            return Ok(None);
        };

        let sample_frame = self.sample_frame;
        self.advance_sample_frame(sample);
        Ok(Some(ChannelSampleFrame {
            channel: self.channel,
            sample_index,
            sample_frame,
            value,
        }))
    }

    fn advance_sample_position(&mut self, sample: &Sample, step: f64) {
        let frame_count = sample.data.frame_count();
        if frame_count == 0 {
            self.stop_sample();
            return;
        }

        let current_pos = self.sample_frame as f64 + (self.sample_frame_fraction as f64 / u32::MAX as f64);
        
        let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
        if has_loop {
            let loop_start = sample.loop_start as f64;
            let loop_length = sample.loop_length as f64;
            let loop_end = loop_start + loop_length;

            match sample.loop_kind {
                SampleLoopKind::Forward => {
                    let mut next_pos = current_pos + step;
                    if next_pos >= loop_end {
                        let over = next_pos - loop_end;
                        let wraps = (over / loop_length).floor();
                        next_pos = loop_start + over - wraps * loop_length;
                    }
                    next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                    self.sample_frame = next_pos as usize;
                    self.sample_frame_fraction = ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                }
                SampleLoopKind::PingPong => {
                    if self.sample_backward {
                        let mut next_pos = current_pos - step;
                        if next_pos <= loop_start {
                            self.sample_backward = false;
                            let under = loop_start - next_pos;
                            let wraps = (under / loop_length).floor() as i32;
                            let rem = under - (wraps as f64) * loop_length;
                            if wraps % 2 == 0 {
                                next_pos = loop_start + rem;
                            } else {
                                self.sample_backward = true;
                                next_pos = loop_end - rem;
                            }
                        }
                        next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction = ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                    } else {
                        let mut next_pos = current_pos + step;
                        if next_pos >= loop_end {
                            self.sample_backward = true;
                            let over = next_pos - loop_end;
                            let wraps = (over / loop_length).floor() as i32;
                            let rem = over - (wraps as f64) * loop_length;
                            if wraps % 2 == 0 {
                                next_pos = loop_end - rem;
                            } else {
                                self.sample_backward = false;
                                next_pos = loop_start + rem;
                            }
                        }
                        next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction = ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                    }
                }
                SampleLoopKind::None => unreachable!(),
            }
        } else {
            let next_pos = current_pos + step;
            if next_pos >= frame_count as f64 {
                self.stop_sample();
            } else {
                self.sample_frame = next_pos as usize;
                self.sample_frame_fraction = ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
            }
        }
    }

    fn advance_sample_frame(&mut self, sample: &Sample) {
        let frame_count = sample.data.frame_count();
        if frame_count == 0 {
            self.stop_sample();
            return;
        }

        let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;

        if has_loop {
            let loop_start = sample.loop_start as usize;
            let loop_length = sample.loop_length as usize;
            let loop_end = loop_start + loop_length;

            match sample.loop_kind {
                SampleLoopKind::Forward => {
                    let next_frame = self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
                    if next_frame >= loop_end {
                        self.sample_frame = loop_start + (next_frame - loop_end) % loop_length;
                    } else {
                        self.sample_frame = next_frame;
                    }
                }
                SampleLoopKind::PingPong => {
                    if self.sample_backward {
                        if self.sample_frame <= loop_start {
                            self.sample_backward = false;
                            self.sample_frame = (loop_start + 1).min(loop_end - 1);
                        } else {
                            self.sample_frame =
                                self.sample_frame.saturating_sub(PLAYBACK_SAMPLE_FRAME_STEP);
                        }
                    } else {
                        let next_frame =
                            self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
                        if next_frame >= loop_end {
                            self.sample_backward = true;
                            self.sample_frame =
                                (loop_end as i32 - 2).max(loop_start as i32) as usize;
                        } else {
                            self.sample_frame = next_frame;
                        }
                    }
                }
                SampleLoopKind::None => unreachable!(),
            }
        } else {
            let next_frame = self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
            if next_frame >= frame_count {
                self.stop_sample();
            } else {
                self.sample_frame = next_frame;
            }
        }
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

    pub fn set_bpm(&mut self, bpm: u16) -> PlaybackResult<()> {
        if bpm < PLAYBACK_MIN_BPM {
            return Err(PlaybackError::InvalidBpm { bpm });
        }
        self.bpm = bpm;
        self.tick_duration_nanos = PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM / u64::from(bpm);
        Ok(())
    }

    pub fn set_tick_speed(&mut self, tick_speed: u16) -> PlaybackResult<()> {
        if tick_speed < PLAYBACK_MIN_TICK_SPEED {
            return Err(PlaybackError::InvalidTickSpeed { tick_speed });
        }
        self.tick_speed = tick_speed;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackCursor {
    order_index: usize,
    row: u16,
    jump_target: Option<PlaybackPosition>,
}

impl PlaybackCursor {
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
                0
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackState {
    clock: PlaybackClock,
    channels: Vec<PlaybackChannelState>,
    tick_samples_fractional_rem: i64,
    song_ended: bool,
    initialized: bool,
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
        let mut state = Self {
            clock,
            channels,
            tick_samples_fractional_rem: 0,
            song_ended: false,
            initialized: false,
        };
        state.apply_row_state(module, &row_state)?;
        Ok(state)
    }

    pub fn clock(&self) -> PlaybackClock {
        self.clock
    }

    pub fn channels(&self) -> &[PlaybackChannelState] {
        &self.channels
    }

    pub fn song_ended(&self) -> bool {
        self.song_ended
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        self.clock.row_state(module)
    }

    pub fn advance_tick(&mut self, module: &Module) -> PlaybackResult<TickAdvance> {
        if self.song_ended {
            return Ok(TickAdvance::SongEnd);
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
        Ok(advance)
    }

    pub fn step_samples(&mut self, module: &Module) -> PlaybackResult<Vec<ChannelSampleFrame>> {
        let mut frames = Vec::new();
        for channel in &mut self.channels {
            if let Some(frame) = channel.step_sample(module)? {
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    pub fn render_raw_mono_pcm(
        &mut self,
        module: &Module,
        sample_rate: u32,
        frame_count: usize,
    ) -> PlaybackResult<Vec<RawMonoPcmFrame>> {
        let mut rendered = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            rendered.push(self.render_raw_mono_frame(module, sample_rate)?);
        }

        Ok(rendered)
    }

    pub fn render_raw_stereo_pcm(
        &mut self,
        module: &Module,
        sample_rate: u32,
        frame_count: usize,
    ) -> PlaybackResult<Vec<RawStereoPcmFrame>> {
        let mut rendered = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            rendered.push(self.render_raw_stereo_frame(module, sample_rate)?);
        }

        Ok(rendered)
    }

    fn render_raw_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawStereoPcmFrame> {
        let mut current_bpm = self.clock.timing().bpm() as i64;

        if !self.initialized {
            self.tick_samples_fractional_rem = 5 * sample_rate as i64;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.song_ended {
                return Ok((0, 0));
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    self.song_ended = true;
                    return Ok((0, 0));
                }
                _ => {
                    let new_bpm = self.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = 2 * current_bpm;
                        let new_denom = 2 * new_bpm;
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                    }
                    self.tick_samples_fractional_rem += 5 * sample_rate as i64;
                }
            }
        }

        let mut mixed_l = 0.0;
        let mut mixed_r = 0.0;
        for channel in &mut self.channels {
            if channel.active {
                if let Some(sample_index) = channel.sample_index {
                    if let Some(sample) = module.samples.get(sample_index) {
                        let frame_count = sample.data.frame_count();
                        if frame_count > 0 {
                            let frequency = period_to_frequency(channel.period, module.header.frequency_table);
                            let step = frequency / sample_rate as f64;

                            let interpolated_val = get_sample_value_linear(
                                &sample.data,
                                channel.sample_frame,
                                channel.sample_frame_fraction,
                                sample,
                            );

                            let vol_factor = (channel.volume as f64 / 255.0)
                                * (channel.volume_envelope_val as f64 / 256.0)
                                * (channel.fadeout_volume as f64 / 65536.0);

                            let channel_mono_pcm = interpolated_val * vol_factor;

                            let mut pan = channel.panning as i32 + channel.panning_envelope_val as i32 - 128;
                            pan = pan.clamp(0, 255);

                            let right_gain = pan as f64 / 255.0;
                            let left_gain = 1.0 - right_gain;

                            mixed_l += channel_mono_pcm * left_gain;
                            mixed_r += channel_mono_pcm * right_gain;

                            channel.advance_sample_position(sample, step);
                        }
                    }
                }
            }
        }

        self.tick_samples_fractional_rem -= 2 * current_bpm;
        Ok((mixed_l as i32, mixed_r as i32))
    }

    fn render_raw_mono_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawMonoPcmFrame> {
        let mut current_bpm = self.clock.timing().bpm() as i64;

        if !self.initialized {
            self.tick_samples_fractional_rem = 5 * sample_rate as i64;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.song_ended {
                return Ok(PLAYBACK_MONO_SILENCE);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    self.song_ended = true;
                    return Ok(PLAYBACK_MONO_SILENCE);
                }
                _ => {
                    let new_bpm = self.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = 2 * current_bpm;
                        let new_denom = 2 * new_bpm;
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                    }
                    self.tick_samples_fractional_rem += 5 * sample_rate as i64;
                }
            }
        }

        let mut mixed = PLAYBACK_MONO_SILENCE;
        for channel in &mut self.channels {
            if channel.active {
                if let Some(sample_index) = channel.sample_index {
                    if let Some(sample) = module.samples.get(sample_index) {
                        let frame_count = sample.data.frame_count();
                        if frame_count > 0 {
                            let frequency = period_to_frequency(channel.period, module.header.frequency_table);
                            let step = frequency / sample_rate as f64;

                            let interpolated_val = get_sample_value_linear(
                                &sample.data,
                                channel.sample_frame,
                                channel.sample_frame_fraction,
                                sample,
                            );

                            let vol_factor = (channel.volume as f64 / 255.0)
                                * (channel.volume_envelope_val as f64 / 256.0)
                                * (channel.fadeout_volume as f64 / 65536.0);

                            let channel_mono_pcm = interpolated_val * vol_factor;
                            mixed += channel_mono_pcm as i32;

                            channel.advance_sample_position(sample, step);
                        }
                    }
                }
            }
        }

        self.tick_samples_fractional_rem -= 2 * current_bpm;
        Ok(mixed)
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
                pattern_index: 0,
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

fn sample_value_at_frame(data: &SampleData, frame: usize) -> Option<PlaybackSampleValue> {
    match data {
        SampleData::Empty => None,
        SampleData::Pcm8(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm8),
        SampleData::Pcm16(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm16),
    }
}

fn sample_value_as_f64(data: &SampleData, index: usize) -> f64 {
    match data {
        SampleData::Empty => 0.0,
        SampleData::Pcm8(values) => {
            if let Some(&val) = values.get(index) {
                ((val as i32) << PLAYBACK_PCM8_TO_I16_SHIFT) as f64
            } else {
                0.0
            }
        }
        SampleData::Pcm16(values) => {
            if let Some(&val) = values.get(index) {
                val as f64
            } else {
                0.0
            }
        }
    }
}

fn next_frame_index(frame: usize, sample: &Sample) -> Option<usize> {
    let frame_count = sample.data.frame_count();
    if frame_count == 0 {
        return None;
    }
    let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
    if has_loop {
        let loop_start = sample.loop_start as usize;
        let loop_length = sample.loop_length as usize;
        let loop_end = loop_start + loop_length;
        
        let next = frame + 1;
        if next >= loop_end {
            Some(loop_start + (next - loop_end) % loop_length)
        } else {
            Some(next)
        }
    } else {
        let next = frame + 1;
        if next >= frame_count {
            None
        } else {
            Some(next)
        }
    }
}

fn relative_frame_index(frame: usize, offset: i32, sample: &Sample) -> Option<usize> {
    let frame_count = sample.data.frame_count();
    if frame_count == 0 {
        return None;
    }
    let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
    if has_loop {
        let loop_start = sample.loop_start as i32;
        let loop_length = sample.loop_length as i32;
        let loop_end = loop_start + loop_length;
        
        let mut target = frame as i32 + offset;
        if target < loop_start {
            let diff = loop_start - target;
            target = loop_end - 1 - (diff - 1) % loop_length;
        } else if target >= loop_end {
            let diff = target - loop_end;
            target = loop_start + diff % loop_length;
        }
        Some(target as usize)
    } else {
        let target = frame as i32 + offset;
        if target < 0 || target >= frame_count as i32 {
            None
        } else {
            Some(target as usize)
        }
    }
}

fn get_sample_value_linear(data: &SampleData, frame: usize, fraction: u32, sample: &Sample) -> f64 {
    let t = fraction as f64 / u32::MAX as f64;
    let y0 = sample_value_as_f64(data, frame);
    let y1 = if let Some(next_idx) = next_frame_index(frame, sample) {
        sample_value_as_f64(data, next_idx)
    } else {
        0.0
    };
    y0 + t * (y1 - y0)
}

#[allow(dead_code)]
fn get_sample_value_cubic(data: &SampleData, frame: usize, fraction: u32, sample: &Sample) -> f64 {
    let t = fraction as f64 / u32::MAX as f64;
    
    let y0 = if let Some(idx) = relative_frame_index(frame, -1, sample) {
        sample_value_as_f64(data, idx)
    } else {
        0.0
    };
    let y1 = sample_value_as_f64(data, frame);
    let y2 = if let Some(idx) = relative_frame_index(frame, 1, sample) {
        sample_value_as_f64(data, idx)
    } else {
        0.0
    };
    let y3 = if let Some(idx) = relative_frame_index(frame, 2, sample) {
        sample_value_as_f64(data, idx)
    } else {
        0.0
    };
    
    let a = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
    let b = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c = -0.5 * y0 + 0.5 * y2;
    let d = y1;
    
    ((a * t + b) * t + c) * t + d
}

fn period_to_frequency(period: u32, table: FrequencyTable) -> f64 {
    if period == 0 {
        return 0.0;
    }
    match table {
        FrequencyTable::Linear => {
            8363.0 * f64::powf(2.0, (4608.0 - period as f64) / 768.0)
        }
        FrequencyTable::Amiga => {
            (8363.0 * 428.0) / period as f64
        }
    }
}
