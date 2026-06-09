use crate::effects::{
    EFFECT_MEMORY_REUSE_OPERAND, EFFECT_SAMPLE_OFFSET, EFFECT_SAMPLE_OFFSET_FRAME_SCALE,
    EFFECT_TONE_PORTAMENTO, PLAYBACK_EMPTY_PERIOD,
};
use crate::envelope::{
    PlaybackEnvelopeState, PLAYBACK_DEFAULT_FADEOUT_VOLUME, PLAYBACK_ENVELOPE_DEFAULT_PANNING,
    PLAYBACK_ENVELOPE_DEFAULT_VOLUME, XM_ENVELOPE_ENABLED_FLAG,
};
use crate::error::{PlaybackError, PlaybackResult};
use crate::RawMonoPcmFrame;
use rustytracker_core::{
    EffectCommand, Module, Note, PatternCell, Sample, SampleData, SampleLoopKind,
    DEFAULT_EFFECT_SLOTS, DEFAULT_INSTRUMENT_NUMBER, FIRST_XM_NOTE_VALUE, SAMPLE_DEFAULT_PANNING,
};

pub const PLAYBACK_INSTRUMENT_NUMBER_BASE: u8 = 1;
pub const PLAYBACK_SAMPLE_START_FRAME: usize = 0;
pub const PLAYBACK_SAMPLE_FRAME_STEP: usize = 1;
pub const PLAYBACK_EMPTY_VOLUME: u8 = 0;
pub const PLAYBACK_PCM8_TO_I16_SHIFT: u32 = 8;
const XM_LINEAR_PERIOD_BASE: i32 = 7680;
const XM_LINEAR_PERIOD_NOTE_SHIFT: u32 = 6;
const XM_LINEAR_FINETUNE_DIVISOR: i32 = 2;
const PLAYBACK_EMPTY_SAMPLE_FRACTION: u32 = 0;
const PLAYBACK_EMPTY_SAMPLE_FRAME: usize = PLAYBACK_SAMPLE_START_FRAME;
const SAMPLE_LOOP_END_EPSILON: f64 = 0.000001;
const PING_PONG_LOOP_REFLECT_BACKSTEP: i32 = 2;

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
    pub vibrato_speed: Vec<u8>,
    pub vibrato_depth: Vec<u8>,
    pub vibrato_pos: Vec<u8>,
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
    pub(crate) fn empty(channel: u16) -> Self {
        Self {
            channel,
            active: false,
            note: Note::Empty,
            instrument: DEFAULT_INSTRUMENT_NUMBER,
            instrument_index: None,
            sample_index: None,
            sample_frame: PLAYBACK_EMPTY_SAMPLE_FRAME,
            sample_frame_fraction: PLAYBACK_EMPTY_SAMPLE_FRACTION,
            volume: PLAYBACK_EMPTY_VOLUME,
            panning: SAMPLE_DEFAULT_PANNING,
            active_effects: vec![EffectCommand::default(); DEFAULT_EFFECT_SLOTS as usize],
            volume_slide_memory: EFFECT_MEMORY_REUSE_OPERAND,
            fine_volume_slide_memory: EFFECT_MEMORY_REUSE_OPERAND,
            period: PLAYBACK_EMPTY_PERIOD,
            base_period: PLAYBACK_EMPTY_PERIOD,
            target_period: PLAYBACK_EMPTY_PERIOD,
            portamento_speed: EFFECT_MEMORY_REUSE_OPERAND,
            portamento_up_speed: EFFECT_MEMORY_REUSE_OPERAND,
            portamento_down_speed: EFFECT_MEMORY_REUSE_OPERAND,
            tone_portamento_speed: EFFECT_MEMORY_REUSE_OPERAND,
            arpeggio_memory: EFFECT_MEMORY_REUSE_OPERAND,
            vibrato_speed: vec![EFFECT_MEMORY_REUSE_OPERAND; DEFAULT_EFFECT_SLOTS as usize],
            vibrato_depth: vec![EFFECT_MEMORY_REUSE_OPERAND; DEFAULT_EFFECT_SLOTS as usize],
            vibrato_pos: vec![EFFECT_MEMORY_REUSE_OPERAND; DEFAULT_EFFECT_SLOTS as usize],
            sample_offset_memory: EFFECT_MEMORY_REUSE_OPERAND,
            sample_backward: false,
            keyon: true,
            fadeout_volume: PLAYBACK_DEFAULT_FADEOUT_VOLUME,
            volume_envelope_state: PlaybackEnvelopeState::new(),
            panning_envelope_state: PlaybackEnvelopeState::new(),
            volume_envelope_val: PLAYBACK_ENVELOPE_DEFAULT_VOLUME,
            panning_envelope_val: PLAYBACK_ENVELOPE_DEFAULT_PANNING,
        }
    }

    pub(crate) fn apply_cell(&mut self, module: &Module, cell: &PatternCell) -> PlaybackResult<()> {
        self.active_effects = cell.effects.clone();
        self.ensure_effect_memory_slots();
        if cell.instrument != DEFAULT_INSTRUMENT_NUMBER && cell.note != Note::Off {
            self.set_instrument(module, cell.instrument)?;
        }

        let mut offset_to_apply = None;
        for effect in &cell.effects {
            if effect.effect == EFFECT_SAMPLE_OFFSET {
                let mut op = effect.operand;
                if op == EFFECT_MEMORY_REUSE_OPERAND {
                    op = self.sample_offset_memory;
                } else {
                    self.sample_offset_memory = op;
                }
                offset_to_apply = Some(usize::from(op) * EFFECT_SAMPLE_OFFSET_FRAME_SCALE);
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
                            (ins.volume_envelope.flags & XM_ENVELOPE_ENABLED_FLAG) != 0
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

        Ok(Some(sample_period_for_note(note, sample)))
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
        self.vibrato_pos.fill(EFFECT_MEMORY_REUSE_OPERAND);
        self.sample_backward = false;
        self.keyon = true;
        self.fadeout_volume = PLAYBACK_DEFAULT_FADEOUT_VOLUME;
        self.volume_envelope_state.reset();
        self.panning_envelope_state.reset();
        self.volume_envelope_val = PLAYBACK_ENVELOPE_DEFAULT_VOLUME;
        self.panning_envelope_val = PLAYBACK_ENVELOPE_DEFAULT_PANNING;

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

        let period = sample_period_for_note(note, sample);
        self.base_period = period;
        self.period = period;

        if let Some(offset) = start_offset {
            self.sample_frame = offset;
            self.sample_frame_fraction = PLAYBACK_EMPTY_SAMPLE_FRACTION;
            if self.sample_frame >= sample.data.frame_count() {
                self.stop_sample();
            }
        } else {
            self.sample_frame = PLAYBACK_EMPTY_SAMPLE_FRAME;
            self.sample_frame_fraction = PLAYBACK_EMPTY_SAMPLE_FRACTION;
        }

        Ok(())
    }

    fn release(&mut self) {
        self.active = false;
        self.note = Note::Off;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_EMPTY_SAMPLE_FRAME;
        self.sample_frame_fraction = PLAYBACK_EMPTY_SAMPLE_FRACTION;
    }

    pub(crate) fn stop_sample(&mut self) {
        self.active = false;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_EMPTY_SAMPLE_FRAME;
        self.sample_frame_fraction = PLAYBACK_EMPTY_SAMPLE_FRACTION;
    }

    pub(crate) fn step_sample(
        &mut self,
        module: &Module,
    ) -> PlaybackResult<Option<ChannelSampleFrame>> {
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

    pub(crate) fn advance_sample_position(&mut self, sample: &Sample, step: f64) {
        let frame_count = sample.data.frame_count();
        if frame_count == 0 {
            self.stop_sample();
            return;
        }

        let current_pos =
            self.sample_frame as f64 + (self.sample_frame_fraction as f64 / u32::MAX as f64);

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
                    next_pos = next_pos.clamp(loop_start, loop_end - SAMPLE_LOOP_END_EPSILON);
                    self.sample_frame = next_pos as usize;
                    self.sample_frame_fraction =
                        ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
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
                        next_pos = next_pos.clamp(loop_start, loop_end - SAMPLE_LOOP_END_EPSILON);
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction =
                            ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
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
                        next_pos = next_pos.clamp(loop_start, loop_end - SAMPLE_LOOP_END_EPSILON);
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction =
                            ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
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
                self.sample_frame_fraction =
                    ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
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
                            self.sample_frame =
                                (loop_start + PLAYBACK_SAMPLE_FRAME_STEP).min(loop_end - 1);
                        } else {
                            self.sample_frame =
                                self.sample_frame.saturating_sub(PLAYBACK_SAMPLE_FRAME_STEP);
                        }
                    } else {
                        let next_frame =
                            self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
                        if next_frame >= loop_end {
                            self.sample_backward = true;
                            self.sample_frame = (loop_end as i32 - PING_PONG_LOOP_REFLECT_BACKSTEP)
                                .max(loop_start as i32)
                                as usize;
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

fn instrument_index_for_number(instrument: u8) -> Option<usize> {
    instrument
        .checked_sub(PLAYBACK_INSTRUMENT_NUMBER_BASE)
        .map(usize::from)
}

fn note_sample_map_index(note: u8) -> Option<usize> {
    note.checked_sub(FIRST_XM_NOTE_VALUE).map(usize::from)
}

fn sample_period_for_note(note: u8, sample: &Sample) -> u32 {
    let note_val = i32::from(note) + i32::from(sample.relative_note);
    let period = XM_LINEAR_PERIOD_BASE
        - ((note_val - 1) << XM_LINEAR_PERIOD_NOTE_SHIFT)
        - (i32::from(sample.finetune) / XM_LINEAR_FINETUNE_DIVISOR);
    period.max(0) as u32
}

fn sample_value_at_frame(data: &SampleData, frame: usize) -> Option<PlaybackSampleValue> {
    match data {
        SampleData::Empty => None,
        SampleData::Pcm8(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm8),
        SampleData::Pcm16(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm16),
    }
}
