use rustytracker_core::{FrequencyTable, SampleLoopKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmModuleHeader {
    pub title: String,
    pub tracker_name: String,
    pub version: u16,
    pub header_size: u32,
    pub song_length: u16,
    pub restart_position: u16,
    pub channel_count: u16,
    pub pattern_count: u16,
    pub instrument_count: u16,
    pub flags: u16,
    pub frequency_table: FrequencyTable,
    pub default_tick_speed: u16,
    pub default_bpm: u16,
    pub orders: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmPatternHeader {
    pub index: usize,
    pub header_length: u32,
    pub packing_type: u8,
    pub row_count: u16,
    pub packed_data_len: u16,
    pub packed_data_offset: usize,
    pub next_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmInstrumentSection {
    pub instruments: Vec<XmInstrument>,
    pub next_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmInstrument {
    pub index: usize,
    pub header_size: u32,
    pub name: String,
    pub instrument_type: u8,
    pub sample_count: u16,
    pub sample_header_size: Option<u32>,
    pub note_sample_map: Option<Vec<u8>>,
    pub volume_envelope: Option<XmEnvelope>,
    pub panning_envelope: Option<XmEnvelope>,
    pub vibrato_type: Option<u8>,
    pub vibrato_sweep: Option<u8>,
    pub vibrato_depth: Option<u8>,
    pub vibrato_rate: Option<u8>,
    pub volume_fadeout: Option<u16>,
    pub samples: Vec<XmSampleHeader>,
    pub next_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmEnvelope {
    pub points: Vec<XmEnvelopePoint>,
    pub point_count: u8,
    pub sustain_point: u8,
    pub loop_start_point: u8,
    pub loop_end_point: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XmEnvelopePoint {
    pub frame: u16,
    pub value: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmSampleHeader {
    pub index: usize,
    pub length: u32,
    pub frame_count: u32,
    pub loop_start: u32,
    pub loop_start_frames: u32,
    pub loop_length: u32,
    pub loop_length_frames: u32,
    pub volume_64: u8,
    pub volume: u8,
    pub finetune: i8,
    pub sample_type: u8,
    pub loop_kind: SampleLoopKind,
    pub panning: u8,
    pub relative_note: i8,
    pub reserved: u8,
    pub name: String,
    pub data_offset: usize,
    pub data_end: usize,
    pub decoded_data: XmSampleData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmSampleData {
    Pcm8(Vec<i8>),
    Pcm16(Vec<i16>),
}

impl XmSampleData {
    pub fn frame_count(&self) -> usize {
        match self {
            Self::Pcm8(values) => values.len(),
            Self::Pcm16(values) => values.len(),
        }
    }

    pub fn as_i8(&self) -> Option<&[i8]> {
        match self {
            Self::Pcm8(values) => Some(values),
            Self::Pcm16(_) => None,
        }
    }

    pub fn as_i16(&self) -> Option<&[i16]> {
        match self {
            Self::Pcm8(_) => None,
            Self::Pcm16(values) => Some(values),
        }
    }
}
