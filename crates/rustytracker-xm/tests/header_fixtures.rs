use std::fs;
use std::path::PathBuf;

use rustytracker_core::{EffectCommand, FrequencyTable, Note};
use rustytracker_xm::{
    decode_xm_pattern, decode_xm_patterns, parse_xm_header, parse_xm_pattern_headers,
    XmModuleHeader, XmParseError, XmPatternHeader,
};

const XM_TEST_SIGNATURE: &[u8; 17] = b"Extended Module: ";
const XM_TEST_MARKER: u8 = 0x1a;
const XM_TEST_MARKER_OFFSET: usize = 37;
const XM_TEST_VERSION_OFFSET: usize = 58;
const XM_TEST_HEADER_SIZE_OFFSET: usize = 60;
const XM_TEST_HEADER_FIELDS_OFFSET: usize = 64;
const XM_TEST_CHANNELS_FIELD_OFFSET: usize = 68;
const XM_TEST_PATTERNS_FIELD_OFFSET: usize = 70;
const XM_TEST_INSTRUMENTS_FIELD_OFFSET: usize = 72;
const XM_TEST_TICK_SPEED_FIELD_OFFSET: usize = 76;
const XM_TEST_BPM_FIELD_OFFSET: usize = 78;
const XM_TEST_HEADER_SIZE: u32 = 276;
const XM_TEST_HEADER_BYTES: usize = 336;
const XM_TEST_VERSION: u16 = 0x0104;
const XM_TEST_DEFAULT_ORDERS: u16 = 1;
const XM_TEST_DEFAULT_CHANNELS: u16 = 4;
const XM_TEST_DEFAULT_PATTERNS: u16 = 0;
const XM_TEST_DEFAULT_INSTRUMENTS: u16 = 0;
const XM_TEST_DEFAULT_SPEED: u16 = 6;
const XM_TEST_DEFAULT_BPM: u16 = 125;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XM_TEST_SINGLE_ROW: u16 = 1;
const XM_TEST_FOUR_CHANNELS: u16 = 4;
const XM_TEST_NOTE: u8 = 49;
const XM_TEST_INSTRUMENT: u8 = 1;
const XM_TEST_EMPTY_EFFECT: u8 = 0;
const XM_TEST_EMPTY_OPERAND: u8 = 0;
const XM_FINE_VOLUME_SLIDE_DOWN_COLUMN: u8 = 0x8d;
const XM_FINE_VOLUME_SLIDE_UP_COLUMN: u8 = 0x9c;
const XM_ZERO_FINE_VOLUME_SLIDE_DOWN_COLUMN: u8 = 0x80;
const XM_ZERO_FINE_VOLUME_SLIDE_UP_COLUMN: u8 = 0x90;
const INTERNAL_FINE_VOLUME_SLIDE_UP_EFFECT: u8 = 0x3a;
const INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT: u8 = 0x3b;
const XM_FINE_VOLUME_SLIDE_UP_OPERAND: u8 = 0x0c;
const XM_FINE_VOLUME_SLIDE_DOWN_OPERAND: u8 = 0x0d;

#[derive(Debug)]
struct ExpectedHeader {
    file_name: &'static str,
    title: &'static str,
    song_length: u16,
    restart_position: u16,
    channel_count: u16,
    pattern_count: u16,
    instrument_count: u16,
    default_tick_speed: u16,
    default_bpm: u16,
    first_orders: &'static [u8],
    first_pattern_rows: &'static [u16],
    first_pattern_packed_lengths: &'static [u16],
    unique_row_counts: &'static [u16],
    packed_pattern_data_total: usize,
    empty_pattern_count: usize,
    decoded_cell_count: usize,
    non_empty_cell_count: usize,
    expanded_pattern_checksum: u64,
    first_non_empty_cell: (usize, u16, u16, [u8; 6]),
}

const FIXTURES: &[ExpectedHeader] = &[
    ExpectedHeader {
        file_name: "milky.xm",
        title: "milk in veins",
        song_length: 17,
        restart_position: 0,
        channel_count: 10,
        pattern_count: 17,
        instrument_count: 7,
        default_tick_speed: 6,
        default_bpm: 133,
        first_orders: &[1, 2, 0, 3, 4, 5, 6, 7],
        first_pattern_rows: &[64, 64, 64, 64, 64, 64, 64, 64],
        first_pattern_packed_lengths: &[746, 751, 742, 746, 885, 939, 939, 939],
        unique_row_counts: &[64],
        packed_pattern_data_total: 16_743,
        empty_pattern_count: 0,
        decoded_cell_count: 10_880,
        non_empty_cell_count: 2_756,
        expanded_pattern_checksum: 0x8706_cb07_f884_9bc2,
        first_non_empty_cell: (0, 0, 0, [49, 1, 0, 0, 0, 0]),
    },
    ExpectedHeader {
        file_name: "slumberjack.xm",
        title: "slumberjack",
        song_length: 42,
        restart_position: 0,
        channel_count: 8,
        pattern_count: 27,
        instrument_count: 7,
        default_tick_speed: 3,
        default_bpm: 84,
        first_orders: &[5, 5, 5, 5, 6, 0, 1, 2],
        first_pattern_rows: &[32, 32, 32, 32, 32, 12, 32, 32],
        first_pattern_packed_lengths: &[376, 376, 380, 388, 453, 128, 363, 457],
        unique_row_counts: &[12, 32],
        packed_pattern_data_total: 12_890,
        empty_pattern_count: 0,
        decoded_cell_count: 6_592,
        non_empty_cell_count: 2_778,
        expanded_pattern_checksum: 0xe7f7_954d_589a_86bb,
        first_non_empty_cell: (0, 0, 0, [41, 1, 8, 128, 0, 0]),
    },
    ExpectedHeader {
        file_name: "sv_ttt.xm",
        title: "The Titan Turrican",
        song_length: 16,
        restart_position: 2,
        channel_count: 6,
        pattern_count: 17,
        instrument_count: 44,
        default_tick_speed: 4,
        default_bpm: 135,
        first_orders: &[9, 0, 1, 2, 3, 4, 5, 6],
        first_pattern_rows: &[64, 64, 64, 64, 64, 64, 64, 64],
        first_pattern_packed_lengths: &[703, 869, 761, 845, 751, 670, 744, 768],
        unique_row_counts: &[64],
        packed_pattern_data_total: 12_677,
        empty_pattern_count: 0,
        decoded_cell_count: 6_528,
        non_empty_cell_count: 3_136,
        expanded_pattern_checksum: 0xf594_4438_b284_a182,
        first_non_empty_cell: (0, 0, 1, [51, 6, 25, 32, 32, 88]),
    },
    ExpectedHeader {
        file_name: "theday.xm",
        title: "the day they landed",
        song_length: 45,
        restart_position: 0,
        channel_count: 8,
        pattern_count: 42,
        instrument_count: 7,
        default_tick_speed: 8,
        default_bpm: 160,
        first_orders: &[0, 2, 3, 4, 5, 1, 6, 9],
        first_pattern_rows: &[64, 64, 64, 64, 64, 64, 64, 64],
        first_pattern_packed_lengths: &[1_353, 1_334, 1_373, 1_446, 1_423, 1_348, 1_325, 1_324],
        unique_row_counts: &[64],
        packed_pattern_data_total: 43_699,
        empty_pattern_count: 0,
        decoded_cell_count: 21_504,
        non_empty_cell_count: 11_952,
        expanded_pattern_checksum: 0x8b8c_76ca_cae2_6ae1,
        first_non_empty_cell: (0, 0, 0, [63, 1, 12, 0, 15, 8]),
    },
    ExpectedHeader {
        file_name: "universalnetwork2_real.xm",
        title: " universal network 2",
        song_length: 31,
        restart_position: 0,
        channel_count: 6,
        pattern_count: 32,
        instrument_count: 16,
        default_tick_speed: 3,
        default_bpm: 125,
        first_orders: &[12, 13, 14, 15, 1, 17, 16, 2],
        first_pattern_rows: &[64, 64, 64, 64, 64, 64, 64, 64],
        first_pattern_packed_lengths: &[429, 608, 717, 840, 861, 873, 924, 774],
        unique_row_counts: &[1, 54, 64],
        packed_pattern_data_total: 22_222,
        empty_pattern_count: 0,
        decoded_cell_count: 11_472,
        non_empty_cell_count: 5_417,
        expanded_pattern_checksum: 0x1759_ebc9_64b5_f77b,
        first_non_empty_cell: (0, 0, 0, [49, 11, 0, 0, 15, 3]),
    },
];

#[test]
fn parses_milkytracker_bundled_xm_headers() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture.file_name)).unwrap();
        let header = parse_xm_header(&bytes).unwrap();

        assert_eq!(header.title, fixture.title, "{}", fixture.file_name);
        assert_eq!(header.tracker_name, "MilkyTracker", "{}", fixture.file_name);
        assert_eq!(header.version, 0x0104, "{}", fixture.file_name);
        assert_eq!(header.header_size, 276, "{}", fixture.file_name);
        assert_eq!(
            header.song_length, fixture.song_length,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.restart_position, fixture.restart_position,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.channel_count, fixture.channel_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.pattern_count, fixture.pattern_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.instrument_count, fixture.instrument_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(header.flags, 1, "{}", fixture.file_name);
        assert_eq!(
            header.frequency_table,
            FrequencyTable::Linear,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.default_tick_speed, fixture.default_tick_speed,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            header.default_bpm, fixture.default_bpm,
            "{}",
            fixture.file_name
        );
        assert_eq!(header.orders.len(), fixture.song_length as usize);
        assert_eq!(
            &header.orders[..fixture.first_orders.len()],
            fixture.first_orders,
            "{}",
            fixture.file_name
        );
    }
}

#[test]
fn decodes_milkytracker_bundled_xm_patterns_to_expanded_cells() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture.file_name)).unwrap();
        let header = parse_xm_header(&bytes).unwrap();
        let patterns = decode_xm_patterns(&bytes, &header).unwrap();
        let stats = decoded_pattern_stats(&patterns);

        assert_eq!(patterns.len(), fixture.pattern_count as usize);
        assert_eq!(stats.cell_count, fixture.decoded_cell_count);
        assert_eq!(stats.non_empty_count, fixture.non_empty_cell_count);
        assert_eq!(
            stats.checksum, fixture.expanded_pattern_checksum,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            stats.first_non_empty.unwrap(),
            fixture.first_non_empty_cell,
            "{}",
            fixture.file_name
        );
    }
}

#[test]
fn decodes_packed_and_unpacked_cells_with_milkytracker_normalization() {
    let header = synthetic_header(2, 1);
    let bytes = [
        49, 1, 0x20, 0x0e, 0xa7, // unpacked cell
        0x9f, 97, 2, 0xc8, 0x0c, 64, // packed cell with all fields
    ];
    let pattern_header = synthetic_pattern_header(1, bytes.len() as u16);
    let pattern = decode_xm_pattern(&bytes, &header, &pattern_header).unwrap();

    let first = pattern.cell(0, 0).unwrap();
    assert_eq!(first.note, Note::Key(49));
    assert_eq!(first.instrument, 1);
    assert_eq!(
        first.effects,
        vec![
            EffectCommand {
                effect: 0x0c,
                operand: 64,
            },
            EffectCommand {
                effect: 0x3a,
                operand: 7,
            },
        ]
    );

    let second = pattern.cell(1, 0).unwrap();
    assert_eq!(second.note, Note::Off);
    assert_eq!(second.instrument, 2);
    assert_eq!(
        second.effects,
        vec![
            EffectCommand {
                effect: 0x08,
                operand: 128,
            },
            EffectCommand {
                effect: 0x0c,
                operand: 255,
            },
        ]
    );
}

#[test]
fn decodes_fine_volume_slides_from_xm_volume_column() {
    let header = synthetic_header(XM_TEST_FOUR_CHANNELS, XM_TEST_SINGLE_ROW);
    let bytes = [
        XM_TEST_NOTE,
        XM_TEST_INSTRUMENT,
        XM_FINE_VOLUME_SLIDE_DOWN_COLUMN,
        XM_TEST_EMPTY_EFFECT,
        XM_TEST_EMPTY_OPERAND,
        XM_TEST_NOTE,
        XM_TEST_INSTRUMENT,
        XM_FINE_VOLUME_SLIDE_UP_COLUMN,
        XM_TEST_EMPTY_EFFECT,
        XM_TEST_EMPTY_OPERAND,
        XM_TEST_NOTE,
        XM_TEST_INSTRUMENT,
        XM_ZERO_FINE_VOLUME_SLIDE_DOWN_COLUMN,
        XM_TEST_EMPTY_EFFECT,
        XM_TEST_EMPTY_OPERAND,
        XM_TEST_NOTE,
        XM_TEST_INSTRUMENT,
        XM_ZERO_FINE_VOLUME_SLIDE_UP_COLUMN,
        XM_TEST_EMPTY_EFFECT,
        XM_TEST_EMPTY_OPERAND,
    ];
    let pattern_header = synthetic_pattern_header(XM_TEST_SINGLE_ROW, bytes.len() as u16);
    let pattern = decode_xm_pattern(&bytes, &header, &pattern_header).unwrap();

    assert_eq!(
        pattern.cell(0, 0).unwrap().effects[0],
        EffectCommand {
            effect: INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            operand: XM_FINE_VOLUME_SLIDE_DOWN_OPERAND,
        }
    );
    assert_eq!(
        pattern.cell(1, 0).unwrap().effects[0],
        EffectCommand {
            effect: INTERNAL_FINE_VOLUME_SLIDE_UP_EFFECT,
            operand: XM_FINE_VOLUME_SLIDE_UP_OPERAND,
        }
    );
    assert_eq!(
        pattern.cell(2, 0).unwrap().effects[0],
        EffectCommand::default()
    );
    assert_eq!(
        pattern.cell(3, 0).unwrap().effects[0],
        EffectCommand::default()
    );
}

#[test]
fn rejects_packed_cells_that_end_mid_field() {
    let header = synthetic_header(1, 1);
    let bytes = [0x9f, 49];
    let pattern_header = synthetic_pattern_header(1, bytes.len() as u16);

    assert!(matches!(
        decode_xm_pattern(&bytes, &header, &pattern_header),
        Err(XmParseError::PackedPatternCellTooShort {
            pattern_index: 0,
            row: 0,
            channel: 0,
            ..
        })
    ));
}

#[test]
fn parses_milkytracker_bundled_xm_pattern_headers() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture.file_name)).unwrap();
        let header = parse_xm_header(&bytes).unwrap();
        let patterns = parse_xm_pattern_headers(&bytes, &header).unwrap();

        assert_eq!(
            patterns.len(),
            fixture.pattern_count as usize,
            "{}",
            fixture.file_name
        );
        assert!(
            patterns
                .iter()
                .all(|pattern| pattern.header_length == 9 && pattern.packing_type == 0),
            "{}",
            fixture.file_name
        );
        assert_eq!(
            patterns
                .iter()
                .take(fixture.first_pattern_rows.len())
                .map(|pattern| pattern.row_count)
                .collect::<Vec<_>>(),
            fixture.first_pattern_rows,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            patterns
                .iter()
                .take(fixture.first_pattern_packed_lengths.len())
                .map(|pattern| pattern.packed_data_len)
                .collect::<Vec<_>>(),
            fixture.first_pattern_packed_lengths,
            "{}",
            fixture.file_name
        );

        let mut unique_row_counts = patterns
            .iter()
            .map(|pattern| pattern.row_count)
            .collect::<Vec<_>>();
        unique_row_counts.sort_unstable();
        unique_row_counts.dedup();
        assert_eq!(
            unique_row_counts, fixture.unique_row_counts,
            "{}",
            fixture.file_name
        );

        assert_eq!(
            patterns
                .iter()
                .map(|pattern| pattern.packed_data_len as usize)
                .sum::<usize>(),
            fixture.packed_pattern_data_total,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            patterns
                .iter()
                .filter(|pattern| pattern.packed_data_len == 0)
                .count(),
            fixture.empty_pattern_count,
            "{}",
            fixture.file_name
        );
    }
}

#[test]
fn rejects_truncated_pattern_header() {
    if !fixtures_available() {
        return;
    }

    let mut bytes = fs::read(fixture_path("milky.xm")).unwrap();
    let header = parse_xm_header(&bytes).unwrap();
    bytes.truncate(336 + 5);

    assert!(matches!(
        parse_xm_pattern_headers(&bytes, &header),
        Err(XmParseError::PatternHeaderTooShort {
            pattern_index: 0,
            ..
        })
    ));
}

#[test]
fn rejects_truncated_pattern_data() {
    if !fixtures_available() {
        return;
    }

    let mut bytes = fs::read(fixture_path("milky.xm")).unwrap();
    let header = parse_xm_header(&bytes).unwrap();
    bytes.truncate(336 + 9 + 10);

    assert!(matches!(
        parse_xm_pattern_headers(&bytes, &header),
        Err(XmParseError::PatternDataTooShort {
            pattern_index: 0,
            ..
        })
    ));
}

#[test]
fn pattern_headers_advance_by_declared_header_length() {
    let mut bytes = synthetic_xm_header_bytes();
    bytes[XM_TEST_PATTERNS_FIELD_OFFSET..XM_TEST_PATTERNS_FIELD_OFFSET + 2]
        .copy_from_slice(&1_u16.to_le_bytes());
    let pattern_offset = bytes.len();
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&5_u16.to_le_bytes());
    bytes.extend_from_slice(&[0xaa, 0xbb, 0xcc]);
    bytes.extend_from_slice(&[49, 1, 0, 0, 0]);

    let header = parse_xm_header(&bytes).unwrap();
    let patterns = parse_xm_pattern_headers(&bytes, &header).unwrap();

    assert_eq!(patterns[0].header_length, 12);
    assert_eq!(patterns[0].packed_data_offset, pattern_offset + 12);
    assert_eq!(patterns[0].next_offset, pattern_offset + 17);
}

#[test]
fn rejects_pattern_headers_smaller_than_required_fields() {
    let mut bytes = synthetic_xm_header_bytes();
    bytes[XM_TEST_PATTERNS_FIELD_OFFSET..XM_TEST_PATTERNS_FIELD_OFFSET + 2]
        .copy_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 5]);

    let header = parse_xm_header(&bytes).unwrap();

    assert_eq!(
        parse_xm_pattern_headers(&bytes, &header).unwrap_err(),
        XmParseError::InvalidPatternHeaderLength {
            pattern_index: 0,
            header_length: 8,
            minimum: 9,
        }
    );
}

#[test]
fn rejects_non_xm_signature() {
    if !fixtures_available() {
        return;
    }

    let mut bytes = fs::read(fixture_path("milky.xm")).unwrap();
    bytes[0] = b'X';

    assert_eq!(
        parse_xm_header(&bytes).unwrap_err(),
        XmParseError::InvalidSignature
    );
}

#[test]
fn rejects_xm_versions_milkytracker_does_not_accept() {
    if !fixtures_available() {
        return;
    }

    let mut bytes = fs::read(fixture_path("milky.xm")).unwrap();
    bytes[58..60].copy_from_slice(&0x0105_u16.to_le_bytes());

    assert_eq!(
        parse_xm_header(&bytes).unwrap_err(),
        XmParseError::UnsupportedVersion(0x0105)
    );
}

#[test]
fn rejects_xm_order_counts_outside_core_range() {
    let mut bytes = synthetic_xm_header_bytes();
    bytes[XM_TEST_HEADER_FIELDS_OFFSET..XM_TEST_HEADER_FIELDS_OFFSET + 2]
        .copy_from_slice(&256_u16.to_le_bytes());

    assert!(matches!(
        parse_xm_header(&bytes),
        Err(XmParseError::InvalidOrderCount {
            order_count: 256,
            maximum: rustytracker_core::MAX_ACTIVE_ORDERS,
            ..
        })
    ));
}

#[test]
fn rejects_xm_channel_counts_outside_core_range() {
    let mut bytes = synthetic_xm_header_bytes();
    bytes[XM_TEST_CHANNELS_FIELD_OFFSET..XM_TEST_CHANNELS_FIELD_OFFSET + 2]
        .copy_from_slice(&(rustytracker_core::EDITOR_PATTERN_CHANNELS + 1).to_le_bytes());

    assert!(matches!(
        parse_xm_header(&bytes),
        Err(XmParseError::InvalidChannelCount {
            channel_count,
            maximum: rustytracker_core::EDITOR_PATTERN_CHANNELS,
            ..
        }) if channel_count == rustytracker_core::EDITOR_PATTERN_CHANNELS + 1
    ));
}

#[test]
fn rejects_truncated_headers() {
    let bytes = vec![0; 32];

    assert!(matches!(
        parse_xm_header(&bytes),
        Err(XmParseError::Truncated { .. })
    ));
}

fn synthetic_xm_header_bytes() -> Vec<u8> {
    let mut bytes = vec![0; XM_TEST_HEADER_BYTES];
    bytes[..XM_TEST_SIGNATURE.len()].copy_from_slice(XM_TEST_SIGNATURE);
    bytes[XM_TEST_MARKER_OFFSET] = XM_TEST_MARKER;
    bytes[XM_TEST_VERSION_OFFSET..XM_TEST_VERSION_OFFSET + 2]
        .copy_from_slice(&XM_TEST_VERSION.to_le_bytes());
    bytes[XM_TEST_HEADER_SIZE_OFFSET..XM_TEST_HEADER_SIZE_OFFSET + 4]
        .copy_from_slice(&XM_TEST_HEADER_SIZE.to_le_bytes());
    bytes[XM_TEST_HEADER_FIELDS_OFFSET..XM_TEST_HEADER_FIELDS_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_ORDERS.to_le_bytes());
    bytes[XM_TEST_CHANNELS_FIELD_OFFSET..XM_TEST_CHANNELS_FIELD_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_CHANNELS.to_le_bytes());
    bytes[XM_TEST_PATTERNS_FIELD_OFFSET..XM_TEST_PATTERNS_FIELD_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_PATTERNS.to_le_bytes());
    bytes[XM_TEST_INSTRUMENTS_FIELD_OFFSET..XM_TEST_INSTRUMENTS_FIELD_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_INSTRUMENTS.to_le_bytes());
    bytes[XM_TEST_TICK_SPEED_FIELD_OFFSET..XM_TEST_TICK_SPEED_FIELD_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_SPEED.to_le_bytes());
    bytes[XM_TEST_BPM_FIELD_OFFSET..XM_TEST_BPM_FIELD_OFFSET + 2]
        .copy_from_slice(&XM_TEST_DEFAULT_BPM.to_le_bytes());
    bytes
}

fn fixtures_available() -> bool {
    fixture_root().is_some()
}

fn fixture_path(file_name: &str) -> PathBuf {
    fixture_root()
        .expect("MilkyTracker fixtures not found; set MILKYTRACKER_ROOT or clone MilkyTracker next to rustytracker")
        .join(file_name)
}

fn fixture_root() -> Option<PathBuf> {
    if let Some(root) = std::env::var_os("MILKYTRACKER_ROOT") {
        let root = PathBuf::from(root);
        let candidates = [root.join("resources/music"), root];
        if let Some(path) = candidates.into_iter().find(|path| path.is_dir()) {
            return Some(path);
        }
    }

    let sibling =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../MilkyTracker/resources/music");
    sibling.is_dir().then_some(sibling)
}

#[derive(Debug, PartialEq, Eq)]
struct DecodedPatternStats {
    cell_count: usize,
    non_empty_count: usize,
    checksum: u64,
    first_non_empty: Option<(usize, u16, u16, [u8; 6])>,
}

fn decoded_pattern_stats(patterns: &[rustytracker_core::Pattern]) -> DecodedPatternStats {
    let mut cell_count = 0;
    let mut non_empty_count = 0;
    let mut checksum = FNV_OFFSET;
    let mut first_non_empty = None;

    for (pattern_index, pattern) in patterns.iter().enumerate() {
        for row in 0..pattern.rows() {
            for channel in 0..pattern.channels() {
                let expanded = expanded_cell_bytes(pattern.cell(channel, row).unwrap());
                cell_count += 1;

                if expanded != [0; 6] {
                    non_empty_count += 1;
                    first_non_empty.get_or_insert((pattern_index, row, channel, expanded));
                }

                for byte in expanded {
                    checksum ^= byte as u64;
                    checksum = checksum.wrapping_mul(FNV_PRIME);
                }
            }
        }
    }

    DecodedPatternStats {
        cell_count,
        non_empty_count,
        checksum,
        first_non_empty,
    }
}

fn expanded_cell_bytes(cell: &rustytracker_core::PatternCell) -> [u8; 6] {
    [
        cell.note.raw(),
        cell.instrument,
        cell.effects[0].effect,
        cell.effects[0].operand,
        cell.effects[1].effect,
        cell.effects[1].operand,
    ]
}

fn synthetic_header(channel_count: u16, pattern_count: u16) -> XmModuleHeader {
    XmModuleHeader {
        title: String::new(),
        tracker_name: String::new(),
        version: 0x0104,
        header_size: 276,
        song_length: 1,
        restart_position: 0,
        channel_count,
        pattern_count,
        instrument_count: 0,
        flags: 1,
        frequency_table: FrequencyTable::Linear,
        default_tick_speed: 6,
        default_bpm: 125,
        orders: vec![0],
    }
}

fn synthetic_pattern_header(row_count: u16, packed_data_len: u16) -> XmPatternHeader {
    XmPatternHeader {
        index: 0,
        header_length: 9,
        packing_type: 0,
        row_count,
        packed_data_len,
        packed_data_offset: 0,
        next_offset: packed_data_len as usize,
    }
}
