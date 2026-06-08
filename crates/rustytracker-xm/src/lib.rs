//! XM file support for RustyTracker.
//!
//! This crate starts read-only. It will grow toward a full parser/writer only
//! through fixture-backed tests.

use rustytracker_core::{EffectCommand, FrequencyTable, Note, Pattern, PatternCell};

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
const XM_EFFECT_VOLUME: u8 = 0x0c;
const XM_EFFECT_GLOBAL_VOLUME: u8 = 0x10;
const XM_EFFECT_EXTENDED: u8 = 0x0e;
const XM_EFFECT_EXTRA_FINE_PORTA: u8 = 0x21;
const INTERNAL_EFFECT_NONZERO_ARPEGGIO: u8 = 0x20;
const INTERNAL_EFFECT_EXTENDED_BASE: u8 = 0x30;
const INTERNAL_EFFECT_EXTRA_FINE_PORTA_BASE: u8 = 0x40;
const XM_VOLUME_SET_MIN: u8 = 0x10;
const XM_VOLUME_SET_MAX: u8 = 0x50;
const XM_VOLUME_COMMAND_MIN: u8 = 0x60;
const XM_VOLUME_FINE_DOWN: u8 = 0x6;
const XM_VOLUME_FINE_UP: u8 = 0x7;
const XM_VOLUME_SET_VIBRATO_SPEED: u8 = 0x8;
const XM_VOLUME_VIBRATO: u8 = 0x9;
const XM_VOLUME_SET_PANNING: u8 = 0xC;
const XM_VOLUME_PANNING_SLIDE_LEFT: u8 = 0xD;
const XM_VOLUME_PANNING_SLIDE_RIGHT: u8 = 0xE;
const XM_VOLUME_TONE_PORTAMENTO: u8 = 0xF;
const XM_VOLUME_VIBRATO_SPEED_DEPTH: u8 = 0xA;
const XM_VOLUME_VIBRATO_DEPTH_SPEED: u8 = 0xB;
const INTERNAL_EFFECT_VOLUME_SLIDE: u8 = 0x0a;
const INTERNAL_EFFECT_SET_VIBRATO_SPEED: u8 = 0x3b;
const INTERNAL_EFFECT_VIBRATO: u8 = 0x3a;
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
const BYTE_MASK: u32 = 0xff;
const XM_PAN_COLUMN_MAX: u8 = 0x0f;
const FULL_PANNING: u8 = 0xff;

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
}

pub type XmResult<T> = Result<T, XmParseError>;

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
                XM_VOLUME_FINE_DOWN => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand;
                }
                XM_VOLUME_FINE_UP => {
                    effect = INTERNAL_EFFECT_VOLUME_SLIDE;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_SET_VIBRATO_SPEED => {
                    effect = INTERNAL_EFFECT_SET_VIBRATO_SPEED;
                    operand = xm_operand;
                }
                XM_VOLUME_VIBRATO => {
                    effect = INTERNAL_EFFECT_VIBRATO;
                    operand = xm_operand;
                }
                XM_VOLUME_VIBRATO_SPEED_DEPTH => {
                    effect = INTERNAL_EFFECT_VIBRATO_COMPAT;
                    operand = xm_operand << XM_NIBBLE_SHIFT;
                }
                XM_VOLUME_VIBRATO_DEPTH_SPEED => {
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
                XM_VOLUME_VIBRATO_DEPTH_SPEED => {
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
