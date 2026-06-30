//! XM file support for RustyTracker.
//!
//! This crate starts read-only. It will grow toward a full parser/writer only
//! through fixture-backed tests.

mod error;
mod model;
mod parser;
mod writer;

use rustytracker_core::{
    INTERNAL_EFFECT_EXTENDED_BASE,
};

pub use error::{XmParseError, XmResult, XmSampleField, XmWriteError, XmWriteResult};
pub use model::{
    XmEnvelope, XmEnvelopePoint, XmInstrument, XmInstrumentSection, XmModuleHeader,
    XmPatternHeader, XmSampleData, XmSampleHeader,
};

pub use parser::{
    parse_xm_header, parse_xm_pattern_headers, decode_xm_patterns, decode_xm_pattern,
    parse_xm_module, parse_xm_instruments,
};
pub use writer::{
    write_xm_header, write_xm_module, write_xm_patterns, write_xm_instruments,
};

pub const XM_HEADER_SIGNATURE_LENGTH: usize = 17;
pub const XM_HEADER_SIGNATURE: &[u8; XM_HEADER_SIGNATURE_LENGTH] = b"Extended Module: ";
pub(crate) const XM_MARKER: u8 = 0x1a;
pub(crate) const TITLE_OFFSET: usize = XM_HEADER_SIGNATURE_LENGTH;
pub(crate) const TITLE_LEN: usize = 20;
pub(crate) const MARKER_OFFSET: usize = 37;
pub(crate) const TRACKER_OFFSET: usize = 38;
pub(crate) const TRACKER_LEN: usize = 20;
pub(crate) const VERSION_OFFSET: usize = 58;
pub(crate) const HEADER_SIZE_OFFSET: usize = 60;
pub(crate) const HEADER_FIELDS_OFFSET: usize = 64;
pub(crate) const ORDER_TABLE_OFFSET: usize = 80;
pub(crate) const XM_ORDER_TABLE_LEN: usize = 256;
pub(crate) const XM_MIN_HEADER_BYTES: usize = ORDER_TABLE_OFFSET + XM_ORDER_TABLE_LEN;
pub(crate) const XM_EXPANDED_EFFECT_SLOTS: u8 = 2;
pub(crate) const XM_VERSION_1_02: u16 = 0x0102;
pub(crate) const XM_VERSION_1_03: u16 = 0x0103;
pub(crate) const XM_VERSION_1_04: u16 = 0x0104;
pub(crate) const XM_WRITER_TRACKER_NAME: &str = "RustyTracker";
pub(crate) const XM_WRITER_HEADER_SIZE: u32 = 276;
pub(crate) const XM_WRITER_AMIGA_FLAGS: u16 = 0x0000;
pub(crate) const XM_WRITER_EMPTY_ORDER: u8 = 0;
pub(crate) const XM_WRITER_PATTERN_HEADER_LEN: u32 = XM_PATTERN_HEADER_LEN as u32;
pub(crate) const XM_WRITER_PATTERN_PACKING_TYPE: u8 = 0;
pub(crate) const XM_WRITER_EMPTY_VOLUME_COLUMN: u8 = 0;
pub(crate) const XM_WRITER_SINGLE_EFFECT_SLOT_COUNT: usize = 1;
pub(crate) const XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE: u32 = XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE;
pub(crate) const XM_WRITER_INSTRUMENT_HEADER_SIZE: u32 =
    XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE + XM_INSTRUMENT_EXTENSION_MAX_LEN as u32;
pub(crate) const XM_WRITER_EMPTY_INSTRUMENT_SAMPLE_COUNT: usize = 0;
pub(crate) const XM_WRITER_INSTRUMENT_TYPE: u8 = 0;
pub(crate) const XM_WRITER_SAMPLE_HEADER_SIZE: u32 = XM_SAMPLE_HEADER_LEN as u32;
pub(crate) const XM_WRITER_EMPTY_SAMPLE_BYTE_LEN: u32 = 0;
pub(crate) const XM_WRITER_SAMPLE_RESERVED: u8 = 0;
pub(crate) const XM_WRITER_EMPTY_ENVELOPE_FRAME: u16 = 0;
pub(crate) const XM_WRITER_EMPTY_ENVELOPE_VALUE: u16 = 0;
pub(crate) const XM_WRITER_DELTA_INITIAL_8: i8 = 0;
pub(crate) const XM_WRITER_DELTA_INITIAL_16: i16 = 0;
pub(crate) const U32_FIELD_MAX: u64 = u32::MAX as u64;
pub(crate) const XM_HEADER_FIELD_STEP: usize = 2;
pub(crate) const XM_RESTART_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP;
pub(crate) const XM_CHANNELS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 2;
pub(crate) const XM_PATTERNS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 3;
pub(crate) const XM_INSTRUMENTS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 4;
pub(crate) const XM_FLAGS_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 5;
pub(crate) const XM_TICK_SPEED_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 6;
pub(crate) const XM_BPM_FIELD_OFFSET: usize = HEADER_FIELDS_OFFSET + XM_HEADER_FIELD_STEP * 7;
pub(crate) const XM_LINEAR_FREQUENCY_FLAG: u16 = 0x0001;
pub(crate) const XM_1_02_PATTERN_HEADER_LEN: usize = 8;
pub(crate) const XM_PATTERN_HEADER_LEN: usize = 9;
pub(crate) const XM_PATTERN_TYPE_OFFSET: usize = 4;
pub(crate) const XM_1_02_PATTERN_ROWS_OFFSET: usize = 5;
pub(crate) const XM_1_02_PATTERN_DATA_LEN_OFFSET: usize = 6;
pub(crate) const XM_1_02_ROW_COUNT_BASE: u16 = 1;
pub(crate) const XM_PATTERN_ROWS_OFFSET: usize = 5;
pub(crate) const XM_PATTERN_DATA_LEN_OFFSET: usize = 7;
pub(crate) const XM_CELL_FIELD_COUNT: usize = 5;
pub(crate) const XM_CELL_PACKED_FLAG: u8 = 0x80;
pub(crate) const XM_NOTE_FIELD_INDEX: usize = 0;
pub(crate) const XM_INSTRUMENT_FIELD_INDEX: usize = 1;
pub(crate) const XM_VOLUME_FIELD_INDEX: usize = 2;
pub(crate) const XM_EFFECT_FIELD_INDEX: usize = 3;
pub(crate) const XM_OPERAND_FIELD_INDEX: usize = 4;
pub(crate) const XM_FIELD_PRESENT_BIT_BASE: u8 = 1;
pub(crate) const FIRST_UNPACKED_CELL_FIELD: usize = 1;
pub(crate) const XM_NOTE_EMPTY: u8 = 0;
pub(crate) const XM_NOTE_OFF: u8 = 97;
pub(crate) const EMPTY_EFFECT: u8 = 0;
pub(crate) const EMPTY_OPERAND: u8 = 0;
pub(crate) const ASCII_CONTROL_MAX: u8 = 32;
pub(crate) const ASCII_DELETE: u8 = 127;
pub(crate) const ASCII_NUL: u8 = 0;
pub(crate) const TEXT_INDEX_TO_LEN_OFFSET: usize = 1;
pub(crate) const BYTE_1_OFFSET: usize = 1;
pub(crate) const BYTE_2_OFFSET: usize = 2;
pub(crate) const BYTE_3_OFFSET: usize = 3;
pub(crate) const VALID_XM_EFFECTS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 21, 25, 27, 29, 33,
];
pub(crate) const XM_EFFECT_PROTRACKER_MIN: u8 = 0x01;
pub(crate) const XM_EFFECT_PROTRACKER_MAX: u8 = 0x11;
pub(crate) const XM_EFFECT_VOLUME: u8 = 0x0c;
pub(crate) const XM_EFFECT_GLOBAL_VOLUME: u8 = 0x10;
pub(crate) const XM_EFFECT_EXTRA_FINE_PORTA: u8 = 0x21;
pub(crate) const XM_VOLUME_SET_MIN: u8 = 0x10;
pub(crate) const XM_VOLUME_SET_MAX: u8 = 0x50;
pub(crate) const XM_VOLUME_COMMAND_MIN: u8 = 0x60;
pub(crate) const XM_VOLUME_SLIDE_DOWN: u8 = 0x6;
pub(crate) const XM_VOLUME_SLIDE_UP: u8 = 0x7;
pub(crate) const XM_VOLUME_FINE_DOWN: u8 = 0x8;
pub(crate) const XM_VOLUME_FINE_UP: u8 = 0x9;
pub(crate) const XM_VOLUME_SET_VIBRATO_SPEED: u8 = 0xA;
pub(crate) const XM_VOLUME_VIBRATO: u8 = 0xB;
pub(crate) const XM_VOLUME_SET_PANNING: u8 = 0xC;
pub(crate) const XM_VOLUME_PANNING_SLIDE_LEFT: u8 = 0xD;
pub(crate) const XM_VOLUME_PANNING_SLIDE_RIGHT: u8 = 0xE;
pub(crate) const XM_VOLUME_TONE_PORTAMENTO: u8 = 0xF;
pub(crate) const INTERNAL_EFFECT_VOLUME_SLIDE: u8 = 0x0a;
pub(crate) const XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND: u8 = 0x0a;
pub(crate) const XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND: u8 = 0x0b;
pub(crate) const INTERNAL_EFFECT_FINE_VOLUME_SLIDE_UP: u8 =
    INTERNAL_EFFECT_EXTENDED_BASE + XM_EXTENDED_FINE_VOLUME_SLIDE_UP_COMMAND;
pub(crate) const INTERNAL_EFFECT_FINE_VOLUME_SLIDE_DOWN: u8 =
    INTERNAL_EFFECT_EXTENDED_BASE + XM_EXTENDED_FINE_VOLUME_SLIDE_DOWN_COMMAND;
pub(crate) const INTERNAL_EFFECT_VIBRATO_COMPAT: u8 = 0x04;
pub(crate) const INTERNAL_EFFECT_PANNING: u8 = 0x08;
pub(crate) const INTERNAL_EFFECT_PANNING_SLIDE: u8 = 0x19;
pub(crate) const INTERNAL_EFFECT_TONE_PORTAMENTO: u8 = 0x03;
pub(crate) const XM_NIBBLE_SHIFT: u8 = 4;
pub(crate) const XM_NIBBLE_MASK: u8 = 0x0f;
pub(crate) const XM_VOLUME_MAX: u8 = 64;
pub(crate) const VOL64_TO_255_SCALE: u32 = 261_120;
pub(crate) const VOL64_TO_255_ROUNDING: u32 = 65_535;
pub(crate) const VOL64_TO_255_SHIFT: u32 = 16;
pub(crate) const CORE_VOLUME_MAX: u16 = 255;
pub(crate) const BYTE_MASK: u32 = 0xff;
pub(crate) const XM_PAN_COLUMN_MAX: u8 = 0x0f;
pub(crate) const FULL_PANNING: u8 = 0xff;
pub(crate) const XM_INSTRUMENT_SIZE_LEN: usize = 4;
pub(crate) const XM_INSTRUMENT_SHORT_SIZE_MIN: u32 = 4;
pub(crate) const XM_INSTRUMENT_NO_EXTENSION_MAX_SIZE: u32 = 29;
pub(crate) const XM_INSTRUMENT_SHORT_BUFFER_LEN: usize = 29;
pub(crate) const XM_INSTRUMENT_FIXED_FIELDS_LEN: usize = 25;
pub(crate) const XM_INSTRUMENT_NAME_LEN: usize = 22;
pub(crate) const XM_INSTRUMENT_TYPE_OFFSET: usize = 22;
pub(crate) const XM_INSTRUMENT_SAMPLE_COUNT_OFFSET: usize = 23;
pub(crate) const XM_INSTRUMENT_BASE_WITH_SAMPLE_HEADER_SIZE: u32 = 33;
pub(crate) const XM_INSTRUMENT_EXTENSION_MAX_LEN: usize = 230;
pub(crate) const XM_NOTE_SAMPLE_MAP_LEN: usize = 96;
pub(crate) const XM_ENVELOPE_POINT_COUNT: usize = 12;
pub(crate) const XM_ENVELOPE_POINT_BYTES: usize = 4;
pub(crate) const XM_ENVELOPE_X_OFFSET: usize = 0;
pub(crate) const XM_ENVELOPE_Y_OFFSET: usize = 2;
pub(crate) const XM_ENVELOPE_VALUE_SHIFT: u16 = 2;
pub(crate) const XM_SAMPLE_HEADER_SIZE_LEN: usize = 4;
pub(crate) const XM_SAMPLE_HEADER_LEN: usize = 40;
pub(crate) const XM_SAMPLE_NAME_LEN: usize = 22;
pub(crate) const XM_ENVELOPE_POINT_COUNT_MAX: u8 = XM_ENVELOPE_POINT_COUNT as u8;
pub(crate) const XM_VIBRATO_DEPTH_SHIFT: u8 = 1;
pub(crate) const XM_VOLUME_FADEOUT_SHIFT: u16 = 1;
pub(crate) const XM_SAMPLE_LENGTH_LEN: usize = 4;
pub(crate) const XM_SAMPLE_LOOP_START_LEN: usize = 4;
pub(crate) const XM_SAMPLE_LOOP_LENGTH_LEN: usize = 4;
pub(crate) const XM_SAMPLE_VOLUME_LEN: usize = 1;
pub(crate) const XM_SAMPLE_FINETUNE_LEN: usize = 1;
pub(crate) const XM_SAMPLE_TYPE_LEN: usize = 1;
pub(crate) const XM_SAMPLE_PANNING_LEN: usize = 1;
pub(crate) const XM_SAMPLE_RELATIVE_NOTE_LEN: usize = 1;
pub(crate) const XM_SAMPLE_RESERVED_LEN: usize = 1;
pub(crate) const XM_EMPTY_SAMPLE_DATA_LEN: u32 = 0;
pub(crate) const XM_SAMPLE_8_BIT_FLAG: u8 = 0x00;
pub(crate) const XM_SAMPLE_16_BIT_FLAG: u8 = 0x10;
pub(crate) const XM_SAMPLE_LOOP_MASK: u8 = 0x03;
pub(crate) const XM_SAMPLE_NON_LOOP_TYPE_MASK: u8 = !XM_SAMPLE_LOOP_MASK;
pub(crate) const XM_SAMPLE_LOOP_NONE: u8 = 0x00;
pub(crate) const XM_SAMPLE_LOOP_FORWARD: u8 = 0x01;
pub(crate) const XM_SAMPLE_LOOP_PING_PONG: u8 = 0x02;
pub(crate) const XM_SAMPLE_LOOP_UNDEFINED: u8 = 0x03;
pub(crate) const XM_SAMPLE_STEREO_FLAG: u8 = 0x20;
pub(crate) const XM_SAMPLE_ADPCM_RESERVED: u8 = 0xad;
pub(crate) const BYTES_PER_8_BIT_SAMPLE: usize = 1;
pub(crate) const BYTES_PER_16_BIT_SAMPLE: usize = 2;
pub(crate) const STEREO_CHANNEL_COUNT: usize = 2;
pub(crate) const STEREO_CHANNEL_COUNT_U32: u32 = STEREO_CHANNEL_COUNT as u32;
pub(crate) const STEREO_AVERAGE_SHIFT: u8 = 1;
pub(crate) const XM_ORDER_REFERENCE_PATTERN_ROWS: u16 = rustytracker_core::DEFAULT_PATTERN_ROWS;
pub(crate) const U16_MAX_AS_USIZE: usize = u16::MAX as usize;
