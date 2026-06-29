use std::fs;

use rustytracker_core::{
    EnvelopePoint, FrequencyTable, SampleData, SampleLoopKind, SAMPLES_PER_INSTRUMENT,
};
use rustytracker_test_support::{
    milkytracker_fixture_path as fixture_path,
    milkytracker_fixtures_available as fixtures_available,
};
use rustytracker_xm::parse_xm_module;

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XM_TEST_SIGNATURE: &[u8; 17] = b"Extended Module: ";
const XM_TEST_TRACKER_NAME: &[u8; 11] = b"Rusty tests";
const XM_TEST_MARKER: u8 = 0x1a;
const XM_TEST_U16_BYTES: usize = 2;
const XM_TEST_U32_BYTES: usize = 4;
const XM_TEST_SIGNATURE_OFFSET: usize = 0;
const XM_TEST_MARKER_OFFSET: usize = 37;
const XM_TEST_TRACKER_OFFSET: usize = 38;
const XM_TEST_VERSION_OFFSET: usize = 58;
const XM_TEST_HEADER_SIZE_OFFSET: usize = 60;
const XM_TEST_SONG_LENGTH_OFFSET: usize = 64;
const XM_TEST_RESTART_OFFSET: usize = 66;
const XM_TEST_CHANNELS_OFFSET: usize = 68;
const XM_TEST_PATTERNS_OFFSET: usize = 70;
const XM_TEST_INSTRUMENTS_OFFSET: usize = 72;
const XM_TEST_FLAGS_OFFSET: usize = 74;
const XM_TEST_SPEED_OFFSET: usize = 76;
const XM_TEST_BPM_OFFSET: usize = 78;
const XM_TEST_ORDER_TABLE_OFFSET: usize = 80;
const XM_TEST_HEADER_BYTES: usize = 336;
const XM_TEST_HEADER_SIZE: u32 = 276;
const XM_TEST_PATTERN_HEADER_LEN: u32 = 9;
const XM_TEST_DEFAULT_ROWS: u16 = 64;
const XM_TEST_CHANNELS: u16 = 2;
const XM_TEST_SONG_LENGTH: u16 = 2;
const XM_TEST_RESTART: u16 = 0;
const XM_TEST_DECLARED_PATTERNS: u16 = 1;
const XM_TEST_INSTRUMENTS: u16 = 0;
const XM_TEST_FIRST_ORDER: u8 = 0;
const XM_TEST_ORDER_REFERENCED_PATTERN: u8 = 2;
const XM_TEST_VERSION: u16 = 0x0104;
const XM_TEST_FLAGS_LINEAR: u16 = 1;
const XM_TEST_SPEED: u16 = 6;
const XM_TEST_BPM: u16 = 125;
const XM_TEST_PATTERN_PACKING_TYPE: u8 = 0;
const XM_TEST_EMPTY_PATTERN_DATA_LEN: u16 = 0;
const XM_TEST_EFFECT_SLOTS: u8 = 2;

#[derive(Debug)]
struct ExpectedModule {
    file_name: &'static str,
    title: &'static str,
    restart_position: u16,
    channel_count: u16,
    pattern_count: usize,
    instrument_count: usize,
    first_orders: &'static [u8],
    first_pattern_rows: u16,
    first_instrument_name: &'static str,
    first_volume_envelope_point_count: u8,
    first_volume_envelope_first_point: EnvelopePoint,
    first_volume_envelope_flags: u8,
    first_panning_envelope_point_count: u8,
    first_panning_envelope_first_point: EnvelopePoint,
    first_panning_envelope_flags: u8,
    first_instrument_volume_fadeout: u16,
    first_instrument_vibrato_depth: u8,
    first_sample_name: &'static str,
    first_sample_length: u32,
    first_sample_loop_start: u32,
    first_sample_loop_length: u32,
    first_sample_loop_kind: SampleLoopKind,
    first_sample_data_prefix: &'static [i8],
    decoded_sample_checksum: u64,
}

const FIXTURES: &[ExpectedModule] = &[
    ExpectedModule {
        file_name: "milky.xm",
        title: "milk in veins",
        restart_position: 0,
        channel_count: 10,
        pattern_count: 17,
        instrument_count: 7,
        first_orders: &[1, 2, 0, 3, 4, 5, 6, 7],
        first_pattern_rows: 64,
        first_instrument_name: "",
        first_volume_envelope_point_count: 4,
        first_volume_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 256,
        },
        first_volume_envelope_flags: 5,
        first_panning_envelope_point_count: 6,
        first_panning_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 128,
        },
        first_panning_envelope_flags: 5,
        first_instrument_volume_fadeout: 736,
        first_instrument_vibrato_depth: 0,
        first_sample_name: "beng",
        first_sample_length: 997,
        first_sample_loop_start: 613,
        first_sample_loop_length: 384,
        first_sample_loop_kind: SampleLoopKind::Forward,
        first_sample_data_prefix: &[
            -19, 88, 21, -33, 4, -17, -6, -12, -10, -9, -13, -5, -19, 87, 19, -31,
        ],
        decoded_sample_checksum: 0x6868_13f3_c203_59df,
    },
    ExpectedModule {
        file_name: "slumberjack.xm",
        title: "slumberjack",
        restart_position: 0,
        channel_count: 8,
        pattern_count: 27,
        instrument_count: 7,
        first_orders: &[5, 5, 5, 5, 6, 0, 1, 2],
        first_pattern_rows: 32,
        first_instrument_name: "",
        first_volume_envelope_point_count: 4,
        first_volume_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 256,
        },
        first_volume_envelope_flags: 7,
        first_panning_envelope_point_count: 4,
        first_panning_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 128,
        },
        first_panning_envelope_flags: 5,
        first_instrument_volume_fadeout: 480,
        first_instrument_vibrato_depth: 16,
        first_sample_name: "",
        first_sample_length: 446,
        first_sample_loop_start: 414,
        first_sample_loop_length: 32,
        first_sample_loop_kind: SampleLoopKind::Forward,
        first_sample_data_prefix: &[
            1, 9, 22, 41, 60, 75, 94, 115, 120, 122, 127, 127, 127, 127, 127, 127,
        ],
        decoded_sample_checksum: 0x85bc_efa0_b95b_2b52,
    },
    ExpectedModule {
        file_name: "sv_ttt.xm",
        title: "The Titan Turrican",
        restart_position: 2,
        channel_count: 6,
        pattern_count: 17,
        instrument_count: 44,
        first_orders: &[9, 0, 1, 2, 3, 4, 5, 6],
        first_pattern_rows: 64,
        first_instrument_name: "svenzzon of titan",
        first_volume_envelope_point_count: 0,
        first_volume_envelope_first_point: EnvelopePoint { frame: 0, value: 0 },
        first_volume_envelope_flags: 0,
        first_panning_envelope_point_count: 0,
        first_panning_envelope_first_point: EnvelopePoint { frame: 0, value: 0 },
        first_panning_envelope_flags: 0,
        first_instrument_volume_fadeout: 0,
        first_instrument_vibrato_depth: 0,
        first_sample_name: "",
        first_sample_length: 132,
        first_sample_loop_start: 0,
        first_sample_loop_length: 132,
        first_sample_loop_kind: SampleLoopKind::Forward,
        first_sample_data_prefix: &[6, 22, 16, 9, 7, 5, 3, 1, -3, -6, -7, -8, -10, -11, -15, -16],
        decoded_sample_checksum: 0x7877_f846_a0ee_3dd9,
    },
    ExpectedModule {
        file_name: "theday.xm",
        title: "the day they landed",
        restart_position: 0,
        channel_count: 8,
        pattern_count: 42,
        instrument_count: 7,
        first_orders: &[0, 2, 3, 4, 5, 1, 6, 9],
        first_pattern_rows: 64,
        first_instrument_name: "2                    0",
        first_volume_envelope_point_count: 2,
        first_volume_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 256,
        },
        first_volume_envelope_flags: 0,
        first_panning_envelope_point_count: 2,
        first_panning_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 128,
        },
        first_panning_envelope_flags: 0,
        first_instrument_volume_fadeout: 0,
        first_instrument_vibrato_depth: 0,
        first_sample_name: "",
        first_sample_length: 87,
        first_sample_loop_start: 2,
        first_sample_loop_length: 85,
        first_sample_loop_kind: SampleLoopKind::Forward,
        first_sample_data_prefix: &[
            -9, -1, -1, 0, 85, 84, 81, 80, -80, -80, -80, -76, -74, -73, -70, -69,
        ],
        decoded_sample_checksum: 0x5f5e_bc48_2da7_96ed,
    },
    ExpectedModule {
        file_name: "universalnetwork2_real.xm",
        title: " universal network 2",
        restart_position: 0,
        channel_count: 6,
        pattern_count: 32,
        instrument_count: 16,
        first_orders: &[12, 13, 14, 15, 1, 17, 16, 2],
        first_pattern_rows: 64,
        first_instrument_name: " ...Strobe&Kmuland...",
        first_volume_envelope_point_count: 2,
        first_volume_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 256,
        },
        first_volume_envelope_flags: 0,
        first_panning_envelope_point_count: 2,
        first_panning_envelope_first_point: EnvelopePoint {
            frame: 0,
            value: 128,
        },
        first_panning_envelope_flags: 0,
        first_instrument_volume_fadeout: 0,
        first_instrument_vibrato_depth: 0,
        first_sample_name: "",
        first_sample_length: 2_345,
        first_sample_loop_start: 0,
        first_sample_loop_length: 0,
        first_sample_loop_kind: SampleLoopKind::None,
        first_sample_data_prefix: &[
            57, 2, 0, -3, -26, -23, -13, -7, 4, 13, 19, 30, 35, 44, 50, 47,
        ],
        decoded_sample_checksum: 0x0162_07eb_38de_b29d,
    },
];

#[test]
fn parses_bundled_xm_files_into_core_modules() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture.file_name)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();

        assert_eq!(
            module.header.title.as_str(),
            fixture.title,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.header.restart_position, fixture.restart_position,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.header.channel_count, fixture.channel_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.header.frequency_table,
            FrequencyTable::Linear,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.patterns.len(),
            fixture.pattern_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.instruments.len(),
            fixture.instrument_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            module.samples.len(),
            fixture.instrument_count * SAMPLES_PER_INSTRUMENT,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            &module.orders[..fixture.first_orders.len()],
            fixture.first_orders,
            "{}",
            fixture.file_name
        );
        assert_eq!(module.patterns[0].rows(), fixture.first_pattern_rows);

        let first_instrument = &module.instruments[0];
        assert_eq!(
            first_instrument.name.as_str(),
            fixture.first_instrument_name
        );
        assert_eq!(first_instrument.sample_slots[0], Some(0));
        assert!(first_instrument.sample_slots[1..]
            .iter()
            .all(Option::is_none));
        assert!(first_instrument
            .note_sample_map
            .iter()
            .all(|sample_index| *sample_index == Some(0)));
        assert_eq!(
            first_instrument.volume_envelope.point_count, fixture.first_volume_envelope_point_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.volume_envelope.points[0], fixture.first_volume_envelope_first_point,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.volume_envelope.flags, fixture.first_volume_envelope_flags,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.panning_envelope.point_count,
            fixture.first_panning_envelope_point_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.panning_envelope.points[0], fixture.first_panning_envelope_first_point,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.panning_envelope.flags, fixture.first_panning_envelope_flags,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.volume_fadeout, fixture.first_instrument_volume_fadeout,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_instrument.vibrato.depth, fixture.first_instrument_vibrato_depth,
            "{}",
            fixture.file_name
        );

        let first_sample = &module.samples[0];
        assert_eq!(first_sample.name.as_str(), fixture.first_sample_name);
        assert_eq!(first_sample.length, fixture.first_sample_length);
        assert_eq!(first_sample.loop_start, fixture.first_sample_loop_start);
        assert_eq!(first_sample.loop_length, fixture.first_sample_loop_length);
        assert_eq!(
            first_sample.loop_kind, fixture.first_sample_loop_kind,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            first_sample.data.frame_count(),
            fixture.first_sample_length as usize
        );
        match &first_sample.data {
            SampleData::Pcm8(values) => assert_eq!(
                &values[..fixture.first_sample_data_prefix.len()],
                fixture.first_sample_data_prefix
            ),
            other => panic!("expected first sample to be 8-bit PCM, got {other:?}"),
        }

        assert_eq!(
            decoded_sample_checksum(&module.samples),
            fixture.decoded_sample_checksum,
            "{}",
            fixture.file_name
        );
    }
}

#[test]
fn adds_empty_patterns_for_orders_past_declared_pattern_count() {
    let bytes = synthetic_xm_with_sparse_order_reference();
    let module = parse_xm_module(&bytes).unwrap();

    assert_eq!(module.orders, vec![0, XM_TEST_ORDER_REFERENCED_PATTERN]);
    assert_eq!(
        module.patterns.len(),
        XM_TEST_ORDER_REFERENCED_PATTERN as usize + 1
    );

    let appended = &module.patterns[XM_TEST_ORDER_REFERENCED_PATTERN as usize];
    assert_eq!(appended.rows(), XM_TEST_DEFAULT_ROWS);
    assert_eq!(appended.channels(), XM_TEST_CHANNELS);
    assert_eq!(appended.effect_slots(), XM_TEST_EFFECT_SLOTS);
}

fn synthetic_xm_with_sparse_order_reference() -> Vec<u8> {
    let mut bytes = vec![0; XM_TEST_HEADER_BYTES];
    bytes[XM_TEST_SIGNATURE_OFFSET..XM_TEST_SIGNATURE_OFFSET + XM_TEST_SIGNATURE.len()]
        .copy_from_slice(XM_TEST_SIGNATURE);
    bytes[XM_TEST_MARKER_OFFSET] = XM_TEST_MARKER;
    bytes[XM_TEST_TRACKER_OFFSET..XM_TEST_TRACKER_OFFSET + XM_TEST_TRACKER_NAME.len()]
        .copy_from_slice(XM_TEST_TRACKER_NAME);
    bytes[XM_TEST_VERSION_OFFSET..XM_TEST_VERSION_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_VERSION.to_le_bytes());
    bytes[XM_TEST_HEADER_SIZE_OFFSET..XM_TEST_HEADER_SIZE_OFFSET + XM_TEST_U32_BYTES]
        .copy_from_slice(&XM_TEST_HEADER_SIZE.to_le_bytes());
    bytes[XM_TEST_SONG_LENGTH_OFFSET..XM_TEST_SONG_LENGTH_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_SONG_LENGTH.to_le_bytes());
    bytes[XM_TEST_RESTART_OFFSET..XM_TEST_RESTART_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_RESTART.to_le_bytes());
    bytes[XM_TEST_CHANNELS_OFFSET..XM_TEST_CHANNELS_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_CHANNELS.to_le_bytes());
    bytes[XM_TEST_PATTERNS_OFFSET..XM_TEST_PATTERNS_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_DECLARED_PATTERNS.to_le_bytes());
    bytes[XM_TEST_INSTRUMENTS_OFFSET..XM_TEST_INSTRUMENTS_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_INSTRUMENTS.to_le_bytes());
    bytes[XM_TEST_FLAGS_OFFSET..XM_TEST_FLAGS_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_FLAGS_LINEAR.to_le_bytes());
    bytes[XM_TEST_SPEED_OFFSET..XM_TEST_SPEED_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_SPEED.to_le_bytes());
    bytes[XM_TEST_BPM_OFFSET..XM_TEST_BPM_OFFSET + XM_TEST_U16_BYTES]
        .copy_from_slice(&XM_TEST_BPM.to_le_bytes());
    bytes[XM_TEST_ORDER_TABLE_OFFSET] = XM_TEST_FIRST_ORDER;
    bytes[XM_TEST_ORDER_TABLE_OFFSET + usize::from(XM_TEST_SONG_LENGTH - 1)] =
        XM_TEST_ORDER_REFERENCED_PATTERN;

    bytes.extend_from_slice(&XM_TEST_PATTERN_HEADER_LEN.to_le_bytes());
    bytes.push(XM_TEST_PATTERN_PACKING_TYPE);
    bytes.extend_from_slice(&XM_TEST_DEFAULT_ROWS.to_le_bytes());
    bytes.extend_from_slice(&XM_TEST_EMPTY_PATTERN_DATA_LEN.to_le_bytes());

    bytes
}

fn decoded_sample_checksum(samples: &[rustytracker_core::Sample]) -> u64 {
    let mut checksum = FNV_OFFSET;

    for sample in samples {
        match &sample.data {
            SampleData::Empty => {}
            SampleData::Pcm8(values) => {
                for value in values.iter() {
                    checksum ^= *value as u8 as u64;
                    checksum = checksum.wrapping_mul(FNV_PRIME);
                }
            }
            SampleData::Pcm16(values) => {
                for value in values.iter() {
                    for byte in value.to_le_bytes() {
                        checksum ^= byte as u64;
                        checksum = checksum.wrapping_mul(FNV_PRIME);
                    }
                }
            }
        }
    }

    checksum
}
