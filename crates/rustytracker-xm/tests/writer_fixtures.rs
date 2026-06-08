use std::fs;
use std::path::PathBuf;

use rustytracker_core::{
    EffectCommand, Envelope, EnvelopePoint, FrequencyTable, Instrument, InstrumentName, Module,
    ModuleTitle, Note, Pattern, PatternCell, Sample, SampleData, SampleLoopKind, SampleName,
    Vibrato, MAX_XM_NOTES, SAMPLES_PER_INSTRUMENT,
};
use rustytracker_xm::{
    decode_xm_patterns, parse_xm_header, parse_xm_instruments, parse_xm_module,
    parse_xm_pattern_headers, write_xm_header, write_xm_instruments, write_xm_module,
    write_xm_patterns, XmSampleField, XmWriteError,
};

const FIXTURES: &[&str] = &[
    "milky.xm",
    "slumberjack.xm",
    "sv_ttt.xm",
    "theday.xm",
    "universalnetwork2_real.xm",
];
const XM_WRITER_TEST_VERSION: u16 = 0x0104;
const XM_WRITER_TEST_HEADER_SIZE: u32 = 276;
const XM_WRITER_TEST_ORDERS: &[u8] = &[0, 0, 0];
const XM_WRITER_FNV_OFFSET: u64 = 0xcbf29ce484222325;
const XM_WRITER_FNV_PRIME: u64 = 0x100000001b3;
const XM_WRITER_OPTION_NONE_TAG: u8 = 0;
const XM_WRITER_OPTION_SOME_TAG: u8 = 1;
const XM_WRITER_SAMPLE_PREFIX_FRAMES: usize = 16;
const XM_WRITER_ORDER_TABLE_LEN: usize = 256;
const XM_WRITER_OVERLONG_ORDER_LEN: usize = XM_WRITER_ORDER_TABLE_LEN + 1;
const XM_WRITER_TEST_ROWS: u16 = 1;
const XM_WRITER_TEST_CHANNELS: u16 = 2;
const XM_WRITER_TEST_EFFECT_SLOTS: u8 = 2;
const XM_WRITER_TEST_NOTE: u8 = 49;
const XM_WRITER_TEST_INSTRUMENT: u8 = 3;
const XM_WRITER_EMPTY_INSTRUMENT: u8 = 0;
const XM_WRITER_PATTERN_HEADER_LEN: u32 = 9;
const XM_WRITER_PATTERN_PACKING_TYPE: u8 = 0;
const XM_WRITER_EMPTY_PATTERN_DATA_LEN: u16 = 0;
const XM_WRITER_UNPACKED_CELL_LEN: usize = 5;
const XM_WRITER_EMPTY_EFFECT: u8 = 0;
const XM_WRITER_EMPTY_OPERAND: u8 = 0;
const XM_WRITER_TEST_EMPTY_VOLUME_COLUMN: u8 = 0;
const XM_WRITER_XM_ARPEGGIO_EFFECT: u8 = 0x00;
const XM_WRITER_INTERNAL_ARPEGGIO_EFFECT: u8 = 0x20;
const XM_WRITER_ARPEGGIO_OPERAND: u8 = 0x37;
const XM_WRITER_XM_EXTENDED_EFFECT: u8 = 0x0e;
const XM_WRITER_INTERNAL_EXTENDED_EFFECT: u8 = 0x3a;
const XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT: u8 = 0x3b;
const XM_WRITER_EXTENDED_SOURCE_OPERAND: u8 = 0x07;
const XM_WRITER_EXTENDED_OPERAND: u8 = 0xa7;
const XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND: u8 = 0x05;
const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND: u8 = 0x06;
const XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND: u8 = 0x00;
const XM_WRITER_FINE_VOLUME_SLIDE_UP_COLUMN: u8 = 0x95;
const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_COLUMN: u8 = 0x86;
const XM_WRITER_FINE_VOLUME_SLIDE_UP_EXTENDED_OPERAND: u8 = 0xa0;
const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_EXTENDED_OPERAND: u8 = 0xb0;
const XM_WRITER_XM_EXTRA_FINE_PORTA_EFFECT: u8 = 0x21;
const XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT: u8 = 0x41;
const XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND: u8 = 0x05;
const XM_WRITER_EXTRA_FINE_PORTA_OPERAND: u8 = 0x15;
const XM_WRITER_VOLUME_EFFECT: u8 = 0x0c;
const XM_WRITER_VOLUME_SLIDE_EFFECT: u8 = 0x0a;
const XM_WRITER_VOLUME_SLIDE_UP_OPERAND: u8 = 0x40;
const XM_WRITER_VOLUME_SLIDE_DOWN_OPERAND: u8 = 0x04;
const XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND: u8 = 0x34;
const XM_WRITER_FULL_VOLUME_255: u8 = 0xff;
const XM_WRITER_FULL_VOLUME_64: u8 = 0x40;
const XM_WRITER_FULL_VOLUME_COLUMN: u8 = 0x50;
const XM_WRITER_VIBRATO_EFFECT: u8 = 0x04;
const XM_WRITER_VIBRATO_SPEED_OPERAND: u8 = 0x40;
const XM_WRITER_VIBRATO_DEPTH_OPERAND: u8 = 0x04;
const XM_WRITER_PANNING_EFFECT: u8 = 0x08;
const XM_WRITER_PANNING_SLIDE_EFFECT: u8 = 0x19;
const XM_WRITER_PANNING_SLIDE_LEFT_OPERAND: u8 = 0x04;
const XM_WRITER_PANNING_SLIDE_RIGHT_OPERAND: u8 = 0x40;
const XM_WRITER_CENTER_PANNING_255: u8 = 0x80;
const XM_WRITER_CENTER_PANNING_COLUMN: u8 = 0xc8;
const XM_WRITER_TONE_PORTAMENTO_EFFECT: u8 = 0x03;
const XM_WRITER_HIGH_NIBBLE_TONE_PORTAMENTO_OPERAND: u8 = 0x40;
const XM_WRITER_LOW_NIBBLE_TONE_PORTAMENTO_OPERAND: u8 = 0x05;
const XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE: u32 = 29;
const XM_WRITER_INSTRUMENT_HEADER_SIZE: u32 = 263;
const XM_WRITER_SAMPLE_HEADER_SIZE: u32 = 40;
const XM_WRITER_SINGLE_INSTRUMENT_COUNT: usize = 1;
const XM_WRITER_EMPTY_SAMPLE_COUNT: u16 = 0;
const XM_WRITER_SINGLE_SAMPLE_COUNT: u16 = 1;
const XM_WRITER_FIRST_INSTRUMENT_INDEX: usize = 0;
const XM_WRITER_FIRST_SAMPLE_INDEX: usize = 0;
const XM_WRITER_SECOND_SAMPLE_INDEX: usize = 1;
const XM_WRITER_EMPTY_ENVELOPE_POINT_INDEX: u8 = 0;
const XM_WRITER_TEST_INSTRUMENT_NAME: &str = "lead inst";
const XM_WRITER_TEST_SAMPLE_NAME: &str = "sample a";
const XM_WRITER_TEST_ENVELOPE_FRAME: u16 = 12;
const XM_WRITER_TEST_ENVELOPE_VALUE: u16 = 32;
const XM_WRITER_TEST_ENVELOPE_POINT_COUNT: u8 = 1;
const XM_WRITER_TEST_ENVELOPE_FLAG: u8 = 1;
const XM_WRITER_TEST_VIBRATO_WAVEFORM: u8 = 2;
const XM_WRITER_TEST_VIBRATO_SWEEP: u8 = 3;
const XM_WRITER_TEST_VIBRATO_DEPTH: u8 = 16;
const XM_WRITER_TEST_VIBRATO_RATE: u8 = 5;
const XM_WRITER_TEST_VOLUME_FADEOUT: u16 = 480;
const XM_WRITER_TEST_SAMPLE_VOLUME_255: u8 = 192;
const XM_WRITER_TEST_SAMPLE_VOLUME_64: u8 = 48;
const XM_WRITER_TEST_SAMPLE_PANNING: u8 = 96;
const XM_WRITER_TEST_SAMPLE_FINETUNE: i8 = -12;
const XM_WRITER_TEST_SAMPLE_RELATIVE_NOTE: i8 = 7;
const XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE: u8 = 1;
const XM_WRITER_16_BIT_SAMPLE_TYPE: u8 = 0x10;
const XM_WRITER_FORWARD_16_BIT_SAMPLE_TYPE: u8 =
    XM_WRITER_16_BIT_SAMPLE_TYPE | XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE;
const XM_WRITER_BYTES_PER_16_BIT_SAMPLE: usize = 2;
const XM_WRITER_TEST_SAMPLE_LOOP_START: u32 = 1;
const XM_WRITER_TEST_SAMPLE_LOOP_LENGTH: u32 = 2;
const XM_WRITER_TEST_SAMPLE_VALUES_8: &[i8] = &[1, 9, 22, 41];
const XM_WRITER_TEST_SAMPLE_FRAME_COUNT_8: u32 = XM_WRITER_TEST_SAMPLE_VALUES_8.len() as u32;
const XM_WRITER_TEST_SAMPLE_DELTAS_8: &[u8] = &[1, 8, 13, 19];
const XM_WRITER_TEST_SAMPLE_VALUES_16: &[i16] = &[1_000, 500, 750, -250];
const XM_WRITER_TEST_SAMPLE_DELTAS_16: &[u8] = &[0xe8, 0x03, 0x0c, 0xfe, 0xfa, 0x00, 0x18, 0xfc];
const XM_WRITER_OVERLONG_SAMPLE_LOOP_START: u32 = u32::MAX;
const XM_WRITER_U32_FIELD_MAX: u64 = u32::MAX as u64;
const XM_WRITER_OVERLONG_16_BIT_LOOP_START_BYTE_LEN: u64 =
    XM_WRITER_OVERLONG_SAMPLE_LOOP_START as u64 * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u64;

#[derive(Debug, PartialEq, Eq)]
struct ModuleRoundtripSummary {
    header: rustytracker_core::ModuleHeader,
    orders: Vec<u8>,
    patterns: Vec<PatternRoundtripSummary>,
    instruments: Vec<InstrumentRoundtripSummary>,
    samples: Vec<SampleRoundtripSummary>,
}

#[derive(Debug, PartialEq, Eq)]
struct PatternRoundtripSummary {
    rows: u16,
    channels: u16,
    effect_slots: u8,
    non_empty_cells: usize,
    expanded_cell_checksum: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct InstrumentRoundtripSummary {
    name: InstrumentName,
    sample_slots: Vec<Option<usize>>,
    note_sample_map_checksum: u64,
    volume_envelope: Envelope,
    panning_envelope: Envelope,
    vibrato: Vibrato,
    volume_fadeout: u16,
}

#[derive(Debug, PartialEq, Eq)]
struct SampleRoundtripSummary {
    name: SampleName,
    length: u32,
    loop_start: u32,
    loop_length: u32,
    loop_kind: SampleLoopKind,
    volume: u8,
    panning: u8,
    flags: u8,
    volume_fadeout: u16,
    sample_type: u8,
    finetune: i8,
    relative_note: i8,
    data: SampleDataRoundtripSummary,
}

#[derive(Debug, PartialEq, Eq)]
enum SampleDataRoundtripSummary {
    Empty,
    Pcm8 {
        frames: usize,
        checksum: u64,
        prefix: Vec<i8>,
    },
    Pcm16 {
        frames: usize,
        checksum: u64,
        prefix: Vec<i16>,
    },
}

#[test]
fn writes_empty_module_header_and_order_table() {
    let mut module = Module::empty();
    module.header.title = ModuleTitle::new("empty writer test");
    module.orders = XM_WRITER_TEST_ORDERS.to_vec();

    let bytes = write_xm_header(&module).unwrap();
    let header = parse_xm_header(&bytes).unwrap();

    assert_eq!(header.title, "empty writer test");
    assert_eq!(header.version, XM_WRITER_TEST_VERSION);
    assert_eq!(header.header_size, XM_WRITER_TEST_HEADER_SIZE);
    assert_eq!(header.song_length, module.orders.len() as u16);
    assert_eq!(header.restart_position, module.header.restart_position);
    assert_eq!(header.channel_count, module.header.channel_count);
    assert_eq!(header.pattern_count, module.patterns.len() as u16);
    assert_eq!(header.instrument_count, module.instruments.len() as u16);
    assert_eq!(header.frequency_table, FrequencyTable::Linear);
    assert_eq!(header.default_tick_speed, module.header.tick_speed);
    assert_eq!(header.default_bpm, module.header.bpm);
    assert_eq!(header.orders, module.orders);
}

#[test]
fn roundtrips_bundled_fixture_headers_and_orders() {
    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();

        let written = write_xm_header(&module).unwrap();
        let header = parse_xm_header(&written).unwrap();

        assert_eq!(header.title, module.header.title.as_str(), "{fixture}");
        assert_eq!(
            header.channel_count, module.header.channel_count,
            "{fixture}"
        );
        assert_eq!(
            header.frequency_table, module.header.frequency_table,
            "{fixture}"
        );
        assert_eq!(
            header.default_tick_speed, module.header.tick_speed,
            "{fixture}"
        );
        assert_eq!(header.default_bpm, module.header.bpm, "{fixture}");
        assert_eq!(
            header.restart_position, module.header.restart_position,
            "{fixture}"
        );
        assert_eq!(header.orders, module.orders, "{fixture}");
        assert_eq!(
            header.pattern_count,
            module.patterns.len() as u16,
            "{fixture}"
        );
        assert_eq!(
            header.instrument_count,
            module.instruments.len() as u16,
            "{fixture}"
        );
    }
}

#[test]
fn roundtrips_bundled_fixtures_to_equivalent_core_modules() {
    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();
        let written = write_xm_module(&module).unwrap();
        let reparsed = parse_xm_module(&written).unwrap();

        assert_module_roundtrip_summary_eq(
            &module_roundtrip_summary(&reparsed),
            &module_roundtrip_summary(&module),
            fixture,
        );
    }
}

#[test]
fn roundtrips_synthetic_effect_cells_through_full_module_writer() {
    for (name, effects) in [
        (
            "effect-column arpeggio",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "effect-column extended command",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_EXTENDED_EFFECT,
                    XM_WRITER_EXTENDED_SOURCE_OPERAND,
                ),
            ],
        ),
        (
            "effect-column extra-fine portamento",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
                    XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
                ),
            ],
        ),
        (
            "volume column set volume",
            [
                effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column volume slide up",
            [
                effect(
                    XM_WRITER_VOLUME_SLIDE_EFFECT,
                    XM_WRITER_VOLUME_SLIDE_UP_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column volume slide down",
            [
                effect(
                    XM_WRITER_VOLUME_SLIDE_EFFECT,
                    XM_WRITER_VOLUME_SLIDE_DOWN_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column fine volume slide up",
            [
                effect(
                    XM_WRITER_INTERNAL_EXTENDED_EFFECT,
                    XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column fine volume slide down",
            [
                effect(
                    XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
                    XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column vibrato speed",
            [
                effect(XM_WRITER_VIBRATO_EFFECT, XM_WRITER_VIBRATO_SPEED_OPERAND),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column vibrato depth",
            [
                effect(XM_WRITER_VIBRATO_EFFECT, XM_WRITER_VIBRATO_DEPTH_OPERAND),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning set",
            [
                effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning slide left",
            [
                effect(
                    XM_WRITER_PANNING_SLIDE_EFFECT,
                    XM_WRITER_PANNING_SLIDE_LEFT_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning slide right",
            [
                effect(
                    XM_WRITER_PANNING_SLIDE_EFFECT,
                    XM_WRITER_PANNING_SLIDE_RIGHT_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column tone portamento high nibble",
            [
                effect(
                    XM_WRITER_TONE_PORTAMENTO_EFFECT,
                    XM_WRITER_HIGH_NIBBLE_TONE_PORTAMENTO_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
    ] {
        let cell = roundtrip_single_cell_module(effects.to_vec());

        assert_eq!(cell.note, Note::Key(XM_WRITER_TEST_NOTE), "{name}");
        assert_eq!(cell.instrument, XM_WRITER_TEST_INSTRUMENT, "{name}");
        assert_eq!(cell.effects, effects.to_vec(), "{name}");
    }
}

#[test]
fn rejects_order_tables_that_do_not_fit_in_xm_header() {
    let mut module = Module::empty();
    module.orders = vec![0; XM_WRITER_OVERLONG_ORDER_LEN];

    assert_eq!(
        write_xm_header(&module).unwrap_err(),
        XmWriteError::TooManyOrders {
            requested: XM_WRITER_OVERLONG_ORDER_LEN,
            maximum: XM_WRITER_ORDER_TABLE_LEN,
        }
    );
}

#[test]
fn writes_empty_pattern_headers_without_payload_data() {
    let module = Module::empty();
    let bytes = write_header_and_patterns(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let patterns = decode_xm_patterns(&bytes, &header).unwrap();

    assert_eq!(pattern_headers.len(), module.patterns.len());
    assert_eq!(
        pattern_headers[0].header_length,
        XM_WRITER_PATTERN_HEADER_LEN
    );
    assert_eq!(
        pattern_headers[0].packing_type,
        XM_WRITER_PATTERN_PACKING_TYPE
    );
    assert_eq!(
        pattern_headers[0].packed_data_len,
        XM_WRITER_EMPTY_PATTERN_DATA_LEN
    );
    assert_eq!(patterns[0].rows(), module.patterns[0].rows());
    assert_eq!(patterns[0].channels(), module.header.channel_count);
}

#[test]
fn writes_empty_instrument_headers_without_sample_data() {
    let module = Module::empty();
    let bytes = write_header_patterns_and_instruments(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    let section = parse_xm_instruments(&bytes, &header, instrument_offset).unwrap();

    assert_eq!(section.instruments.len(), module.instruments.len());
    assert_eq!(section.next_offset, bytes.len());
    assert!(section.instruments.iter().all(|instrument| {
        instrument.header_size == XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE
            && instrument.sample_count == 0
            && instrument.sample_header_size.is_none()
            && instrument.note_sample_map.is_none()
    }));
}

#[test]
fn writes_extended_zero_sample_instruments_when_metadata_must_roundtrip() {
    let mut module = Module::empty();
    let mut instrument = Instrument::empty(XM_WRITER_FIRST_INSTRUMENT_INDEX);
    instrument.name = InstrumentName::new(XM_WRITER_TEST_INSTRUMENT_NAME);
    instrument.sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    instrument.note_sample_map = vec![None; MAX_XM_NOTES as usize];
    instrument.volume_envelope = test_envelope();
    instrument.panning_envelope = test_envelope();
    instrument.vibrato = Vibrato {
        waveform: XM_WRITER_TEST_VIBRATO_WAVEFORM,
        sweep: XM_WRITER_TEST_VIBRATO_SWEEP,
        depth: XM_WRITER_TEST_VIBRATO_DEPTH,
        rate: XM_WRITER_TEST_VIBRATO_RATE,
    };
    instrument.volume_fadeout = XM_WRITER_TEST_VOLUME_FADEOUT;
    module.instruments = vec![instrument];

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let instrument = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];

    assert_eq!(section.instruments.len(), XM_WRITER_SINGLE_INSTRUMENT_COUNT);
    assert_eq!(section.next_offset, bytes.len());
    assert_eq!(instrument.header_size, XM_WRITER_INSTRUMENT_HEADER_SIZE);
    assert_eq!(instrument.name, XM_WRITER_TEST_INSTRUMENT_NAME);
    assert_eq!(instrument.sample_count, XM_WRITER_EMPTY_SAMPLE_COUNT);
    assert_eq!(
        instrument.sample_header_size,
        Some(XM_WRITER_SAMPLE_HEADER_SIZE)
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.panning_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.vibrato_type,
        Some(XM_WRITER_TEST_VIBRATO_WAVEFORM)
    );
    assert_eq!(instrument.vibrato_sweep, Some(XM_WRITER_TEST_VIBRATO_SWEEP));
    assert_eq!(instrument.vibrato_depth, Some(XM_WRITER_TEST_VIBRATO_DEPTH));
    assert_eq!(instrument.vibrato_rate, Some(XM_WRITER_TEST_VIBRATO_RATE));
    assert_eq!(
        instrument.volume_fadeout,
        Some(XM_WRITER_TEST_VOLUME_FADEOUT)
    );
    assert!(instrument.samples.is_empty());
}

#[test]
fn writes_instrument_metadata_and_empty_sample_headers() {
    let module = module_with_one_named_empty_sample();
    let bytes = write_header_patterns_and_instruments(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    let section = parse_xm_instruments(&bytes, &header, instrument_offset).unwrap();
    let instrument = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];
    let sample = &instrument.samples[XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(section.instruments.len(), XM_WRITER_SINGLE_INSTRUMENT_COUNT);
    assert_eq!(section.next_offset, bytes.len());
    assert_eq!(instrument.header_size, XM_WRITER_INSTRUMENT_HEADER_SIZE);
    assert_eq!(instrument.name, XM_WRITER_TEST_INSTRUMENT_NAME);
    assert_eq!(instrument.sample_count, XM_WRITER_SINGLE_SAMPLE_COUNT);
    assert_eq!(
        instrument.sample_header_size,
        Some(XM_WRITER_SAMPLE_HEADER_SIZE)
    );
    assert_eq!(
        instrument.note_sample_map.as_ref().unwrap(),
        &vec![XM_WRITER_EMPTY_OPERAND; MAX_XM_NOTES as usize]
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().point_count,
        XM_WRITER_TEST_ENVELOPE_POINT_COUNT
    );
    assert_eq!(
        instrument.volume_envelope.as_ref().unwrap().flags,
        XM_WRITER_TEST_ENVELOPE_FLAG
    );
    assert_eq!(
        instrument.panning_envelope.as_ref().unwrap().points[0],
        rustytracker_xm::XmEnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }
    );
    assert_eq!(
        instrument.vibrato_type,
        Some(XM_WRITER_TEST_VIBRATO_WAVEFORM)
    );
    assert_eq!(instrument.vibrato_sweep, Some(XM_WRITER_TEST_VIBRATO_SWEEP));
    assert_eq!(instrument.vibrato_depth, Some(XM_WRITER_TEST_VIBRATO_DEPTH));
    assert_eq!(instrument.vibrato_rate, Some(XM_WRITER_TEST_VIBRATO_RATE));
    assert_eq!(
        instrument.volume_fadeout,
        Some(XM_WRITER_TEST_VOLUME_FADEOUT)
    );

    assert_eq!(sample.length, 0);
    assert_eq!(sample.name, XM_WRITER_TEST_SAMPLE_NAME);
    assert_eq!(sample.volume_64, XM_WRITER_TEST_SAMPLE_VOLUME_64);
    assert_eq!(sample.volume, XM_WRITER_TEST_SAMPLE_VOLUME_255);
    assert_eq!(sample.panning, XM_WRITER_TEST_SAMPLE_PANNING);
    assert_eq!(sample.finetune, XM_WRITER_TEST_SAMPLE_FINETUNE);
    assert_eq!(sample.relative_note, XM_WRITER_TEST_SAMPLE_RELATIVE_NOTE);
    assert_eq!(sample.sample_type, XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE);
    assert_eq!(sample.loop_kind, SampleLoopKind::Forward);
}

#[test]
fn roundtrips_nonzero_core_sample_indexes_as_xm_local_slots() {
    let mut module = Module::empty();
    let mut instrument = Instrument::empty(XM_WRITER_FIRST_INSTRUMENT_INDEX);
    let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    sample_slots[XM_WRITER_FIRST_SAMPLE_INDEX] = Some(XM_WRITER_SECOND_SAMPLE_INDEX);
    instrument.sample_slots = sample_slots;
    instrument.note_sample_map = vec![Some(XM_WRITER_SECOND_SAMPLE_INDEX); MAX_XM_NOTES as usize];
    instrument.volume_fadeout = XM_WRITER_TEST_VOLUME_FADEOUT;

    let expected_local_sample = Sample {
        name: SampleName::new(XM_WRITER_TEST_SAMPLE_NAME),
        length: XM_WRITER_TEST_SAMPLE_FRAME_COUNT_8,
        volume: XM_WRITER_TEST_SAMPLE_VOLUME_255,
        panning: XM_WRITER_TEST_SAMPLE_PANNING,
        volume_fadeout: XM_WRITER_TEST_VOLUME_FADEOUT,
        data: SampleData::Pcm8(XM_WRITER_TEST_SAMPLE_VALUES_8.to_vec()),
        ..Sample::default()
    };

    module.instruments = vec![instrument];
    module.samples = vec![Sample::default(), expected_local_sample.clone()];

    let written = write_xm_module(&module).unwrap();
    let reparsed = parse_xm_module(&written).unwrap();
    let instrument = &reparsed.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX];

    assert_eq!(
        instrument.sample_slots[XM_WRITER_FIRST_SAMPLE_INDEX],
        Some(XM_WRITER_FIRST_SAMPLE_INDEX)
    );
    assert!(instrument
        .sample_slots
        .iter()
        .skip(XM_WRITER_SECOND_SAMPLE_INDEX)
        .all(Option::is_none));
    assert!(instrument
        .note_sample_map
        .iter()
        .all(|sample_index| *sample_index == Some(XM_WRITER_FIRST_SAMPLE_INDEX)));
    assert_eq!(reparsed.samples.len(), SAMPLES_PER_INSTRUMENT);
    assert_eq!(
        sample_roundtrip_summary(&reparsed.samples[XM_WRITER_FIRST_SAMPLE_INDEX]),
        sample_roundtrip_summary(&expected_local_sample)
    );
    assert_eq!(
        reparsed.samples[XM_WRITER_SECOND_SAMPLE_INDEX],
        Sample::default()
    );
}

#[test]
fn writes_8_bit_sample_payloads_with_delta_encoding() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_TEST_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_length = XM_WRITER_TEST_SAMPLE_LOOP_LENGTH;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::Pcm8(XM_WRITER_TEST_SAMPLE_VALUES_8.to_vec());

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let sample = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX].samples
        [XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(
        &bytes[bytes.len() - XM_WRITER_TEST_SAMPLE_DELTAS_8.len()..],
        XM_WRITER_TEST_SAMPLE_DELTAS_8
    );
    assert_eq!(sample.length, XM_WRITER_TEST_SAMPLE_VALUES_8.len() as u32);
    assert_eq!(sample.loop_start, XM_WRITER_TEST_SAMPLE_LOOP_START);
    assert_eq!(sample.loop_length, XM_WRITER_TEST_SAMPLE_LOOP_LENGTH);
    assert_eq!(
        sample.decoded_data.as_i8().unwrap(),
        XM_WRITER_TEST_SAMPLE_VALUES_8
    );
}

#[test]
fn writes_16_bit_sample_payloads_with_delta_encoding() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_TEST_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_length = XM_WRITER_TEST_SAMPLE_LOOP_LENGTH;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::Pcm16(XM_WRITER_TEST_SAMPLE_VALUES_16.to_vec());

    let bytes = write_header_patterns_and_instruments(&module);
    let section = parse_written_instruments(&bytes);
    let sample = &section.instruments[XM_WRITER_FIRST_INSTRUMENT_INDEX].samples
        [XM_WRITER_FIRST_SAMPLE_INDEX];

    assert_eq!(
        &bytes[bytes.len() - XM_WRITER_TEST_SAMPLE_DELTAS_16.len()..],
        XM_WRITER_TEST_SAMPLE_DELTAS_16
    );
    assert_eq!(
        sample.length,
        (XM_WRITER_TEST_SAMPLE_VALUES_16.len() * XM_WRITER_BYTES_PER_16_BIT_SAMPLE) as u32
    );
    assert_eq!(sample.sample_type, XM_WRITER_FORWARD_16_BIT_SAMPLE_TYPE);
    assert_eq!(
        sample.loop_start,
        XM_WRITER_TEST_SAMPLE_LOOP_START * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u32
    );
    assert_eq!(
        sample.loop_length,
        XM_WRITER_TEST_SAMPLE_LOOP_LENGTH * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u32
    );
    assert_eq!(sample.loop_start_frames, XM_WRITER_TEST_SAMPLE_LOOP_START);
    assert_eq!(sample.loop_length_frames, XM_WRITER_TEST_SAMPLE_LOOP_LENGTH);
    assert_eq!(
        sample.decoded_data.as_i16().unwrap(),
        XM_WRITER_TEST_SAMPLE_VALUES_16
    );
}

#[test]
fn rejects_16_bit_sample_loop_offsets_that_do_not_fit_xm_u32_fields() {
    let mut module = module_with_one_named_empty_sample();
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].loop_start = XM_WRITER_OVERLONG_SAMPLE_LOOP_START;
    module.samples[XM_WRITER_FIRST_SAMPLE_INDEX].data =
        SampleData::Pcm16(XM_WRITER_TEST_SAMPLE_VALUES_16.to_vec());

    assert_eq!(
        write_xm_instruments(&module).unwrap_err(),
        XmWriteError::SampleFieldTooLarge {
            instrument_index: XM_WRITER_FIRST_INSTRUMENT_INDEX,
            sample_index: XM_WRITER_FIRST_SAMPLE_INDEX,
            field: XmSampleField::LoopStart,
            value: XM_WRITER_OVERLONG_16_BIT_LOOP_START_BYTE_LEN,
            maximum: XM_WRITER_U32_FIELD_MAX,
        }
    );
}

#[test]
fn writes_simple_unpacked_pattern_cells() {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    let mut pattern = Pattern::new(
        XM_WRITER_TEST_ROWS,
        XM_WRITER_TEST_CHANNELS,
        XM_WRITER_TEST_EFFECT_SLOTS,
    );
    pattern
        .set_cell(
            0,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                ..PatternCell::default()
            },
        )
        .unwrap();
    module.patterns = vec![pattern];

    let bytes = write_header_and_patterns(&module);
    let header = parse_xm_header(&bytes).unwrap();
    let patterns = decode_xm_patterns(&bytes, &header).unwrap();
    let cell = patterns[0].cell(0, 0).unwrap();

    assert_eq!(cell.note, Note::Key(XM_WRITER_TEST_NOTE));
    assert_eq!(cell.instrument, XM_WRITER_TEST_INSTRUMENT);
    assert_eq!(cell.effects, PatternCell::default().effects);
}

#[test]
fn writes_internal_arpeggio_back_to_xm_effect_zero() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND
        )
    );
}

#[test]
fn writes_internal_extended_effects_back_to_xm_e_commands() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_SOURCE_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_SOURCE_OPERAND
        )
    );
}

#[test]
fn does_not_relocate_mixed_direction_volume_slide_to_volume_column() {
    let bytes = write_single_cell_pattern(vec![
        effect(
            XM_WRITER_VOLUME_SLIDE_EFFECT,
            XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
        ),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_VOLUME_SLIDE_EFFECT,
            XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects,
        vec![
            EffectCommand::default(),
            effect(
                XM_WRITER_VOLUME_SLIDE_EFFECT,
                XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
            ),
        ]
    );
}

#[test]
fn writes_internal_fine_volume_slides_to_xm_volume_column_when_effect_column_is_needed() {
    for (fine_effect, fine_operand, volume_column) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_COLUMN,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_COLUMN,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(fine_effect, fine_operand),
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                volume_column,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "fine slide {fine_effect:#04x}/{fine_operand:#04x}"
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                effect(fine_effect, fine_operand),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "fine slide {fine_effect:#04x}/{fine_operand:#04x}"
        );
    }
}

#[test]
fn does_not_backfill_internal_fine_volume_slides_from_later_effect_slots() {
    for (fine_effect, fine_operand) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
            effect(fine_effect, fine_operand),
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "fine slide {fine_effect:#04x}/{fine_operand:#04x}"
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "fine slide {fine_effect:#04x}/{fine_operand:#04x}"
        );
    }
}

#[test]
fn writes_zero_operand_internal_fine_volume_slides_to_effect_column_for_roundtrip_symmetry() {
    for (fine_effect, extended_operand) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_EXTENDED_OPERAND,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_EXTENDED_OPERAND,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![effect(
            fine_effect,
            XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND,
        )]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_EXTENDED_EFFECT,
                extended_operand,
            ],
            "fine slide {fine_effect:#04x}"
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects[1],
            effect(fine_effect, XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND),
            "fine slide {fine_effect:#04x}"
        );
    }
}

#[test]
fn writes_internal_extra_fine_portamento_back_to_xm_21() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
        )
    );
}

#[test]
fn writes_full_scale_core_volume_back_to_xm_volume_operand() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_VOLUME_EFFECT,
            XM_WRITER_FULL_VOLUME_64,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255)
    );
}

#[test]
fn writes_relocatable_first_effect_to_volume_column_when_effect_column_is_needed() {
    let bytes = write_single_cell_pattern(vec![
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_FULL_VOLUME_COLUMN,
            XM_WRITER_XM_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects,
        vec![
            effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND
            ),
        ]
    );
}

#[test]
fn writes_first_panning_effect_to_volume_column() {
    let bytes = write_single_cell_pattern(vec![
        effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255),
        EffectCommand::default(),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_CENTER_PANNING_COLUMN,
            XM_WRITER_EMPTY_EFFECT,
            XM_WRITER_EMPTY_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[0],
        effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255)
    );
}

#[test]
fn writes_zero_operand_slides_to_effect_column_for_roundtrip_symmetry() {
    for slide_effect in [
        XM_WRITER_VOLUME_SLIDE_EFFECT,
        XM_WRITER_PANNING_SLIDE_EFFECT,
    ] {
        let bytes = write_single_cell_pattern(vec![effect(slide_effect, XM_WRITER_EMPTY_OPERAND)]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                slide_effect,
                XM_WRITER_EMPTY_OPERAND,
            ],
            "slide effect {slide_effect:#04x}"
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects[1],
            effect(slide_effect, XM_WRITER_EMPTY_OPERAND),
            "slide effect {slide_effect:#04x}"
        );
    }
}

#[test]
fn does_not_relocate_lossy_effects_to_volume_column_when_effect_column_is_occupied() {
    for lossy_effect in [
        effect(
            XM_WRITER_TONE_PORTAMENTO_EFFECT,
            XM_WRITER_LOW_NIBBLE_TONE_PORTAMENTO_OPERAND,
        ),
        effect(XM_WRITER_VOLUME_SLIDE_EFFECT, XM_WRITER_EMPTY_OPERAND),
        effect(XM_WRITER_PANNING_SLIDE_EFFECT, XM_WRITER_EMPTY_OPERAND),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
            lossy_effect,
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "lossy effect {:#04x}/{:#04x}",
            lossy_effect.effect,
            lossy_effect.operand
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "lossy effect {:#04x}/{:#04x}",
            lossy_effect.effect,
            lossy_effect.operand
        );
    }
}

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../MilkyTracker/resources/music")
        .join(file_name)
}

fn write_header_and_patterns(module: &Module) -> Vec<u8> {
    let mut bytes = write_xm_header(module).unwrap();
    bytes.extend_from_slice(&write_xm_patterns(module).unwrap());
    bytes
}

fn write_header_patterns_and_instruments(module: &Module) -> Vec<u8> {
    let mut bytes = write_header_and_patterns(module);
    bytes.extend_from_slice(&write_xm_instruments(module).unwrap());
    bytes
}

fn parse_written_instruments(bytes: &[u8]) -> rustytracker_xm::XmInstrumentSection {
    let header = parse_xm_header(bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;

    parse_xm_instruments(bytes, &header, instrument_offset).unwrap()
}

fn module_with_one_named_empty_sample() -> Module {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    let mut instrument = Instrument::empty(XM_WRITER_FIRST_INSTRUMENT_INDEX);
    let mut sample_slots = vec![None; SAMPLES_PER_INSTRUMENT];
    sample_slots[XM_WRITER_FIRST_SAMPLE_INDEX] = Some(XM_WRITER_FIRST_SAMPLE_INDEX);

    instrument.name = InstrumentName::new(XM_WRITER_TEST_INSTRUMENT_NAME);
    instrument.sample_slots = sample_slots;
    instrument.note_sample_map = vec![Some(XM_WRITER_FIRST_SAMPLE_INDEX); MAX_XM_NOTES as usize];
    instrument.volume_envelope = test_envelope();
    instrument.panning_envelope = test_envelope();
    instrument.vibrato = Vibrato {
        waveform: XM_WRITER_TEST_VIBRATO_WAVEFORM,
        sweep: XM_WRITER_TEST_VIBRATO_SWEEP,
        depth: XM_WRITER_TEST_VIBRATO_DEPTH,
        rate: XM_WRITER_TEST_VIBRATO_RATE,
    };
    instrument.volume_fadeout = XM_WRITER_TEST_VOLUME_FADEOUT;

    module.instruments = vec![instrument];
    module.samples = vec![Sample {
        name: SampleName::new(XM_WRITER_TEST_SAMPLE_NAME),
        volume: XM_WRITER_TEST_SAMPLE_VOLUME_255,
        panning: XM_WRITER_TEST_SAMPLE_PANNING,
        loop_kind: SampleLoopKind::Forward,
        finetune: XM_WRITER_TEST_SAMPLE_FINETUNE,
        relative_note: XM_WRITER_TEST_SAMPLE_RELATIVE_NOTE,
        ..Sample::default()
    }];

    module
}

fn test_envelope() -> Envelope {
    Envelope {
        points: vec![EnvelopePoint {
            frame: XM_WRITER_TEST_ENVELOPE_FRAME,
            value: XM_WRITER_TEST_ENVELOPE_VALUE,
        }],
        point_count: XM_WRITER_TEST_ENVELOPE_POINT_COUNT,
        sustain_point: XM_WRITER_EMPTY_ENVELOPE_POINT_INDEX,
        loop_start_point: XM_WRITER_EMPTY_ENVELOPE_POINT_INDEX,
        loop_end_point: XM_WRITER_EMPTY_ENVELOPE_POINT_INDEX,
        flags: XM_WRITER_TEST_ENVELOPE_FLAG,
    }
}

fn write_single_cell_pattern(effects: Vec<EffectCommand>) -> Vec<u8> {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    let mut pattern = Pattern::new(
        XM_WRITER_TEST_ROWS,
        XM_WRITER_TEST_CHANNELS,
        effects.len() as u8,
    );
    pattern
        .set_cell(
            0,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                effects,
            },
        )
        .unwrap();
    module.patterns = vec![pattern];

    write_header_and_patterns(&module)
}

fn roundtrip_single_cell_module(effects: Vec<EffectCommand>) -> PatternCell {
    let mut module = Module::empty_with_channels(XM_WRITER_TEST_CHANNELS).unwrap();
    let mut pattern = Pattern::new(
        XM_WRITER_TEST_ROWS,
        XM_WRITER_TEST_CHANNELS,
        XM_WRITER_TEST_EFFECT_SLOTS,
    );
    pattern
        .set_cell(
            0,
            0,
            PatternCell {
                note: Note::Key(XM_WRITER_TEST_NOTE),
                instrument: XM_WRITER_TEST_INSTRUMENT,
                effects,
            },
        )
        .unwrap();
    module.patterns = vec![pattern];

    let written = write_xm_module(&module).unwrap();
    let reparsed = parse_xm_module(&written).unwrap();

    reparsed.patterns[0].cell(0, 0).unwrap().clone()
}

fn first_raw_pattern_cell(bytes: &[u8]) -> &[u8] {
    let header = parse_xm_header(bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(bytes, &header).unwrap();
    let offset = pattern_headers[0].packed_data_offset;

    &bytes[offset..offset + XM_WRITER_UNPACKED_CELL_LEN]
}

fn first_decoded_cell(bytes: &[u8]) -> PatternCell {
    let header = parse_xm_header(bytes).unwrap();
    let patterns = decode_xm_patterns(bytes, &header).unwrap();

    patterns[0].cell(0, 0).unwrap().clone()
}

fn effect(effect: u8, operand: u8) -> EffectCommand {
    EffectCommand { effect, operand }
}

fn module_roundtrip_summary(module: &Module) -> ModuleRoundtripSummary {
    ModuleRoundtripSummary {
        header: module.header.clone(),
        orders: module.orders.clone(),
        patterns: module
            .patterns
            .iter()
            .map(pattern_roundtrip_summary)
            .collect(),
        instruments: module
            .instruments
            .iter()
            .map(instrument_roundtrip_summary)
            .collect(),
        samples: module
            .samples
            .iter()
            .map(sample_roundtrip_summary)
            .collect(),
    }
}

fn assert_module_roundtrip_summary_eq(
    actual: &ModuleRoundtripSummary,
    expected: &ModuleRoundtripSummary,
    fixture: &str,
) {
    assert_eq!(actual.header, expected.header, "{fixture} header");
    assert_eq!(actual.orders, expected.orders, "{fixture} orders");
    assert_eq!(
        actual.patterns.len(),
        expected.patterns.len(),
        "{fixture} pattern count"
    );
    assert_eq!(
        actual.instruments.len(),
        expected.instruments.len(),
        "{fixture} instrument count"
    );
    assert_eq!(
        actual.samples.len(),
        expected.samples.len(),
        "{fixture} sample count"
    );

    for (index, (actual, expected)) in actual
        .patterns
        .iter()
        .zip(expected.patterns.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{fixture} pattern {index}");
    }

    for (index, (actual, expected)) in actual
        .instruments
        .iter()
        .zip(expected.instruments.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{fixture} instrument {index}");
    }

    for (index, (actual, expected)) in actual
        .samples
        .iter()
        .zip(expected.samples.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{fixture} sample {index}");
    }
}

fn pattern_roundtrip_summary(pattern: &Pattern) -> PatternRoundtripSummary {
    let mut non_empty_cells = 0;
    let mut checksum = XM_WRITER_FNV_OFFSET;

    for row in 0..pattern.rows() {
        for channel in 0..pattern.channels() {
            let cell = pattern
                .cell(channel, row)
                .expect("summary walks cells inside pattern bounds");

            if cell.note != Note::Empty
                || cell.instrument != XM_WRITER_EMPTY_INSTRUMENT
                || cell
                    .effects
                    .iter()
                    .any(|effect| *effect != EffectCommand::default())
            {
                non_empty_cells += 1;
            }

            checksum = fnv_byte(checksum, cell.note.raw());
            checksum = fnv_byte(checksum, cell.instrument);

            for effect in &cell.effects {
                checksum = fnv_byte(checksum, effect.effect);
                checksum = fnv_byte(checksum, effect.operand);
            }
        }
    }

    PatternRoundtripSummary {
        rows: pattern.rows(),
        channels: pattern.channels(),
        effect_slots: pattern.effect_slots(),
        non_empty_cells,
        expanded_cell_checksum: checksum,
    }
}

fn sample_roundtrip_summary(sample: &Sample) -> SampleRoundtripSummary {
    SampleRoundtripSummary {
        name: sample.name.clone(),
        length: sample.length,
        loop_start: sample.loop_start,
        loop_length: sample.loop_length,
        loop_kind: sample.loop_kind,
        volume: sample.volume,
        panning: sample.panning,
        flags: sample.flags,
        volume_fadeout: sample.volume_fadeout,
        sample_type: sample.sample_type,
        finetune: sample.finetune,
        relative_note: sample.relative_note,
        data: sample_data_roundtrip_summary(&sample.data),
    }
}

fn instrument_roundtrip_summary(instrument: &Instrument) -> InstrumentRoundtripSummary {
    InstrumentRoundtripSummary {
        name: instrument.name.clone(),
        sample_slots: instrument.sample_slots.clone(),
        note_sample_map_checksum: checksum_optional_usize(&instrument.note_sample_map),
        volume_envelope: instrument.volume_envelope.clone(),
        panning_envelope: instrument.panning_envelope.clone(),
        vibrato: instrument.vibrato,
        volume_fadeout: instrument.volume_fadeout,
    }
}

fn sample_data_roundtrip_summary(data: &SampleData) -> SampleDataRoundtripSummary {
    match data {
        SampleData::Empty => SampleDataRoundtripSummary::Empty,
        SampleData::Pcm8(values) => SampleDataRoundtripSummary::Pcm8 {
            frames: values.len(),
            checksum: checksum_i8(values),
            prefix: values
                .iter()
                .take(XM_WRITER_SAMPLE_PREFIX_FRAMES)
                .copied()
                .collect(),
        },
        SampleData::Pcm16(values) => SampleDataRoundtripSummary::Pcm16 {
            frames: values.len(),
            checksum: checksum_i16(values),
            prefix: values
                .iter()
                .take(XM_WRITER_SAMPLE_PREFIX_FRAMES)
                .copied()
                .collect(),
        },
    }
}

fn checksum_i8(values: &[i8]) -> u64 {
    values.iter().fold(XM_WRITER_FNV_OFFSET, |checksum, value| {
        fnv_byte(checksum, *value as u8)
    })
}

fn checksum_i16(values: &[i16]) -> u64 {
    values.iter().fold(XM_WRITER_FNV_OFFSET, |checksum, value| {
        let bytes = value.to_le_bytes();
        fnv_byte(fnv_byte(checksum, bytes[0]), bytes[1])
    })
}

fn checksum_optional_usize(values: &[Option<usize>]) -> u64 {
    values
        .iter()
        .fold(XM_WRITER_FNV_OFFSET, |checksum, value| match value {
            Some(value) => checksum_usize(fnv_byte(checksum, XM_WRITER_OPTION_SOME_TAG), *value),
            None => fnv_byte(checksum, XM_WRITER_OPTION_NONE_TAG),
        })
}

fn checksum_usize(mut checksum: u64, value: usize) -> u64 {
    for byte in value.to_le_bytes() {
        checksum = fnv_byte(checksum, byte);
    }

    checksum
}

fn fnv_byte(checksum: u64, byte: u8) -> u64 {
    (checksum ^ byte as u64).wrapping_mul(XM_WRITER_FNV_PRIME)
}
