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
const XM_MIN_HEADER_BYTES: usize = ORDER_TABLE_OFFSET + 256;
const XM_EXPANDED_EFFECT_SLOTS: u8 = 2;
const VALID_XM_EFFECTS: &[u8] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 20, 21, 25, 27, 29, 33,
];

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
    if !matches!(version, 0x0102 | 0x0103 | 0x0104) {
        return Err(XmParseError::UnsupportedVersion(version));
    }

    let header_size = read_u32(bytes, HEADER_SIZE_OFFSET);
    let song_length = read_u16(bytes, HEADER_FIELDS_OFFSET);
    let restart_position = read_u16(bytes, HEADER_FIELDS_OFFSET + 2);
    let channel_count = read_u16(bytes, HEADER_FIELDS_OFFSET + 4);
    let pattern_count = read_u16(bytes, HEADER_FIELDS_OFFSET + 6);
    let instrument_count = read_u16(bytes, HEADER_FIELDS_OFFSET + 8);
    let flags = read_u16(bytes, HEADER_FIELDS_OFFSET + 10);
    let default_tick_speed = read_u16(bytes, HEADER_FIELDS_OFFSET + 12);
    let default_bpm = read_u16(bytes, HEADER_FIELDS_OFFSET + 14);

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
        frequency_table: if flags & 1 == 1 {
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
    let fixed_pattern_header_len = if header.version == 0x0102 { 8 } else { 9 };
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
        let packing_type = bytes[offset + 4];
        let (row_count, packed_data_len) = if header.version == 0x0102 {
            (bytes[offset + 5] as u16 + 1, read_u16(bytes, offset + 6))
        } else {
            (read_u16(bytes, offset + 5), read_u16(bytes, offset + 7))
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
) -> XmResult<[u8; 5]> {
    let first = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
    let mut slot = [0; 5];

    if first & 0x80 != 0 {
        for field in 0..5 {
            if first & (1 << field) != 0 {
                slot[field] = read_xm_slot_byte(data, data_cursor, pattern_index, row, channel)?;
            }
        }
    } else {
        slot[0] = first;
        for field in slot.iter_mut().skip(1) {
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
            expected: *data_cursor + 1,
            actual: data.len(),
        });
    }

    let byte = data[*data_cursor];
    *data_cursor += 1;
    Ok(byte)
}

fn normalize_xm_slot(mut slot: [u8; 5]) -> PatternCell {
    if !VALID_XM_EFFECTS.contains(&slot[3]) {
        slot[3] = 0;
        slot[4] = 0;
    }

    if slot[3] == 0x0c || slot[3] == 0x10 {
        slot[4] = vol64_to_255(slot[4]);
    }

    if slot[3] == 0 && slot[4] != 0 {
        slot[3] = 0x20;
    }

    if slot[3] == 0x0e {
        slot[3] = (slot[4] >> 4) + 0x30;
        slot[4] &= 0x0f;
    }

    if slot[3] == 0x21 {
        slot[3] = (slot[4] >> 4) + 0x40;
        slot[4] &= 0x0f;
    }

    PatternCell {
        note: xm_note_to_core(slot[0]),
        instrument: slot[1],
        effects: vec![
            convert_xm_volume_effect(slot[2]),
            EffectCommand {
                effect: slot[3],
                operand: slot[4],
            },
        ],
    }
}

fn xm_note_to_core(note: u8) -> Note {
    match note {
        0 => Note::Empty,
        97 => Note::Off,
        value => Note::Key(value),
    }
}

fn convert_xm_volume_effect(volume: u8) -> EffectCommand {
    let mut effect = 0;
    let mut operand = 0;

    if (0x10..=0x50).contains(&volume) {
        effect = 0x0c;
        operand = vol64_to_255(volume - 0x10);
    }

    if volume >= 0x60 {
        let xm_effect = volume >> 4;
        let xm_operand = volume & 0x0f;

        if xm_operand != 0 {
            match xm_effect {
                0x6 => {
                    effect = 0x0a;
                    operand = xm_operand;
                }
                0x7 => {
                    effect = 0x0a;
                    operand = xm_operand << 4;
                }
                0x8 => {
                    effect = 0x3b;
                    operand = xm_operand;
                }
                0x9 => {
                    effect = 0x3a;
                    operand = xm_operand;
                }
                0xA => {
                    effect = 0x04;
                    operand = xm_operand << 4;
                }
                0xB => {
                    effect = 0x04;
                    operand = xm_operand;
                }
                0xC => {
                    effect = 0x08;
                    operand = pan15_to_255(xm_operand);
                }
                0xD => {
                    effect = 0x19;
                    operand = xm_operand;
                }
                0xE => {
                    effect = 0x19;
                    operand = xm_operand << 4;
                }
                0xF => {
                    effect = 0x03;
                    operand = xm_operand << 4;
                }
                _ => {}
            }
        } else {
            match xm_effect {
                0xB => {
                    effect = 0x04;
                    operand = xm_operand;
                }
                0xC => {
                    effect = 0x08;
                    operand = pan15_to_255(xm_operand);
                }
                0xF => {
                    effect = 0x03;
                    operand = xm_operand;
                }
                _ => {}
            }
        }
    }

    EffectCommand { effect, operand }
}

fn vol64_to_255(volume: u8) -> u8 {
    (((volume.min(64) as u32 * 261_120 + 65_535) >> 16) & 0xff) as u8
}

fn pan15_to_255(panning: u8) -> u8 {
    if panning >= 0x0f {
        0xff
    } else {
        panning << 4
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn decode_fixed_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .rposition(|&byte| byte > 32)
        .map(|index| index + 1)
        .unwrap_or(0);

    bytes[..end]
        .iter()
        .map(|&byte| {
            if byte == 0 || byte < 32 || byte > 127 {
                ' '
            } else {
                byte as char
            }
        })
        .collect()
}
