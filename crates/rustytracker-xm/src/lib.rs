//! XM file support for RustyTracker.
//!
//! This crate starts read-only. It will grow toward a full parser/writer only
//! through fixture-backed tests.

use rustytracker_core::{
    EffectCommand, Envelope as CoreEnvelope, EnvelopePoint as CoreEnvelopePoint, FrequencyTable,
    Instrument, InstrumentName, Module, ModuleHeader, ModuleTitle, Note, Pattern, PatternCell,
    Sample, SampleData as CoreSampleData, SampleLoopKind, SampleName, Vibrato as CoreVibrato,
    SAMPLES_PER_INSTRUMENT, SAMPLE_DEFAULT_FLAGS, SAMPLE_DEFAULT_VOLUME_FADEOUT,
};

const XM_SIGNATURE: &[u8; 17] = b"Extended Module: ";
const XM_MARKER: u8 = 0x1a;
const TITLE_OFFSET: usize = 17;
const TITLE_LEN: usize = 20;
const MARKER_OFFSET: usize = 37;
const TRACKER_OFFSET: usize = 38;
const TRACKER_LEN: usize = 20;
const VERSION_OFFSET: usize = 58;
const HEADER_SIZE_OFFSET: usize = 60;
const HEADER_FIELDS_OFFSET: usize = 64;
const ORDER_TABLE_OFFSET: usize = 80;
const XM_ORDER_TABLE_LEN: usize = 256;
const XM_MIN_HEADER_BYTES: usize = ORDER_TABLE_OFFSET + XM_ORDER_TABLE_LEN;
const XM_EXPANDED_EFFECT_SLOTS: u8 = 2;
const XM_VERSION_1_02: u16 = 0x0102;
const XM_VERSION_1_03: u16 = 0x0103;
const XM_VERSION_1_04: u16 = 0x0104;
const XM_WRITER_TRACKER_NAME: &str = "RustyTracker";
const XM_WRITER_HEADER_SIZE: u32 = 276;
const XM_WRITER_AMIGA_FLAGS: u16 = 0x0000;
const XM_WRITER_EMPTY_ORDER: u8 = 0;
const XM_WRITER_PATTERN_HEADER_LEN: u32 = XM_PATTERN_HEADER_LEN as u32;
const XM_WRITER_PATTERN_PACKING_TYPE: u8 = 0;
const XM_WRITER_EMPTY_VOLUME_COLUMN: u8 = 0;
const XM_WRITER_SINGLE_EFFECT_SLOT_COUNT: usize = 1;
const XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE: u32 = XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE;
const XM_WRITER_INSTRUMENT_HEADER_SIZE: u32 =
    XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE + XM_INSTRUMENT_EXTENSION_MAX_LEN as u32;
const XM_WRITER_INSTRUMENT_TYPE: u8 = 0;
const XM_WRITER_SAMPLE_HEADER_SIZE: u32 = XM_SAMPLE_HEADER_LEN as u32;
const XM_WRITER_EMPTY_SAMPLE_BYTE_LEN: u32 = 0;
const XM_WRITER_SAMPLE_RESERVED: u8 = 0;
const XM_WRITER_EMPTY_ENVELOPE_FRAME: u16 = 0;
const XM_WRITER_EMPTY_ENVELOPE_VALUE: u16 = 0;
const XM_WRITER_DELTA_INITIAL_8: i8 = 0;
const XM_WRITER_DELTA_INITIAL_16: i16 = 0;
const U32_FIELD_MAX: u64 = u32::MAX as u64;
const XM_HEADER_FIELD_STEP: usize = 2;
const XM_RESTART_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP;
const XM_CHANNELS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 2;
const XM_PATTERNS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 3;
const XM_INSTRUMENTS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 4;
const XM_FLAGS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 5;
const XM_TICK_SPEED_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 6;
const XM_BPM_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 7;
const XM_LINEAR_FREQUENCY_FLAG: u16 = 0x0001;
const XM_1_02_PATTERN_HEADER_LEN: usize = 8;
const XM_PATTERN_HEADER_LEN: usize = 9;
const XM_PATTERN_TYPE_OFFSET: usize = 4;
const XM_1_02_PATTERN_ROWS_OFFSET: usize = 5;
const XM_1_02_PATTERN_DATA_LEN_OFFSET: usize = 6;
const XM_1_02_ROW_COUNT_BASE: u16 = 1;
const XM_PATTERN_ROWS_OFFSET: usize = 5;
const XM_PATTERN_DATA_LEN_OFFSET: usize = 7;
const XM_CELL_FIELD_COUNT: usize = 5;
const XM_CELL_PACKED_FLAG: u8 = 0x80;
const XM_NOTE_FIELD_INDEX: usize = 0;
const XM_INSTRUMENT_FIELD_INDEX: usize = 1;
const XM_VOLUME_FIELD_INDEX: usize = 2;
const XM_EFFECT_FIELD_INDEX: usize = 3;
const XM_OPERAND_FIELD_INDEX: usize = 4;
const XM_FIELD_PRESENT_BIT_BASE: u8 = 1;
const FIRST_UNPACKED_CELL_FIELD: usize = 1;
const XM_NOTE_EMPTY: u8 = 0;
const XM_NOTE_OFF: u8 = 97;
const EMPTY_EFFECT: u8 = 0;
const EMPTY_OPERAND: u8 = 0;
const ASCII_CONTROL_MAX: u8 = 32;
const ASCII_DELETE: u8 = 127;
const ASCII_NUL: u8 = 0;
const TEXT_INDEX_TO_LEN_OFFSET: usize = 1;
const BYTE_1_OFFSET: usize = 1;
const BYTE_2_OFFSET: usize = 2;
const BYTE_3_OFFSET: usize = 3;
const VALID_XM_EFFECTS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 21, 25, 27, 29, 33,
];
const XM_EFFECT_PROTRACKER_MIN: u8 = 0x01;
const XM_EFFECT_PROTRACKER_MAX: u8 = 0x11;
const XM_EFFECT_VOLUME: u8 = 0x0c;
const XM_EFFECT_GLOBAL_VOLUME: u8 = 0x10;
const XM_EFFECT_EXTENDED: u8 = 0x0e;
const XM_EFFECT_EXTRA_FINE_PORTA: u8 = 0x21;
const INTERNAL_EFFECT_NONZERO_ARPEGGIO: u8 = 0x20;
const INTERNAL_EFFECT_EXTENDED_BASE: u8 = 0x30;
const INTERNAL_EFFECT_EXTENDED_MAX: u8 = INTERNAL_EFFECT_EXTENDED_BASE + XM_NIBBLE_MASK;
const INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE: u8 = 0x40;
const XM_EXTRA_FINE_PORTA_UP_COMMAND: u8 = 0x01;
const XM_EXTRA_FINE_PORTA_DOWN_COMMAND: u8 = 0x02;
const INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN: u8 =
    INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE + XM_EXTRA_FINE_PORTA_UP_COMMAND;
const INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX: u8 =
    INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE + XM_EXTRA_FINE_PORTA_DOWN_COMMAND;
const XM_VOLUME_SET_MIN: u8 = 0x10;
const XM_VOLUME_SET_MAX: u8 = 0x50;
const XM_VOLUME_COMMAND_MIN: u8 = 0x60;
const XM_VOLUME_SLIDE_DOWN: u8 = 0x6;
const XM_VOLUME_SLIDE_UP: u8 = 0x7;
const XM_VOLUME_FINE_DOWN: u8 = 0x8;
const XM_VOLUME_FINE_UP: u8 = 0x9;
const XM_VOLUME_SET_VIBRATO_SPEED: u8 = 0xA;
const XM_VOLUME_VIBRATO: u8 = 0xB;
const XM_VOLUME_SET_PANNING: u8 = 0xC;
const XM_VOLUME_PANNING_SLIDE_LEFT: u8 = 0xD;
const XM_VOLUME_PANNING_SLIDE_RIGHT: u8 = 0xE;
const XM_VOLUME_TONE_PORTAMENTO: u8 = 0xF;
const INTERNAL_EFFECT_VOLUME_SLIDE: u8 = 0x0a;
const XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND: u8 = 0x0a;
const XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND: u8 = 0x0b;
const INTERNAL_EFFECT_FINE_VOLUME_SLIDE_UP: u8 =
    INTERNAL_EFFECT_EXTENDED_BASE + XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND;
const INTERNAL_EFFECT_FINE_VOLUME_SLIDE_DOWN: u8 =
    INTERNAL_EFFECT_EXTENDED_BASE + XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND;
const INTERNAL_EFFECT_VIBRATO_COMPAT: u8 = 0x04;
const INTERNAL_EFFECT_PANNING: u8 = 0x08;
const INTERNAL_EFFECT_PANNING_SLIDE: u8 = 0x19;
const INTERNAL_EFFECT_TONE_PORTAMENTO: u8 = 0x03;
const XM_NIBBLE_SHIFT: u8 = 4;
const XM_NIBBLE_MASK: u8 = 0x0f;
const XM_VOLUME_MAX: u8 = 64;
const VOL64_TO_255_SCALE: u32 = 261_120;
const VOL64_TO_255_ROUNDING: u32 = 65_535;
const VOL64_TO_255_SHIFT: u32 = 16;
const CORE_VOLUME_MAX: u16 = 255;
const BYTE_MASK: u32 = 0xff;
const XM_PAN_COLUMN_MAX: u8 = 0x0f;
const FULL_PANNING: u8 = 0xff;
const XM_INSTRUMENT_SIZE_LEN: usize = 4;
const XM_INSTRUMENT_SHORT_SIZE_MIN: u32 = 4;
const XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE: u32 = 29;
const XM_INSTRUMENT_SHORT_BUFFER_LEN: usize = 29;
const XM_INSTRUMENT_FIXED_FIELDS_LEN: usize = 25;
const XM_INSTRUMENT_NAME_LEN: usize = 22;
const XM_INSTRUMENT_TYPE_OFFSET: usize = 22;
const XM_INSTRUMENT_SAMPLE_COUNT_OFFSET: usize = 23;
const XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE: u32 = 33;
const XM_INSTRUMENT_EXTENSION_MAX_LEN: usize = 230;
const XM_NOTE_SAMPLE_MAP_LEN: usize = 96;
const XM_ENVELOPE_POINT_COUNT: usize = 12;
const XM_ENVELOPE_POINT_BYTES: usize = 4;
const XM_ENVELOPE_X_OFFSET: usize = 0;
const XM_ENVELOPE_Y_OFFSET: usize = 2;
const XM_ENVELOPE_VALUE_SHIFT: u16 = 2;
const XM_SAMPLE_HEADER_SIZE_LEN: usize = 4;
const XM_SAMPLE_HEADER_LEN: usize = 40;
const XM_SAMPLE_NAME_LEN: usize = 22;
const XM_ENVELOPE_POINT_COUNT_MAX: u8 = XM_ENVELOPE_POINT_COUNT as u8;
const XM_VIBRATO_DEPTH_SHIFT: u8 = 1;
const XM_VOLUME_FADEOUT_SHIFT: u16 = 1;
const XM_SAMPLE_LENGTH_LEN: usize = 4;
const XM_SAMPLE_LOOP_START_LEN: usize = 4;
const XM_SAMPLE_LOOP_LENGTH_LEN: usize = 4;
const XM_SAMPLE_VOLUME_LEN: usize = 1;
const XM_SAMPLE_FINETUNE_LEN: usize = 1;
const XM_SAMPLE_TYPE_LEN: usize = 1;
const XM_SAMPLE_PANNING_LEN: usize = 1;
const XM_SAMPLE_RELATIVE_NOTE_LEN: usize = 1;
const XM_SAMPLE_RESERVED_LEN: usize = 1;
const XM_EMPTY_SAMPLE_DATA_LEN: u32 = 0;
const XM_SAMPLE_8_BIT_FLAG: u8 = 0x00;
const XM_SAMPLE_16_BIT_FLAG: u8 = 0x10;
const XM_SAMPLE_LOOP_MASK: u8 = 0x03;
const XM_SAMPLE_NON_LOOP_TYPE_MASK: u8 = !XM_SAMPLE_LOOP_MASK;
const XM_SAMPLE_LOOP_NONE: u8 = 0x00;
const XM_SAMPLE_LOOP_FORWARD: u8 = 0x01;
const XM_SAMPLE_LOOP_PING_PONG: u8 = 0x02;
const XM_SAMPLE_LOOP_UNDEFINED: u8 = 0x03;
const XM_SAMPLE_STEREO_FLAG: u8 = 0x20;
const XM_SAMPLE_ADPCM_RESERVED: u8 = 0xad;
const BYTES_PER_8_BIT_SAMPLE: usize = 1;
const BYTES_PER_16_BIT_SAMPLE: usize = 2;
const STEREO_CHANNEL_COUNT: usize = 2;
const STEREO_CHANNEL_COUNT_U32: u32 = STEREO_CHANNEL_COUNT as u32;
const STEREO_AVERAGE_SHIFT: u8 = 1;
const XM_ORDER_REFERENCE_PATTERN_ROWS: u16 = rustytracker_core::DEFAULT_PATTERN_ROWS;
const U16_MAX_AS_USIZE: usize = u16::MAX as usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmParseError {
    Truncated {
        expected: usize,
        actual: usize,
    },
    InvalidSignature,
    InvalidMarker(u8),
    UnsupportedVersion(u16),
    OrderTableTooShort {
        song_length: usize,
        available: usize,
    },
    PatternHeaderTooShort {
        pattern_index: usize,
        expected: usize,
        actual: usize,
    },
    PatternDataTooShort {
        pattern_index: usize,
        expected: usize,
        actual: usize,
    },
    PackedPatternCellTooShort {
        pattern_index: usize,
        row: u16,
        channel: u16,
        expected: usize,
        actual: usize,
    },
    PackedPatternDataLengthMismatch {
        pattern_index: usize,
        consumed: usize,
        declared: usize,
    },
    InstrumentHeaderTooShort {
        instrument_index: usize,
        expected: usize,
        actual: usize,
    },
    InstrumentBodyTooShort {
        instrument_index: usize,
        expected: usize,
        actual: usize,
    },
    InvalidInstrumentSize {
        instrument_index: usize,
        size: u32,
    },
    InstrumentExtensionTooLong {
        instrument_index: usize,
        extension_len: usize,
        maximum: usize,
    },
    TooManyInstrumentSamples {
        instrument_index: usize,
        sample_count: u16,
        maximum: usize,
    },
    SampleHeaderTooShort {
        instrument_index: usize,
        sample_index: usize,
        expected: usize,
        actual: usize,
    },
    SampleDataTooShort {
        instrument_index: usize,
        sample_index: usize,
        expected: usize,
        actual: usize,
    },
    UnsupportedAdpcmSample {
        instrument_index: usize,
        sample_index: usize,
    },
}

pub type XmResult<T> = Result<T, XmParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmWriteError {
    TooManyOrders {
        requested: usize,
        maximum: usize,
    },
    TooManyPatterns {
        requested: usize,
        maximum: usize,
    },
    TooManyInstruments {
        requested: usize,
        maximum: usize,
    },
    PatternDataTooLong {
        pattern_index: usize,
        byte_len: usize,
        maximum: usize,
    },
    TooManyInstrumentSamples {
        instrument_index: usize,
        requested: usize,
        maximum: usize,
    },
    SampleFieldTooLarge {
        instrument_index: usize,
        sample_index: usize,
        field: XmSampleField,
        value: u64,
        maximum: u64,
    },
}

pub type XmWriteResult<T> = Result<T, XmWriteError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmSampleField {
    Length,
    LoopStart,
    LoopLength,
}

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

pub fn parse_xm_header(bytes: &[u8]) -> XmResult<XmModuleHeader> {
    if bytes.len() < XM_MIN_HEADER_BYTES {
        return Err(XmParseError::Truncated {
            expected: XM_MIN_HEADER_BYTES,
            actual: bytes.len(),
        });
    }

    if &bytes[..XM_SIGNATURE.len()] != XM_SIGNATURE {
        return Err(XmParseError::InvalidSignature);
    }

    if bytes[MARKER_OFFSET] != XM_MARKER {
        return Err(XmParseError::InvalidMarker(bytes[MARKER_OFFSET]));
    }

    let version = read_u16(bytes, VERSION_OFFSET);
    if !matches!(version, XM_VERSION_1_02 | XM_VERSION_1_03 | XM_VERSION_1_04) {
        return Err(XmParseError::UnsupportedVersion(version));
    }

    let header_size = read_u32(bytes, HEADER_SIZE_OFFSET);
    let song_length = read_u16(bytes, HEADER_FIELDS_OFFSET);
    let restart_position = read_u16(bytes, XM_RESTART_FIELD_OFFSET);
    let channel_count = read_u16(bytes, XM_CHANNELS_FIELD_OFFSET);
    let pattern_count = read_u16(bytes, XM_PATTERNS_FIELD_OFFSET);
    let instrument_count = read_u16(bytes, XM_INSTRUMENTS_FIELD_OFFSET);
    let flags = read_u16(bytes, XM_FLAGS_FIELD_OFFSET);
    let default_tick_speed = read_u16(bytes, XM_TICK_SPEED_FIELD_OFFSET);
    let default_bpm = read_u16(bytes, XM_BPM_FIELD_OFFSET);

    let order_end = ORDER_TABLE_OFFSET + song_length as usize;
    if order_end > bytes.len() {
        return Err(XmParseError::OrderTableTooShort {
            song_length: song_length as usize,
            available: bytes.len().saturating_sub(ORDER_TABLE_OFFSET),
        });
    }

    Ok(XmModuleHeader {
        title: decode_fixed_text(&bytes[TITLE_OFFSET..TITLE_OFFSET + TITLE_LEN]),
        tracker_name: decode_fixed_text(&bytes[TRACKER_OFFSET..TRACKER_OFFSET + TRACKER_LEN]),
        version,
        header_size,
        song_length,
        restart_position,
        channel_count,
        pattern_count,
        instrument_count,
        flags,
        frequency_table: if flags & XM_LINEAR_FREQUENCY_FLAG == XM_LINEAR_FREQUENCY_FLAG {
            FrequencyTable::Linear
        } else {
            FrequencyTable::Amiga
        },
        default_tick_speed,
        default_bpm,
        orders: bytes[ORDER_TABLE_OFFSET..order_end].to_vec(),
    })
}

pub fn write_xm_header(module: &Module) -> XmWriteResult<Vec<u8>> {
    if module.orders.len() > XM_ORDER_TABLE_LEN {
        return Err(XmWriteError::TooManyOrders {
            requested: module.orders.len(),
            maximum: XM_ORDER_TABLE_LEN,
        });
    }

    if module.patterns.len() > U16_MAX_AS_USIZE {
        return Err(XmWriteError::TooManyPatterns {
            requested: module.patterns.len(),
            maximum: U16_MAX_AS_USIZE,
        });
    }

    if module.instruments.len() > U16_MAX_AS_USIZE {
        return Err(XmWriteError::TooManyInstruments {
            requested: module.instruments.len(),
            maximum: U16_MAX_AS_USIZE,
        });
    }

    let mut bytes = vec![ASCII_NUL; XM_MIN_HEADER_BYTES];

    bytes[..XM_SIGNATURE.len()].copy_from_slice(XM_SIGNATURE);
    bytes[MARKER_OFFSET] = XM_MARKER;
    write_fixed_text(
        &mut bytes[TITLE_OFFSET..TITLE_OFFSET + TITLE_LEN],
        module.header.title.as_str(),
    );
    write_fixed_text(
        &mut bytes[TRACKER_OFFSET..TRACKER_OFFSET + TRACKER_LEN],
        XM_WRITER_TRACKER_NAME,
    );
    write_u16(&mut bytes, VERSION_OFFSET, XM_VERSION_1_04);
    write_u32(&mut bytes, HEADER_SIZE_OFFSET, XM_WRITER_HEADER_SIZE);
    write_u16(&mut bytes, HEADER_FIELDS_OFFSET, module.orders.len() as u16);
    write_u16(
        &mut bytes,
        XM_RESTART_FIELD_OFFSET,
        module.header.restart_position,
    );
    write_u16(
        &mut bytes,
        XM_CHANNELS_FIELD_OFFSET,
        module.header.channel_count,
    );
    write_u16(
        &mut bytes,
        XM_PATTERNS_FIELD_OFFSET,
        module.patterns.len() as u16,
    );
    write_u16(
        &mut bytes,
        XM_INSTRUMENTS_FIELD_OFFSET,
        module.instruments.len() as u16,
    );
    write_u16(
        &mut bytes,
        XM_FLAGS_FIELD_OFFSET,
        match module.header.frequency_table {
            FrequencyTable::Amiga => XM_WRITER_AMIGA_FLAGS,
            FrequencyTable::Linear => XM_LINEAR_FREQUENCY_FLAG,
        },
    );
    write_u16(
        &mut bytes,
        XM_TICK_SPEED_FIELD_OFFSET,
        module.header.tick_speed,
    );
    write_u16(&mut bytes, XM_BPM_FIELD_OFFSET, module.header.bpm);

    bytes[ORDER_TABLE_OFFSET..ORDER_TABLE_OFFSET + XM_ORDER_TABLE_LEN].fill(XM_WRITER_EMPTY_ORDER);
    bytes[ORDER_TABLE_OFFSET..ORDER_TABLE_OFFSET + module.orders.len()]
        .copy_from_slice(&module.orders);

    Ok(bytes)
}

pub fn write_xm_patterns(module: &Module) -> XmWriteResult<Vec<u8>> {
    let mut bytes = Vec::new();

    for (pattern_index, pattern) in module.patterns.iter().enumerate() {
        let data = if pattern_is_empty(pattern) {
            Vec::new()
        } else {
            write_xm_pattern_data(pattern)
        };

        if data.len() > U16_MAX_AS_USIZE {
            return Err(XmWriteError::PatternDataTooLong {
                pattern_index,
                byte_len: data.len(),
                maximum: U16_MAX_AS_USIZE,
            });
        }

        let header_offset = bytes.len();
        bytes.resize(header_offset + XM_PATTERN_HEADER_LEN, ASCII_NUL);
        write_u32(&mut bytes, header_offset, XM_WRITER_PATTERN_HEADER_LEN);
        bytes[header_offset + XM_PATTERN_TYPE_OFFSET] = XM_WRITER_PATTERN_PACKING_TYPE;
        write_u16(
            &mut bytes,
            header_offset + XM_PATTERN_ROWS_OFFSET,
            pattern.rows(),
        );
        write_u16(
            &mut bytes,
            header_offset + XM_PATTERN_DATA_LEN_OFFSET,
            data.len() as u16,
        );
        bytes.extend_from_slice(&data);
    }

    Ok(bytes)
}

pub fn write_xm_instruments(module: &Module) -> XmWriteResult<Vec<u8>> {
    let mut bytes = Vec::new();

    for (instrument_index, instrument) in module.instruments.iter().enumerate() {
        write_xm_instrument(&mut bytes, module, instrument_index, instrument)?;
    }

    Ok(bytes)
}

fn active_xm_sample_count(module: &Module, instrument: &Instrument) -> usize {
    instrument
        .sample_slots
        .iter()
        .enumerate()
        .rev()
        .find(|(_, sample_index)| {
            sample_index
                .and_then(|sample_index| module.samples.get(sample_index))
                .is_some_and(sample_is_active)
        })
        .map(|(sample_index, _)| sample_index + 1)
        .unwrap_or_default()
}

fn sample_is_active(sample: &Sample) -> bool {
    sample != &Sample::default()
}

fn write_xm_instrument(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument_index: usize,
    instrument: &Instrument,
) -> XmWriteResult<()> {
    let sample_count = active_xm_sample_count(module, instrument);
    if sample_count > SAMPLES_PER_INSTRUMENT {
        return Err(XmWriteError::TooManyInstrumentSamples {
            instrument_index,
            requested: sample_count,
            maximum: SAMPLES_PER_INSTRUMENT,
        });
    }

    let header_size = if sample_count == 0 {
        XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE
    } else {
        XM_WRITER_INSTRUMENT_HEADER_SIZE
    };
    let instrument_offset = bytes.len();
    bytes.resize(instrument_offset + header_size as usize, ASCII_NUL);

    write_u32(bytes, instrument_offset, header_size);
    write_fixed_text(
        &mut bytes[instrument_offset + XM_INSTRUMENT_SIZE_LEN
            ..instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_NAME_LEN],
        instrument.name.as_str(),
    );
    bytes[instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_TYPE_OFFSET] =
        XM_WRITER_INSTRUMENT_TYPE;
    write_u16(
        bytes,
        instrument_offset + XM_INSTRUMENT_SIZE_LEN + XM_INSTRUMENT_SAMPLE_COUNT_OFFSET,
        sample_count as u16,
    );

    if sample_count == 0 {
        return Ok(());
    }

    let sample_header_size_offset =
        instrument_offset + XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE as usize;
    write_u32(
        bytes,
        sample_header_size_offset,
        XM_WRITER_SAMPLE_HEADER_SIZE,
    );
    write_xm_instrument_extension(
        bytes,
        sample_header_size_offset + XM_SAMPLE_HEADER_SIZE_LEN,
        instrument,
        sample_count,
    );

    for sample_index in 0..sample_count {
        write_xm_sample_header(bytes, module, instrument_index, instrument, sample_index)?;
    }

    for sample_index in 0..sample_count {
        write_xm_sample_payload(bytes, module, instrument, sample_index);
    }

    Ok(())
}

fn write_xm_instrument_extension(
    bytes: &mut [u8],
    extension_offset: usize,
    instrument: &Instrument,
    sample_count: usize,
) {
    let mut offset = extension_offset;

    write_xm_note_sample_map(bytes, offset, instrument, sample_count);
    offset += XM_NOTE_SAMPLE_MAP_LEN;

    write_xm_envelope_points(bytes, offset, &instrument.volume_envelope);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;
    write_xm_envelope_points(bytes, offset, &instrument.panning_envelope);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;

    bytes[offset] = instrument
        .volume_envelope
        .point_count
        .min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument
        .panning_envelope
        .point_count
        .min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.sustain_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.loop_start_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.loop_end_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.sustain_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.loop_start_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.loop_end_point;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.volume_envelope.flags;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.panning_envelope.flags;
    offset += BYTE_1_OFFSET;

    bytes[offset] = instrument.vibrato.waveform;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.sweep;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.depth >> XM_VIBRATO_DEPTH_SHIFT;
    offset += BYTE_1_OFFSET;
    bytes[offset] = instrument.vibrato.rate;
    offset += BYTE_1_OFFSET;
    write_u16(
        bytes,
        offset,
        instrument.volume_fadeout >> XM_VOLUME_FADEOUT_SHIFT,
    );
}

fn write_xm_note_sample_map(
    bytes: &mut [u8],
    offset: usize,
    instrument: &Instrument,
    sample_count: usize,
) {
    for note_index in 0..XM_NOTE_SAMPLE_MAP_LEN {
        bytes[offset + note_index] = instrument
            .note_sample_map
            .get(note_index)
            .and_then(|sample_index| *sample_index)
            .and_then(|sample_index| xm_sample_slot_for_core_sample(instrument, sample_index))
            .filter(|sample_index| *sample_index < sample_count)
            .map(|sample_index| sample_index as u8)
            .unwrap_or_default();
    }
}

fn xm_sample_slot_for_core_sample(
    instrument: &Instrument,
    core_sample_index: usize,
) -> Option<usize> {
    instrument
        .sample_slots
        .iter()
        .position(|sample_index| *sample_index == Some(core_sample_index))
}

fn write_xm_envelope_points(bytes: &mut [u8], offset: usize, envelope: &CoreEnvelope) {
    for point_index in 0..XM_ENVELOPE_POINT_COUNT {
        let point_offset = offset + point_index * XM_ENVELOPE_POINT_BYTES;
        let point = envelope
            .points
            .get(point_index)
            .copied()
            .unwrap_or(CoreEnvelopePoint {
                frame: XM_WRITER_EMPTY_ENVELOPE_FRAME,
                value: XM_WRITER_EMPTY_ENVELOPE_VALUE,
            });

        write_u16(bytes, point_offset + XM_ENVELOPE_X_OFFSET, point.frame);
        write_u16(
            bytes,
            point_offset + XM_ENVELOPE_Y_OFFSET,
            point.value >> XM_ENVELOPE_VALUE_SHIFT,
        );
    }
}

fn write_xm_sample_header(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument_index: usize,
    instrument: &Instrument,
    sample_index: usize,
) -> XmWriteResult<()> {
    let sample = xm_sample_for_slot(module, instrument, sample_index);
    let header_offset = bytes.len();
    bytes.resize(header_offset + XM_SAMPLE_HEADER_LEN, ASCII_NUL);

    if let Some(sample) = sample {
        let sample_byte_len =
            xm_sample_data_byte_len(&sample.data, instrument_index, sample_index)?;
        let loop_start_byte_len = xm_sample_frame_count_to_byte_len(
            sample.loop_start,
            &sample.data,
            instrument_index,
            sample_index,
            XmSampleField::LoopStart,
        )?;
        let loop_length_byte_len = xm_sample_frame_count_to_byte_len(
            sample.loop_length,
            &sample.data,
            instrument_index,
            sample_index,
            XmSampleField::LoopLength,
        )?;
        let mut cursor = header_offset;
        write_u32(bytes, cursor, sample_byte_len);
        cursor += XM_SAMPLE_LENGTH_LEN;
        write_u32(bytes, cursor, loop_start_byte_len);
        cursor += XM_SAMPLE_LOOP_START_LEN;
        write_u32(bytes, cursor, loop_length_byte_len);
        cursor += XM_SAMPLE_LOOP_LENGTH_LEN;
        bytes[cursor] = vol255_to_64(sample.volume);
        cursor += XM_SAMPLE_VOLUME_LEN;
        bytes[cursor] = sample.finetune as u8;
        cursor += XM_SAMPLE_FINETUNE_LEN;
        bytes[cursor] = xm_sample_type(sample);
        cursor += XM_SAMPLE_TYPE_LEN;
        bytes[cursor] = sample.panning;
        cursor += XM_SAMPLE_PANNING_LEN;
        bytes[cursor] = sample.relative_note as u8;
        cursor += XM_SAMPLE_RELATIVE_NOTE_LEN;
        bytes[cursor] = XM_WRITER_SAMPLE_RESERVED;
        cursor += XM_SAMPLE_RESERVED_LEN;
        write_fixed_text(
            &mut bytes[cursor..cursor + XM_SAMPLE_NAME_LEN],
            sample.name.as_str(),
        );
    }

    Ok(())
}

fn xm_sample_type(sample: &Sample) -> u8 {
    xm_sample_data_type(sample) | xm_sample_loop_kind(sample.loop_kind)
}

fn xm_sample_data_type(sample: &Sample) -> u8 {
    match &sample.data {
        CoreSampleData::Empty => sample.sample_type & XM_SAMPLE_NON_LOOP_TYPE_MASK,
        CoreSampleData::Pcm8(_) => XM_SAMPLE_8_BIT_FLAG,
        CoreSampleData::Pcm16(_) => XM_SAMPLE_16_BIT_FLAG,
    }
}

fn xm_sample_loop_kind(loop_kind: SampleLoopKind) -> u8 {
    match loop_kind {
        SampleLoopKind::None => XM_SAMPLE_LOOP_NONE,
        SampleLoopKind::Forward => XM_SAMPLE_LOOP_FORWARD,
        SampleLoopKind::PingPong => XM_SAMPLE_LOOP_PING_PONG,
    }
}

fn xm_sample_for_slot<'a>(
    module: &'a Module,
    instrument: &Instrument,
    sample_index: usize,
) -> Option<&'a Sample> {
    instrument
        .sample_slots
        .get(sample_index)
        .and_then(|sample_index| *sample_index)
        .and_then(|sample_index| module.samples.get(sample_index))
}

fn xm_sample_data_byte_len(
    data: &CoreSampleData,
    instrument_index: usize,
    sample_index: usize,
) -> XmWriteResult<u32> {
    if matches!(data, CoreSampleData::Empty) {
        return Ok(XM_WRITER_EMPTY_SAMPLE_BYTE_LEN);
    }

    let frame_count = data.frame_count() as u64;
    let byte_len = frame_count.saturating_mul(xm_sample_bytes_per_frame(data) as u64);

    xm_u32_sample_field(
        byte_len,
        instrument_index,
        sample_index,
        XmSampleField::Length,
    )
}

fn xm_sample_frame_count_to_byte_len(
    frame_count: u32,
    data: &CoreSampleData,
    instrument_index: usize,
    sample_index: usize,
    field: XmSampleField,
) -> XmWriteResult<u32> {
    if matches!(data, CoreSampleData::Empty) {
        return Ok(XM_WRITER_EMPTY_SAMPLE_BYTE_LEN);
    }

    let byte_len = u64::from(frame_count) * xm_sample_bytes_per_frame(data) as u64;

    xm_u32_sample_field(byte_len, instrument_index, sample_index, field)
}

fn xm_u32_sample_field(
    value: u64,
    instrument_index: usize,
    sample_index: usize,
    field: XmSampleField,
) -> XmWriteResult<u32> {
    if value > U32_FIELD_MAX {
        return Err(XmWriteError::SampleFieldTooLarge {
            instrument_index,
            sample_index,
            field,
            value,
            maximum: U32_FIELD_MAX,
        });
    }

    Ok(value as u32)
}

fn xm_sample_bytes_per_frame(data: &CoreSampleData) -> usize {
    match data {
        CoreSampleData::Empty | CoreSampleData::Pcm8(_) => BYTES_PER_8_BIT_SAMPLE,
        CoreSampleData::Pcm16(_) => BYTES_PER_16_BIT_SAMPLE,
    }
}

fn write_xm_sample_payload(
    bytes: &mut Vec<u8>,
    module: &Module,
    instrument: &Instrument,
    sample_index: usize,
) {
    if let Some(sample) = xm_sample_for_slot(module, instrument, sample_index) {
        match &sample.data {
            CoreSampleData::Empty => {}
            CoreSampleData::Pcm8(values) => write_xm_delta8(bytes, values),
            CoreSampleData::Pcm16(values) => write_xm_delta16(bytes, values),
        }
    }
}

fn write_xm_delta8(bytes: &mut Vec<u8>, values: &[i8]) {
    let mut previous = XM_WRITER_DELTA_INITIAL_8;

    for &value in values {
        let delta = value.wrapping_sub(previous);
        bytes.push(delta as u8);
        previous = value;
    }
}

fn write_xm_delta16(bytes: &mut Vec<u8>, values: &[i16]) {
    let mut previous = XM_WRITER_DELTA_INITIAL_16;

    for &value in values {
        let delta = value.wrapping_sub(previous);
        bytes.extend_from_slice(&delta.to_le_bytes());
        previous = value;
    }
}

fn pattern_is_empty(pattern: &Pattern) -> bool {
    for row in 0..pattern.rows() {
        for channel in 0..pattern.channels() {
            let cell = pattern
                .cell(channel, row)
                .expect("writer walks cells inside pattern bounds");
            if cell.note != Note::Empty
                || cell.instrument != EMPTY_OPERAND
                || cell
                    .effects
                    .iter()
                    .any(|effect| *effect != EffectCommand::default())
            {
                return false;
            }
        }
    }

    true
}

fn write_xm_pattern_data(pattern: &Pattern) -> Vec<u8> {
    let mut bytes = Vec::new();

    for row in 0..pattern.rows() {
        for channel in 0..pattern.channels() {
            let cell = pattern
                .cell(channel, row)
                .expect("writer walks cells inside pattern bounds");
            write_xm_cell(&mut bytes, cell);
        }
    }

    bytes
}

fn write_xm_cell(bytes: &mut Vec<u8>, cell: &PatternCell) {
    let (volume, effect) = xm_columns_from_core_effects(&cell.effects);

    bytes.push(core_note_to_xm(cell.note));
    bytes.push(cell.instrument);
    bytes.push(volume);
    bytes.push(effect.effect);
    bytes.push(effect.operand);
}

fn xm_columns_from_core_effects(effects: &[EffectCommand]) -> (u8, EffectCommand) {
    if effects.len() <= XM_WRITER_SINGLE_EFFECT_SLOT_COUNT {
        let effect = effects
            .iter()
            .rev()
            .find(|effect| **effect != EffectCommand::default())
            .copied()
            .map(core_effect_to_xm)
            .unwrap_or_default();

        return (XM_WRITER_EMPTY_VOLUME_COLUMN, effect);
    }

    let mut volume = XM_WRITER_EMPTY_VOLUME_COLUMN;
    let mut effect_column = EffectCommand::default();

    for (index, effect) in effects.iter().copied().enumerate() {
        if effect == EffectCommand::default() {
            continue;
        }

        let xm_effect = core_effect_to_xm(effect);

        if index == 0 {
            if !note_portamento_requires_effect_column(xm_effect) {
                if let Some(volume_column) = xm_effect_to_volume_column(xm_effect, true) {
                    volume = volume_column;
                    continue;
                }
            }

            if effect_column == EffectCommand::default() {
                effect_column = xm_effect;
                continue;
            }
        }

        if effect_column == EffectCommand::default() {
            effect_column = xm_effect;
        } else if volume == XM_WRITER_EMPTY_VOLUME_COLUMN {
            if let Some(volume_column) = xm_effect_to_volume_column(xm_effect, false) {
                volume = volume_column;
            }
        }
    }

    (volume, effect_column)
}

fn core_effect_to_xm(effect: EffectCommand) -> EffectCommand {
    match effect.effect {
        INTERNAL_EFFECT_NONZERO_ARPEGGIO => EffectCommand {
            effect: EMPTY_EFFECT,
            operand: effect.operand,
        },
        INTERNAL_EFFECT_EXTENDED_BASE..=INTERNAL_EFFECT_EXTENDED_MAX => EffectCommand {
            effect: XM_EFFECT_EXTENDED,
            operand: ((effect.effect - INTERNAL_EFFECT_EXTENDED_BASE) << XM_NIBBLE_SHIFT)
                | (effect.operand & XM_NIBBLE_MASK),
        },
        INTERNAL_EFFECT_EXTRA_FINE_PORTA_MIN..=INTERNAL_EFFECT_EXTRA_FINE_PORTA_MAX => {
            EffectCommand {
                effect: XM_EFFECT_EXTRA_FINE_PORTA,
                operand: ((effect.effect - INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE)
                    << XM_NIBBLE_SHIFT)
                    | effect.operand.min(XM_NIBBLE_MASK),
            }
        }
        XM_EFFECT_PROTRACKER_MIN..=XM_EFFECT_PROTRACKER_MAX => {
            let operand =
                if effect.effect == XM_EFFECT_VOLUME || effect.effect == XM_EFFECT_GLOBAL_VOLUME {
                    vol255_to_64(effect.operand)
                } else {
                    effect.operand
                };

            EffectCommand {
                effect: effect.effect,
                operand,
            }
        }
        _ => effect,
    }
}

fn xm_effect_to_volume_column(
    effect: EffectCommand,
    allow_fine_volume_slide_relocation: bool,
) -> Option<u8> {
    match effect.effect {
        XM_EFFECT_VOLUME => Some(XM_VOLUME_SET_MIN + effect.operand.min(XM_VOLUME_MAX)),
        XM_EFFECT_EXTENDED if allow_fine_volume_slide_relocation => {
            xm_extended_fine_volume_slide_column(effect.operand)
        }
        XM_EFFECT_EXTENDED => None,
        INTERNAL_EFFECT_VOLUME_SLIDE => xm_volume_slide_column(effect.operand),
        INTERNAL_EFFECT_VIBRATO_COMPAT => xm_vibrato_column(effect.operand),
        INTERNAL_EFFECT_PANNING => Some(volume_command(
            XM_VOLUME_SET_PANNING,
            effect.operand >> XM_NIBBLE_SHIFT,
        )),
        INTERNAL_EFFECT_PANNING_SLIDE => xm_panning_slide_column(effect.operand),
        INTERNAL_EFFECT_TONE_PORTAMENTO => {
            if note_portamento_requires_effect_column(effect) {
                None
            } else {
                Some(volume_command(
                    XM_VOLUME_TONE_PORTAMENTO,
                    effect.operand >> XM_NIBBLE_SHIFT,
                ))
            }
        }
        _ => None,
    }
}

fn xm_volume_slide_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if low != EMPTY_OPERAND && high == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SLIDE_DOWN, low))
    } else if high != EMPTY_OPERAND && low == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SLIDE_UP, high))
    } else {
        None
    }
}

fn xm_extended_fine_volume_slide_column(operand: u8) -> Option<u8> {
    let command = operand >> XM_NIBBLE_SHIFT;
    let amount = operand & XM_NIBBLE_MASK;

    if amount == EMPTY_OPERAND {
        return None;
    }

    match command {
        XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND => {
            Some(volume_command(XM_VOLUME_FINE_DOWN, amount))
        }
        XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND => Some(volume_command(XM_VOLUME_FINE_UP, amount)),
        _ => None,
    }
}

fn xm_vibrato_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if high != EMPTY_OPERAND && low == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_SET_VIBRATO_SPEED, high))
    } else if high == EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_VIBRATO, low))
    } else {
        None
    }
}

fn xm_panning_slide_column(operand: u8) -> Option<u8> {
    let low = operand & XM_NIBBLE_MASK;
    let high = operand >> XM_NIBBLE_SHIFT;

    if low != EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_PANNING_SLIDE_LEFT, low))
    } else if high != EMPTY_OPERAND {
        Some(volume_command(XM_VOLUME_PANNING_SLIDE_RIGHT, high))
    } else {
        None
    }
}

fn note_portamento_requires_effect_column(effect: EffectCommand) -> bool {
    effect.effect == INTERNAL_EFFECT_TONE_PORTAMENTO
        && effect.operand & XM_NIBBLE_MASK != EMPTY_OPERAND
}

fn volume_command(command: u8, operand: u8) -> u8 {
    (command << XM_NIBBLE_SHIFT) | (operand & XM_NIBBLE_MASK)
}

fn core_note_to_xm(note: Note) -> u8 {
    match note {
        Note::Empty => XM_NOTE_EMPTY,
        Note::Key(value) => value,
        Note::Off => XM_NOTE_OFF,
    }
}

pub fn parse_xm_pattern_headers(
    bytes: &[u8],
    header: &XmModuleHeader,
) -> XmResult<Vec<XmPatternHeader>> {
    let fixed_pattern_header_len = if header.version == XM_VERSION_1_02 {
        XM_1_02_PATTERN_HEADER_LEN
    } else {
        XM_PATTERN_HEADER_LEN
    };
    let mut offset = HEADER_SIZE_OFFSET + header.header_size as usize;
    let mut patterns = Vec::with_capacity(header.pattern_count as usize);

    for pattern_index in 0..header.pattern_count as usize {
        let header_end = offset + fixed_pattern_header_len;
        if header_end > bytes.len() {
            return Err(XmParseError::PatternHeaderTooShort {
                pattern_index,
                expected: header_end,
                actual: bytes.len(),
            });
        }

        let header_length = read_u32(bytes, offset);
        let packing_type = bytes[offset + XM_PATTERN_TYPE_OFFSET];
        let (row_count, packed_data_len) = if header.version == XM_VERSION_1_02 {
            (
                bytes[offset + XM_1_02_PATTERN_ROWS_OFFSET] as u16 + XM_1_02_ROW_COUNT_BASE,
                read_u16(bytes, offset + XM_1_02_PATTERN_DATA_LEN_OFFSET),
            )
        } else {
            (
                read_u16(bytes, offset + XM_PATTERN_ROWS_OFFSET),
                read_u16(bytes, offset + XM_PATTERN_DATA_LEN_OFFSET),
            )
        };

        let packed_data_offset = header_end;
        let next_offset = packed_data_offset + packed_data_len as usize;
        if next_offset > bytes.len() {
            return Err(XmParseError::PatternDataTooShort {
                pattern_index,
                expected: next_offset,
                actual: bytes.len(),
            });
        }

        patterns.push(XmPatternHeader {
            index: pattern_index,
            header_length,
            packing_type,
            row_count,
            packed_data_len,
            packed_data_offset,
            next_offset,
        });
        offset = next_offset;
    }

    Ok(patterns)
}

pub fn decode_xm_patterns(bytes: &[u8], header: &XmModuleHeader) -> XmResult<Vec<Pattern>> {
    parse_xm_pattern_headers(bytes, header)?
        .iter()
        .map(|pattern_header| decode_xm_pattern(bytes, header, pattern_header))
        .collect()
}

pub fn decode_xm_pattern(
    bytes: &[u8],
    header: &XmModuleHeader,
    pattern_header: &XmPatternHeader,
) -> XmResult<Pattern> {
    if pattern_header.next_offset > bytes.len() {
        return Err(XmParseError::PatternDataTooShort {
            pattern_index: pattern_header.index,
            expected: pattern_header.next_offset,
            actual: bytes.len(),
        });
    }

    let data = &bytes[pattern_header.packed_data_offset..pattern_header.next_offset];
    let mut data_cursor = 0;
    let mut pattern = Pattern::new(
        pattern_header.row_count,
        header.channel_count,
        XM_EXPANDED_EFFECT_SLOTS,
    );

    if data.is_empty() {
        return Ok(pattern);
    }

    for row in 0..pattern_header.row_count {
        for channel in 0..header.channel_count {
            let slot = read_xm_slot(data, &mut data_cursor, pattern_header.index, row, channel)?;
            let cell = normalize_xm_slot(slot);
            pattern
                .set_cell(channel, row, cell)
                .expect("decoder writes cells inside the allocated pattern shape");
        }
    }

    if data_cursor != data.len() {
        return Err(XmParseError::PackedPatternDataLengthMismatch {
            pattern_index: pattern_header.index,
            consumed: data_cursor,
            declared: data.len(),
        });
    }

    Ok(pattern)
}

pub fn parse_xm_module(bytes: &[u8]) -> XmResult<Module> {
    let header = parse_xm_header(bytes)?;
    let pattern_headers = parse_xm_pattern_headers(bytes, &header)?;
    let mut patterns = pattern_headers
        .iter()
        .map(|pattern_header| decode_xm_pattern(bytes, &header, pattern_header))
        .collect::<XmResult<Vec<_>>>()?;
    extend_patterns_for_order_references(&mut patterns, &header);
    let instrument_offset = pattern_headers
        .last()
        .map(|pattern_header| pattern_header.next_offset)
        .unwrap_or(HEADER_SIZE_OFFSET + header.header_size as usize);
    let instrument_section = parse_xm_instruments(bytes, &header, instrument_offset)?;

    Ok(Module {
        header: ModuleHeader {
            title: ModuleTitle::new(&header.title),
            channel_count: header.channel_count,
            frequency_table: header.frequency_table,
            bpm: header.default_bpm,
            tick_speed: header.default_tick_speed,
            main_volume: rustytracker_core::DEFAULT_MAIN_VOLUME,
            restart_position: header.restart_position,
        },
        orders: header.orders,
        patterns,
        instruments: instrument_section
            .instruments
            .iter()
            .map(convert_instrument_to_core)
            .collect(),
        samples: convert_samples_to_core(&instrument_section.instruments),
    })
}

fn extend_patterns_for_order_references(patterns: &mut Vec<Pattern>, header: &XmModuleHeader) {
    let required_pattern_count = header
        .orders
        .iter()
        .map(|&pattern_index| pattern_index as usize + BYTE_1_OFFSET)
        .max()
        .unwrap_or(patterns.len());

    while patterns.len() < required_pattern_count {
        patterns.push(Pattern::new(
            XM_ORDER_REFERENCE_PATTERN_ROWS,
            header.channel_count,
            XM_EXPANDED_EFFECT_SLOTS,
        ));
    }
}

pub fn parse_xm_instruments(
    bytes: &[u8],
    header: &XmModuleHeader,
    start_offset: usize,
) -> XmResult<XmInstrumentSection> {
    let mut offset = start_offset;
    let mut instruments = Vec::with_capacity(header.instrument_count as usize);

    for instrument_index in 0..header.instrument_count as usize {
        let parsed = parse_xm_instrument(bytes, instrument_index, offset)?;
        offset = parsed.next_offset;
        instruments.push(parsed);
    }

    Ok(XmInstrumentSection {
        instruments,
        next_offset: offset,
    })
}

fn parse_xm_instrument(
    bytes: &[u8],
    instrument_index: usize,
    start_offset: usize,
) -> XmResult<XmInstrument> {
    ensure_instrument_range(
        bytes,
        start_offset,
        XM_INSTRUMENT_SIZE_LEN,
        instrument_index,
        true,
    )?;

    let header_size = read_u32(bytes, start_offset);
    let mut offset = start_offset + XM_INSTRUMENT_SIZE_LEN;
    let (name, instrument_type, sample_count) =
        read_instrument_identity(bytes, instrument_index, header_size, &mut offset)?;

    if sample_count as usize > XM_NOTE_SAMPLE_MAP_LEN {
        return Err(XmParseError::TooManyInstrumentSamples {
            instrument_index,
            sample_count,
            maximum: XM_NOTE_SAMPLE_MAP_LEN,
        });
    }

    let mut instrument = XmInstrument {
        index: instrument_index,
        header_size,
        name,
        instrument_type,
        sample_count,
        sample_header_size: None,
        note_sample_map: None,
        volume_envelope: None,
        panning_envelope: None,
        vibrato_type: None,
        vibrato_sweep: None,
        vibrato_depth: None,
        vibrato_rate: None,
        volume_fadeout: None,
        samples: Vec::with_capacity(sample_count as usize),
        next_offset: offset,
    };

    if header_size <= XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE {
        return Ok(instrument);
    }

    ensure_instrument_range(
        bytes,
        offset,
        XM_SAMPLE_HEADER_SIZE_LEN,
        instrument_index,
        false,
    )?;
    instrument.sample_header_size = Some(read_u32(bytes, offset));
    offset += XM_SAMPLE_HEADER_SIZE_LEN;

    let extension_len = header_size
        .checked_sub(XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE)
        .ok_or(XmParseError::InvalidInstrumentSize {
            instrument_index,
            size: header_size,
        })? as usize;

    if extension_len > XM_INSTRUMENT_EXTENSION_MAX_LEN {
        return Err(XmParseError::InstrumentExtensionTooLong {
            instrument_index,
            extension_len,
            maximum: XM_INSTRUMENT_EXTENSION_MAX_LEN,
        });
    }

    ensure_instrument_range(bytes, offset, extension_len, instrument_index, false)?;
    let mut extension = [ASCII_NUL; XM_INSTRUMENT_EXTENSION_MAX_LEN];
    extension[..extension_len].copy_from_slice(&bytes[offset..offset + extension_len]);
    offset += extension_len;

    let extension_data = parse_instrument_extension(&extension);
    instrument.note_sample_map = Some(extension_data.note_sample_map);
    instrument.volume_envelope = Some(extension_data.volume_envelope);
    instrument.panning_envelope = Some(extension_data.panning_envelope);
    instrument.vibrato_type = Some(extension_data.vibrato_type);
    instrument.vibrato_sweep = Some(extension_data.vibrato_sweep);
    instrument.vibrato_depth = Some(extension_data.vibrato_depth);
    instrument.vibrato_rate = Some(extension_data.vibrato_rate);
    instrument.volume_fadeout = Some(extension_data.volume_fadeout);

    let mut samples = Vec::with_capacity(sample_count as usize);
    for sample_index in 0..sample_count as usize {
        let sample = read_sample_header(bytes, instrument_index, sample_index, offset)?;
        offset += XM_SAMPLE_HEADER_LEN;
        samples.push(sample);
    }

    let mut sample_data_offset = offset;
    for sample in &mut samples {
        sample.data_offset = sample_data_offset;
        sample.data_end = sample_data_offset + sample.length as usize;
        if sample.data_end > bytes.len() {
            return Err(XmParseError::SampleDataTooShort {
                instrument_index,
                sample_index: sample.index,
                expected: sample.data_end,
                actual: bytes.len(),
            });
        }
        if is_adpcm_sample(sample.reserved) {
            return Err(XmParseError::UnsupportedAdpcmSample {
                instrument_index,
                sample_index: sample.index,
            });
        }
        sample.decoded_data = decode_sample_data(
            &bytes[sample.data_offset..sample.data_end],
            sample.sample_type,
        );
        sample_data_offset = sample.data_end;
    }

    instrument.samples = samples;
    instrument.next_offset = sample_data_offset;
    Ok(instrument)
}

fn convert_instrument_to_core(instrument: &XmInstrument) -> Instrument {
    let base_sample = instrument.index * SAMPLES_PER_INSTRUMENT;
    let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    for sample in &instrument.samples {
        if sample.index < SAMPLES_PER_INSTRUMENT {
            sample_slots[sample.index] = Some(base_sample + sample.index);
        }
    }

    Instrument {
        name: InstrumentName::new(&instrument.name),
        sample_slots,
        note_sample_map: instrument
            .note_sample_map
            .as_ref()
            .map(|note_map| {
                note_map
                    .iter()
                    .map(|&sample_index| {
                        let sample_index = sample_index as usize;
                        if sample_index < instrument.sample_count as usize
                            && sample_index < SAMPLES_PER_INSTRUMENT
                        {
                            Some(base_sample + sample_index)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![None; XM_NOTE_SAMPLE_MAP_LEN]),
        volume_envelope: instrument
            .volume_envelope
            .as_ref()
            .map(convert_envelope_to_core)
            .unwrap_or_default(),
        panning_envelope: instrument
            .panning_envelope
            .as_ref()
            .map(convert_envelope_to_core)
            .unwrap_or_default(),
        vibrato: CoreVibrato {
            waveform: instrument.vibrato_type.unwrap_or_default(),
            sweep: instrument.vibrato_sweep.unwrap_or_default(),
            depth: instrument.vibrato_depth.unwrap_or_default(),
            rate: instrument.vibrato_rate.unwrap_or_default(),
        },
        volume_fadeout: instrument
            .volume_fadeout
            .unwrap_or(SAMPLE_DEFAULT_VOLUME_FADEOUT),
    }
}

fn convert_envelope_to_core(envelope: &XmEnvelope) -> CoreEnvelope {
    CoreEnvelope {
        points: envelope
            .points
            .iter()
            .map(|point| CoreEnvelopePoint {
                frame: point.frame,
                value: point.value,
            })
            .collect(),
        point_count: envelope.point_count,
        sustain_point: envelope.sustain_point,
        loop_start_point: envelope.loop_start_point,
        loop_end_point: envelope.loop_end_point,
        flags: envelope.flags,
    }
}

fn convert_samples_to_core(instruments: &[XmInstrument]) -> Vec<Sample> {
    let mut samples = vec![Sample::default(); instruments.len() * SAMPLES_PER_INSTRUMENT];

    for instrument in instruments {
        let base_sample = instrument.index * SAMPLES_PER_INSTRUMENT;
        let volume_fadeout = instrument
            .volume_fadeout
            .unwrap_or(SAMPLE_DEFAULT_VOLUME_FADEOUT);

        for sample in &instrument.samples {
            if sample.index >= SAMPLES_PER_INSTRUMENT {
                continue;
            }

            samples[base_sample + sample.index] = Sample {
                name: SampleName::new(&sample.name),
                length: sample.frame_count,
                loop_start: sample.loop_start_frames,
                loop_length: sample.loop_length_frames,
                loop_kind: sample.loop_kind,
                volume: sample.volume,
                panning: sample.panning,
                flags: SAMPLE_DEFAULT_FLAGS,
                volume_fadeout,
                sample_type: sample.sample_type,
                finetune: sample.finetune,
                relative_note: sample.relative_note,
                data: match &sample.decoded_data {
                    XmSampleData::Pcm8(values) => CoreSampleData::Pcm8(values.clone()),
                    XmSampleData::Pcm16(values) => CoreSampleData::Pcm16(values.clone()),
                },
            };
        }
    }

    samples
}

fn read_instrument_identity(
    bytes: &[u8],
    instrument_index: usize,
    header_size: u32,
    offset: &mut usize,
) -> XmResult<(String, u8, u16)> {
    if (XM_INSTRUMENT_SHORT_SIZE_MIN..XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE).contains(&header_size) {
        let payload_len = (header_size - XM_INSTRUMENT_SIZE_LEN as u32) as usize;
        ensure_instrument_range(bytes, *offset, payload_len, instrument_index, false)?;

        let mut buffer = [ASCII_NUL; XM_INSTRUMENT_SHORT_BUFFER_LEN];
        buffer[..payload_len].copy_from_slice(&bytes[*offset..*offset + payload_len]);
        *offset += payload_len;

        return Ok((
            decode_fixed_text(&buffer[..XM_INSTRUMENT_NAME_LEN]),
            buffer[XM_INSTRUMENT_TYPE_OFFSET],
            read_u16(&buffer, XM_INSTRUMENT_SAMPLE_COUNT_OFFSET),
        ));
    }

    ensure_instrument_range(
        bytes,
        *offset,
        XM_INSTRUMENT_FIXED_FIELDS_LEN,
        instrument_index,
        false,
    )?;

    let name = decode_fixed_text(&bytes[*offset..*offset + XM_INSTRUMENT_NAME_LEN]);
    *offset += XM_INSTRUMENT_NAME_LEN;
    let instrument_type = bytes[*offset];
    *offset += BYTE_1_OFFSET;
    let sample_count = read_u16(bytes, *offset);
    *offset += BYTE_2_OFFSET;

    Ok((name, instrument_type, sample_count))
}

struct ParsedInstrumentExtension {
    note_sample_map: Vec<u8>,
    volume_envelope: XmEnvelope,
    panning_envelope: XmEnvelope,
    vibrato_type: u8,
    vibrato_sweep: u8,
    vibrato_depth: u8,
    vibrato_rate: u8,
    volume_fadeout: u16,
}

fn parse_instrument_extension(
    extension: &[u8; XM_INSTRUMENT_EXTENSION_MAX_LEN],
) -> ParsedInstrumentExtension {
    let mut offset = 0;
    let note_sample_map = extension[offset..offset + XM_NOTE_SAMPLE_MAP_LEN].to_vec();
    offset += XM_NOTE_SAMPLE_MAP_LEN;

    let volume_points = read_envelope_points(extension, offset);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;
    let panning_points = read_envelope_points(extension, offset);
    offset += XM_ENVELOPE_POINT_COUNT * XM_ENVELOPE_POINT_BYTES;

    let volume_point_count = extension[offset].min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    let panning_point_count = extension[offset].min(XM_ENVELOPE_POINT_COUNT_MAX);
    offset += BYTE_1_OFFSET;
    let volume_sustain_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_loop_start_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_loop_end_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_sustain_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_loop_start_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_loop_end_point = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_flags = extension[offset];
    offset += BYTE_1_OFFSET;
    let panning_flags = extension[offset];
    offset += BYTE_1_OFFSET;

    let vibrato_type = extension[offset];
    offset += BYTE_1_OFFSET;
    let vibrato_sweep = extension[offset];
    offset += BYTE_1_OFFSET;
    let vibrato_depth = extension[offset] << XM_VIBRATO_DEPTH_SHIFT;
    offset += BYTE_1_OFFSET;
    let vibrato_rate = extension[offset];
    offset += BYTE_1_OFFSET;
    let volume_fadeout = read_u16(extension, offset) << XM_VOLUME_FADEOUT_SHIFT;

    ParsedInstrumentExtension {
        note_sample_map,
        volume_envelope: XmEnvelope {
            points: volume_points,
            point_count: volume_point_count,
            sustain_point: volume_sustain_point,
            loop_start_point: volume_loop_start_point,
            loop_end_point: volume_loop_end_point,
            flags: volume_flags,
        },
        panning_envelope: XmEnvelope {
            points: panning_points,
            point_count: panning_point_count,
            sustain_point: panning_sustain_point,
            loop_start_point: panning_loop_start_point,
            loop_end_point: panning_loop_end_point,
            flags: panning_flags,
        },
        vibrato_type,
        vibrato_sweep,
        vibrato_depth,
        vibrato_rate,
        volume_fadeout,
    }
}

fn read_envelope_points(bytes: &[u8], offset: usize) -> Vec<XmEnvelopePoint> {
    (0..XM_ENVELOPE_POINT_COUNT)
        .map(|point_index| {
            let point_offset = offset + point_index * XM_ENVELOPE_POINT_BYTES;
            XmEnvelopePoint {
                frame: read_u16(bytes, point_offset + XM_ENVELOPE_X_OFFSET),
                value: read_u16(bytes, point_offset + XM_ENVELOPE_Y_OFFSET)
                    << XM_ENVELOPE_VALUE_SHIFT,
            }
        })
        .collect()
}

fn read_sample_header(
    bytes: &[u8],
    instrument_index: usize,
    sample_index: usize,
    offset: usize,
) -> XmResult<XmSampleHeader> {
    if offset + XM_SAMPLE_HEADER_LEN > bytes.len() {
        return Err(XmParseError::SampleHeaderTooShort {
            instrument_index,
            sample_index,
            expected: offset + XM_SAMPLE_HEADER_LEN,
            actual: bytes.len(),
        });
    }

    let mut cursor = offset;
    let length = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LENGTH_LEN;
    let loop_start = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LOOP_START_LEN;
    let loop_length = read_u32(bytes, cursor);
    cursor += XM_SAMPLE_LOOP_LENGTH_LEN;
    let volume_64 = bytes[cursor];
    cursor += XM_SAMPLE_VOLUME_LEN;
    let finetune = bytes[cursor] as i8;
    cursor += XM_SAMPLE_FINETUNE_LEN;
    let sample_type = bytes[cursor];
    cursor += XM_SAMPLE_TYPE_LEN;
    let panning = bytes[cursor];
    cursor += XM_SAMPLE_PANNING_LEN;
    let relative_note = bytes[cursor] as i8;
    cursor += XM_SAMPLE_RELATIVE_NOTE_LEN;
    let reserved = bytes[cursor];
    cursor += XM_SAMPLE_RESERVED_LEN;
    let name = decode_fixed_text(&bytes[cursor..cursor + XM_SAMPLE_NAME_LEN]);
    let frame_count = sample_frame_count(length, sample_type);
    let loop_start_frames = sample_frame_count(loop_start, sample_type);
    let loop_length_frames = sample_frame_count(loop_length, sample_type);
    let decoded_data = empty_sample_data(sample_type);

    Ok(XmSampleHeader {
        index: sample_index,
        length,
        frame_count,
        loop_start,
        loop_start_frames,
        loop_length,
        loop_length_frames,
        volume_64,
        volume: vol64_to_255(volume_64),
        finetune,
        sample_type,
        loop_kind: sample_loop_kind(sample_type),
        panning,
        relative_note,
        reserved,
        name,
        data_offset: XM_EMPTY_SAMPLE_DATA_LEN as usize,
        data_end: XM_EMPTY_SAMPLE_DATA_LEN as usize,
        decoded_data,
    })
}

fn sample_frame_count(byte_len: u32, sample_type: u8) -> u32 {
    let sample_count = if is_16_bit_sample(sample_type) {
        byte_len / BYTES_PER_16_BIT_SAMPLE as u32
    } else {
        byte_len
    };

    if is_stereo_sample(sample_type) {
        sample_count / STEREO_CHANNEL_COUNT_U32
    } else {
        sample_count
    }
}

fn empty_sample_data(sample_type: u8) -> XmSampleData {
    if is_16_bit_sample(sample_type) {
        XmSampleData::Pcm16(Vec::new())
    } else {
        XmSampleData::Pcm8(Vec::new())
    }
}

fn decode_sample_data(bytes: &[u8], sample_type: u8) -> XmSampleData {
    if is_16_bit_sample(sample_type) {
        let values = decode_delta16(bytes);
        XmSampleData::Pcm16(if is_stereo_sample(sample_type) {
            mix_stereo_i16_to_mono(values)
        } else {
            values
        })
    } else {
        let values = decode_delta8(bytes);
        XmSampleData::Pcm8(if is_stereo_sample(sample_type) {
            mix_stereo_i8_to_mono(values)
        } else {
            values
        })
    }
}

fn decode_delta8(bytes: &[u8]) -> Vec<i8> {
    let mut accumulator = 0_i8;
    bytes
        .iter()
        .map(|&byte| {
            accumulator = accumulator.wrapping_add(byte as i8);
            accumulator
        })
        .collect()
}

fn decode_delta16(bytes: &[u8]) -> Vec<i16> {
    let mut accumulator = 0_i16;
    bytes
        .chunks_exact(BYTES_PER_16_BIT_SAMPLE)
        .map(|chunk| {
            let delta = i16::from_le_bytes([chunk[0], chunk[BYTE_1_OFFSET]]);
            accumulator = accumulator.wrapping_add(delta);
            accumulator
        })
        .collect()
}

fn is_16_bit_sample(sample_type: u8) -> bool {
    sample_type & XM_SAMPLE_16_BIT_FLAG == XM_SAMPLE_16_BIT_FLAG
}

fn is_stereo_sample(sample_type: u8) -> bool {
    sample_type & XM_SAMPLE_STEREO_FLAG == XM_SAMPLE_STEREO_FLAG
}

fn is_adpcm_sample(reserved: u8) -> bool {
    reserved == XM_SAMPLE_ADPCM_RESERVED
}

fn sample_loop_kind(sample_type: u8) -> SampleLoopKind {
    match sample_type & XM_SAMPLE_LOOP_MASK {
        XM_SAMPLE_LOOP_NONE => SampleLoopKind::None,
        XM_SAMPLE_LOOP_FORWARD => SampleLoopKind::Forward,
        XM_SAMPLE_LOOP_PING_PONG | XM_SAMPLE_LOOP_UNDEFINED => SampleLoopKind::PingPong,
        _ => unreachable!("loop-kind mask can only produce XM loop values"),
    }
}

fn mix_stereo_i8_to_mono(values: Vec<i8>) -> Vec<i8> {
    let frame_count = values.len() / STEREO_CHANNEL_COUNT;

    (0..frame_count)
        .map(|frame| {
            average_stereo_sample(values[frame] as i32, values[frame + frame_count] as i32)
                .clamp(i8::MIN as i32, i8::MAX as i32) as i8
        })
        .collect()
}

fn mix_stereo_i16_to_mono(values: Vec<i16>) -> Vec<i16> {
    let frame_count = values.len() / STEREO_CHANNEL_COUNT;

    (0..frame_count)
        .map(|frame| {
            average_stereo_sample(values[frame] as i32, values[frame + frame_count] as i32)
                .clamp(i16::MIN as i32, i16::MAX as i32) as i16
        })
        .collect()
}

fn average_stereo_sample(left: i32, right: i32) -> i32 {
    (left + right) >> STEREO_AVERAGE_SHIFT
}

fn ensure_instrument_range(
    bytes: &[u8],
    offset: usize,
    len: usize,
    instrument_index: usize,
    header: bool,
) -> XmResult<()> {
    let expected = offset + len;
    if expected <= bytes.len() {
        return Ok(());
    }

    if header {
        Err(XmParseError::InstrumentHeaderTooShort {
            instrument_index,
            expected,
            actual: bytes.len(),
        })
    } else {
        Err(XmParseError::InstrumentBodyTooShort {
            instrument_index,
            expected,
            actual: bytes.len(),
        })
    }
}

fn read_xm_slot(
    data: &[u8],
    data_cursor: &mut usize,
    pattern_index: usize,
    row: u16,
    channel: u16,
) -> XmResult<[u8; XM_CELL_FIELD_COUNT]> {
    let first = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
    let mut slot = [EMPTY_OPERAND; XM_CELL_FIELD_COUNT];

    if first & XM_CELL_PACKED_FLAG != EMPTY_OPERAND {
        for field in 0..XM_CELL_FIELD_COUNT {
            if first & (XM_FIELD_PRESENT_BIT_BASE << field) != EMPTY_OPERAND {
                slot[field] = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
            }
        }
    } else {
        slot[XM_NOTE_FIELD_INDEX] = first;
        for field in slot.iter_mut().skip(FIRST_UNPACKED_CELL_FIELD) {
            *field = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
        }
    }

    Ok(slot)
}

fn read_xm_slot_byte(
    data: &[u8],
    data_cursor: &mut usize,
    pattern_index: usize,
    row: u16,
    channel: u16,
) -> XmResult<u8> {
    if *data_cursor >= data.len() {
        return Err(XmParseError::PackedPatternCellTooShort {
            pattern_index,
            row,
            channel,
            expected: *data_cursor + BYTE_1_OFFSET,
            actual: data.len(),
        });
    }

    let byte = data[*data_cursor];
    *data_cursor += BYTE_1_OFFSET;
    Ok(byte)
}

fn normalize_xm_slot(mut slot: [u8; XM_CELL_FIELD_COUNT]) -> PatternCell {
    if !VALID_XM_EFFECTS.contains(&slot[XM_EFFECT_FIELD_INDEX]) {
        slot[XM_EFFECT_FIELD_INDEX] = EMPTY_EFFECT;
        slot[XM_OPERAND_FIELD_INDEX] = EMPTY_OPERAND;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_VOLUME
        || slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_GLOBAL_VOLUME
    {
        slot[XM_OPERAND_FIELD_INDEX] = vol64_to_255(slot[XM_OPERAND_FIELD_INDEX]);
    }

    if slot[XM_EFFECT_FIELD_INDEX] == EMPTY_EFFECT && slot[XM_OPERAND_FIELD_INDEX] != EMPTY_OPERAND
    {
        slot[XM_EFFECT_FIELD_INDEX] = INTERNAL_EFFECT_NONZERO_ARPEGGIO;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_EXTENDED {
        slot[XM_EFFECT_FIELD_INDEX] =
            (slot[XM_OPERAND_FIELD_INDEX] >> XM_NIBBLE_SHIFT) + INTERNAL_EFFECT_EXTENDED_BASE;
        slot[XM_OPERAND_FIELD_INDEX] &= XM_NIBBLE_MASK;
    }

    if slot[XM_EFFECT_FIELD_INDEX] == XM_EFFECT_EXTRA_FINE_PORTA {
        slot[XM_EFFECT_FIELD_INDEX] = (slot[XM_OPERAND_FIELD_INDEX] >> XM_NIBBLE_SHIFT)
            + INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE;
        slot[XM_OPERAND_FIELD_INDEX] &= XM_NIBBLE_MASK;
    }

    PatternCell {
        note: xm_note_to_core(slot[XM_NOTE_FIELD_INDEX]),
        instrument: slot[XM_INSTRUMENT_FIELD_INDEX],
        effects: vec![
            convert_xm_volume_effect(slot[XM_VOLUME_FIELD_INDEX]),
            EffectCommand {
                effect: slot[XM_EFFECT_FIELD_INDEX],
                operand: slot[XM_OPERAND_FIELD_INDEX],
            },
        ],
    }
}

fn xm_note_to_core(note: u8) -> Note {
    match note {
        XM_NOTE_EMPTY => Note::Empty,
        XM_NOTE_OFF => Note::Off,
        value => Note::Key(value),
    }
}

fn convert_xm_volume_effect(volume: u8) -> EffectCommand {
    let mut effect = EMPTY_EFFECT;
    let mut operand = EMPTY_OPERAND;

    if (XM_VOLUME_SET_MIN..=XM_VOLUME_SET_MAX).contains(&volume) {
        effect = XM_EFFECT_VOLUME;
        operand = vol64_to_255(volume - XM_VOLUME_SET_MIN);
    }

    if volume >= XM_VOLUME_COMMAND_MIN {
        let xm_effect = volume >> XM_NIBBLE_SHIFT;
        let xm_operand = volume & XM_NIBBLE_MASK;

        if xm_operand != EMPTY_OPERAND {
            match xm_effect {
                XM_VOLUME_SLIDE_DOWN => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand;
                }
                XM_VOLUME_SLIDE_UP => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_FINE_DOWN => {
                    effect = INTERNAL_EFFECT_FINE_VOLUME_SLIDE_DOWN;
                    operand = xm_operand;
                }
                XM_VOLUME_FINE_UP => {
                    effect = INTERNAL_EFFECT_FINE_VOLUME_SLIDE_UP;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_VIBRATO_SPEED => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_VIBRATO => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_PANNING => {
                    effect = INTERNAL_EFFECT_PANNING;
                    operand = pan15_to_255(xm_operand);
                }
                XM_VOLUME_PANNING_SLIDE_LEFT => {
                    effect = INTERNAL_EFFECT_PANNING_SLIDE;
                    operand = xm_operand;
                }
                XM_VOLUME_PANNING_SLIDE_RIGHT => {
                    effect = INTERNAL_EFFECT_PANNING_SLIDE;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_TONE_PORTAMENTO => {
                    effect = INTERNAL_EFFECT_TONE_PORTAMENTO;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                _ => {}
            }
        } else {
            match xm_effect {
                XM_VOLUME_VIBRATO => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand;
                }
                XM_VOLUME_SET_PANNING => {
                    effect = INTERNAL_EFFECT_PANNING;
                    operand = pan15_to_255(xm_operand);
                }
                XM_VOLUME_TONE_PORTAMENTO => {
                    effect = INTERNAL_EFFECT_TONE_PORTAMENTO;
                    operand = xm_operand;
                }
                _ => {}
            }
        }
    }

    EffectCommand { effect, operand }
}

fn vol64_to_255(volume: u8) -> u8 {
    (((volume.min(XM_VOLUME_MAX) as u32 * VOL64_TO_255_SCALE + VOL64_TO_255_ROUNDING)
        >> VOL64_TO_255_SHIFT)
        & BYTE_MASK) as u8
}

fn vol255_to_64(volume: u8) -> u8 {
    ((u16::from(volume) * u16::from(XM_VOLUME_MAX)) / CORE_VOLUME_MAX) as u8
}

fn pan15_to_255(panning: u8) -> u8 {
    if panning >= XM_PAN_COLUMN_MAX {
        FULL_PANNING
    } else {
        panning << XM_NIBBLE_SHIFT
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + BYTE_1_OFFSET]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + BYTE_1_OFFSET],
        bytes[offset + BYTE_2_OFFSET],
        bytes[offset + BYTE_3_OFFSET],
    ])
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + BYTE_2_OFFSET].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + BYTE_3_OFFSET + BYTE_1_OFFSET].copy_from_slice(&value.to_le_bytes());
}

fn write_fixed_text(bytes: &mut [u8], value: &str) {
    bytes.fill(ASCII_NUL);

    for (target, source) in bytes.iter_mut().zip(value.as_bytes()) {
        *target = *source;
    }
}

fn decode_fixed_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|&byte| byte > ASCII_CONTROL_MAX)
        .map(|index| index + TEXT_INDEX_TO_LEN_OFFSET)
        .unwrap_or(ASCII_NUL as usize);

    bytes[..end]
        .iter()
        .map(|&byte| {
            if byte == ASCII_NUL || byte < ASCII_CONTROL_MAX || byte > ASCII_DELETE {
                ' '
            } else {
                byte as char
            }
        })
        .collect()
}
