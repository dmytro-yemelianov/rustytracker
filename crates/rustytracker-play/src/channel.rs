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
    EffectCommand, FrequencyTable, Module, Note, PatternCell, Sample, DEFAULT_EFFECT_SLOTS,
    DEFAULT_INSTRUMENT_NUMBER, FIRST_XM_NOTE_VALUE, SAMPLE_DEFAULT_PANNING,
};

pub const PLAYBACK_INSTRUMENT_NUMBER_BASE: u8 = 1;
pub const PLAYBACK_SAMPLE_START_FRAME: usize = 0;
pub const PLAYBACK_SAMPLE_FRAME_STEP: usize = 1;
pub const PLAYBACK_EMPTY_VOLUME: u8 = 0;
pub const PLAYBACK_PCM8_TO_I16_SHIFT: u32 = 8;
const XM_LINEAR_PERIOD_BASE: i32 = 7680;
const XM_LINEAR_PERIOD_NOTE_SHIFT: u32 = 6;
const XM_LINEAR_FINETUNE_DIVISOR: i32 = 2;
const AMIGA_NOTE_MIN: i32 = 1;
const AMIGA_NOTE_MAX: i32 = 120;
const AMIGA_FINETUNE_CENTER: i32 = 128;
const AMIGA_FINETUNE_MIN: i32 = 0;
const AMIGA_FINETUNE_MAX: i32 = 255;
const AMIGA_FINETUNE_TABLE_SHIFT: u32 = 4;
const AMIGA_LOG_PERIOD_INTERP_CENTER: i32 = 8;
const AMIGA_LOG_PERIOD_INTERP_DENOMINATOR: i32 = 15;
const AMIGA_PANNING_LEFT: u8 = 0;
const AMIGA_PANNING_RIGHT: u8 = 255;
const AMIGA_LOG_PERIOD_TABLE: [i32; 105] = [
    907 * 32,
    900 * 32,
    894 * 32,
    887 * 32,
    881 * 32,
    875 * 32,
    868 * 32,
    862 * 32,
    856 * 32,
    850 * 32,
    844 * 32,
    838 * 32,
    832 * 32,
    826 * 32,
    820 * 32,
    814 * 32,
    808 * 32,
    802 * 32,
    796 * 32,
    791 * 32,
    785 * 32,
    779 * 32,
    774 * 32,
    768 * 32,
    762 * 32,
    757 * 32,
    752 * 32,
    746 * 32,
    741 * 32,
    736 * 32,
    730 * 32,
    725 * 32,
    720 * 32,
    715 * 32,
    709 * 32,
    704 * 32,
    699 * 32,
    694 * 32,
    689 * 32,
    684 * 32,
    678 * 32,
    675 * 32,
    670 * 32,
    665 * 32,
    660 * 32,
    655 * 32,
    651 * 32,
    646 * 32,
    640 * 32,
    636 * 32,
    632 * 32,
    628 * 32,
    623 * 32,
    619 * 32,
    614 * 32,
    610 * 32,
    604 * 32,
    601 * 32,
    597 * 32,
    592 * 32,
    588 * 32,
    584 * 32,
    580 * 32,
    575 * 32,
    570 * 32,
    567 * 32,
    563 * 32,
    559 * 32,
    555 * 32,
    551 * 32,
    547 * 32,
    543 * 32,
    538 * 32,
    535 * 32,
    532 * 32,
    528 * 32,
    524 * 32,
    520 * 32,
    516 * 32,
    513 * 32,
    508 * 32,
    505 * 32,
    502 * 32,
    498 * 32,
    494 * 32,
    491 * 32,
    487 * 32,
    484 * 32,
    480 * 32,
    477 * 32,
    474 * 32,
    470 * 32,
    467 * 32,
    463 * 32,
    460 * 32,
    457 * 32,
    453 * 32,
    450 * 32,
    447 * 32,
    443 * 32,
    440 * 32,
    437 * 32,
    434 * 32,
    431 * 32,
    428 * 32,
];
const PLAYBACK_EMPTY_SAMPLE_FRACTION: u32 = 0;
const PLAYBACK_EMPTY_SAMPLE_FRAME: usize = PLAYBACK_SAMPLE_START_FRAME;

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
    pub triggered: Option<Option<usize>>,
    pub stopped: bool,
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
            triggered: None,
            stopped: false,
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

        Ok(Some(sample_period_for_note(
            note,
            sample,
            module.header.frequency_table,
        )))
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
        self.triggered = Some(start_offset);
        self.stopped = false;

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
        self.panning =
            panning_for_triggered_sample(module.header.frequency_table, self.channel, sample);

        let period = sample_period_for_note(note, sample, module.header.frequency_table);
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
        self.stopped = true;
    }

    pub(crate) fn stop_sample(&mut self) {
        self.active = false;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_EMPTY_SAMPLE_FRAME;
        self.stopped = true;
        self.sample_frame_fraction = PLAYBACK_EMPTY_SAMPLE_FRACTION;
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

fn sample_period_for_note(note: u8, sample: &Sample, table: FrequencyTable) -> u32 {
    match table {
        FrequencyTable::Linear => linear_period_for_note(note, sample),
        FrequencyTable::Amiga => amiga_log_period_for_note(note, sample),
    }
}

fn linear_period_for_note(note: u8, sample: &Sample) -> u32 {
    let note_val = i32::from(note) + i32::from(sample.relative_note);
    let period = XM_LINEAR_PERIOD_BASE
        - ((note_val - 1) << XM_LINEAR_PERIOD_NOTE_SHIFT)
        - (i32::from(sample.finetune) / XM_LINEAR_FINETUNE_DIVISOR);
    period.max(0) as u32
}

fn panning_for_triggered_sample(table: FrequencyTable, channel: u16, sample: &Sample) -> u8 {
    match table {
        FrequencyTable::Linear => sample.panning,
        FrequencyTable::Amiga => match channel & 3 {
            0 | 3 => AMIGA_PANNING_LEFT,
            1 | 2 => AMIGA_PANNING_RIGHT,
            _ => unreachable!(),
        },
    }
}

fn amiga_log_period_for_note(note: u8, sample: &Sample) -> u32 {
    let note_val =
        (i32::from(note) + i32::from(sample.relative_note)).clamp(AMIGA_NOTE_MIN, AMIGA_NOTE_MAX);
    let finetune = (i32::from(sample.finetune) + AMIGA_FINETUNE_CENTER)
        .clamp(AMIGA_FINETUNE_MIN, AMIGA_FINETUNE_MAX);
    let octave = (note_val - 1) / 12;
    let semitone = ((note_val - 1) % 12) << 3;
    let table_index = ((finetune >> AMIGA_FINETUNE_TABLE_SHIFT) + semitone) as usize;
    let v1 = AMIGA_LOG_PERIOD_TABLE[table_index];
    let v2 = AMIGA_LOG_PERIOD_TABLE[table_index + 1];
    let t = (finetune >> AMIGA_FINETUNE_TABLE_SHIFT) - AMIGA_LOG_PERIOD_INTERP_CENTER;
    let interpolated = v1 + (t * (v2 - v1)) / AMIGA_LOG_PERIOD_INTERP_DENOMINATOR;

    (interpolated >> octave).max(1) as u32
}
