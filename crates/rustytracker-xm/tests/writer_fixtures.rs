pub use std::fs;

pub use rustytracker_core::{
    EffectCommand, Envelope, EnvelopePoint, FrequencyTable, Instrument, InstrumentName, Module,
    ModuleTitle, Note, Pattern, PatternCell, Sample, SampleData, SampleLoopKind, SampleName,
    Vibrato, MAX_XM_NOTES, SAMPLES_PER_INSTRUMENT,
};
pub use rustytracker_test_support::{
    milkytracker_fixture_path as fixture_path,
    milkytracker_fixtures_available as fixtures_available,
};
pub use rustytracker_xm::{
    decode_xm_patterns, parse_xm_header, parse_xm_instruments, parse_xm_module,
    parse_xm_pattern_headers, write_xm_header, write_xm_instruments, write_xm_module,
    write_xm_patterns, XmSampleField, XmWriteError,
};

pub const FIXTURES: &[&str] = &[
    "milky.xm",
    "slumberjack.xm",
    "sv_ttt.xm",
    "theday.xm",
    "universalnetwork2_real.xm",
];
pub const XM_WRITER_TEST_VERSION: u16 = 0x0104;
pub const XM_WRITER_TEST_HEADER_SIZE: u32 = 276;
pub const XM_WRITER_TEST_ORDERS: &[u8] = &[0, 0, 0];
pub const XM_WRITER_FNV_OFFSET: u64 = 0xcbf29ce484222325;
pub const XM_WRITER_FNV_PRIME: u64 = 0x100000001b3;
pub const XM_WRITER_OPTION_NONE_TAG: u8 = 0;
pub const XM_WRITER_OPTION_SOME_TAG: u8 = 1;
pub const XM_WRITER_SAMPLE_PREFIX_FRAMES: usize = 16;
pub const XM_WRITER_MAX_ACTIVE_ORDERS: usize = rustytracker_core::MAX_ACTIVE_ORDERS;
pub const XM_WRITER_OVERLONG_ORDER_LEN: usize = XM_WRITER_MAX_ACTIVE_ORDERS + 1;
pub const XM_WRITER_TEST_ROWS: u16 = 1;
pub const XM_WRITER_TEST_CHANNELS: u16 = 2;
pub const XM_WRITER_TEST_EFFECT_SLOTS: u8 = 2;
pub const XM_WRITER_TEST_NOTE: u8 = 49;
pub const XM_WRITER_TEST_INSTRUMENT: u8 = 3;
pub const XM_WRITER_EMPTY_INSTRUMENT: u8 = 0;
pub const XM_WRITER_PATTERN_HEADER_LEN: u32 = 9;
pub const XM_WRITER_PATTERN_PACKING_TYPE: u8 = 0;
pub const XM_WRITER_EMPTY_PATTERN_DATA_LEN: u16 = 0;
pub const XM_WRITER_UNPACKED_CELL_LEN: usize = 5;
pub const XM_WRITER_EMPTY_EFFECT: u8 = 0;
pub const XM_WRITER_EMPTY_OPERAND: u8 = 0;
pub const XM_WRITER_TEST_EMPTY_VOLUME_COLUMN: u8 = 0;
pub const XM_WRITER_XM_ARPEGGIO_EFFECT: u8 = 0x00;
pub const XM_WRITER_INTERNAL_ARPEGGIO_EFFECT: u8 = 0x20;
pub const XM_WRITER_ARPEGGIO_OPERAND: u8 = 0x37;
pub const XM_WRITER_XM_EXTENDED_EFFECT: u8 = 0x0e;
pub const XM_WRITER_INTERNAL_EXTENDED_EFFECT: u8 = 0x3a;
pub const XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT: u8 = 0x3b;
pub const XM_WRITER_EXTENDED_SOURCE_OPERAND: u8 = 0x07;
pub const XM_WRITER_EXTENDED_OPERAND: u8 = 0xa7;
pub const XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND: u8 = 0x05;
pub const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND: u8 = 0x06;
pub const XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND: u8 = 0x00;
pub const XM_WRITER_FINE_VOLUME_SLIDE_UP_COLUMN: u8 = 0x95;
pub const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_COLUMN: u8 = 0x86;
pub const XM_WRITER_FINE_VOLUME_SLIDE_UP_EXTENDED_OPERAND: u8 = 0xa0;
pub const XM_WRITER_FINE_VOLUME_SLIDE_DOWN_EXTENDED_OPERAND: u8 = 0xb0;
pub const XM_WRITER_XM_EXTRA_FINE_PORTA_EFFECT: u8 = 0x21;
pub const XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT: u8 = 0x41;
pub const XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND: u8 = 0x05;
pub const XM_WRITER_EXTRA_FINE_PORTA_OPERAND: u8 = 0x15;
pub const XM_WRITER_VOLUME_EFFECT: u8 = 0x0c;
pub const XM_WRITER_VOLUME_SLIDE_EFFECT: u8 = 0x0a;
pub const XM_WRITER_VOLUME_SLIDE_UP_OPERAND: u8 = 0x40;
pub const XM_WRITER_VOLUME_SLIDE_DOWN_OPERAND: u8 = 0x04;
pub const XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND: u8 = 0x34;
pub const XM_WRITER_FULL_VOLUME_255: u8 = 0xff;
pub const XM_WRITER_FULL_VOLUME_64: u8 = 0x40;
pub const XM_WRITER_FULL_VOLUME_COLUMN: u8 = 0x50;
pub const XM_WRITER_VIBRATO_EFFECT: u8 = 0x04;
pub const XM_WRITER_VIBRATO_SPEED_OPERAND: u8 = 0x40;
pub const XM_WRITER_VIBRATO_DEPTH_OPERAND: u8 = 0x04;
pub const XM_WRITER_PANNING_EFFECT: u8 = 0x08;
pub const XM_WRITER_PANNING_SLIDE_EFFECT: u8 = 0x19;
pub const XM_WRITER_PANNING_SLIDE_LEFT_OPERAND: u8 = 0x04;
pub const XM_WRITER_PANNING_SLIDE_RIGHT_OPERAND: u8 = 0x40;
pub const XM_WRITER_CENTER_PANNING_255: u8 = 0x80;
pub const XM_WRITER_CENTER_PANNING_COLUMN: u8 = 0xc8;
pub const XM_WRITER_TONE_PORTAMENTO_EFFECT: u8 = 0x03;
pub const XM_WRITER_HIGH_NIBBLE_TONE_PORTAMENTO_OPERAND: u8 = 0x40;
pub const XM_WRITER_LOW_NIBBLE_TONE_PORTAMENTO_OPERAND: u8 = 0x05;
pub const XM_WRITER_EMPTY_INSTRUMENT_HEADER_SIZE: u32 = 29;
pub const XM_WRITER_INSTRUMENT_HEADER_SIZE: u32 = 263;
pub const XM_WRITER_SAMPLE_HEADER_SIZE: u32 = 40;
pub const XM_WRITER_SINGLE_INSTRUMENT_COUNT: usize = 1;
pub const XM_WRITER_EMPTY_SAMPLE_COUNT: u16 = 0;
pub const XM_WRITER_SINGLE_SAMPLE_COUNT: u16 = 1;
pub const XM_WRITER_FIRST_INSTRUMENT_INDEX: usize = 0;
pub const XM_WRITER_FIRST_SAMPLE_INDEX: usize = 0;
pub const XM_WRITER_SECOND_SAMPLE_INDEX: usize = 1;
pub const XM_WRITER_EMPTY_ENVELOPE_POINT_INDEX: u8 = 0;
pub const XM_WRITER_TEST_INSTRUMENT_NAME: &str = "lead inst";
pub const XM_WRITER_TEST_SAMPLE_NAME: &str = "sample a";
pub const XM_WRITER_TEST_ENVELOPE_FRAME: u16 = 12;
pub const XM_WRITER_TEST_ENVELOPE_VALUE: u16 = 32;
pub const XM_WRITER_TEST_ENVELOPE_POINT_COUNT: u8 = 1;
pub const XM_WRITER_TEST_ENVELOPE_FLAG: u8 = 1;
pub const XM_WRITER_TEST_VIBRATO_WAVEFORM: u8 = 2;
pub const XM_WRITER_TEST_VIBRATO_SWEEP: u8 = 3;
pub const XM_WRITER_TEST_VIBRATO_DEPTH: u8 = 16;
pub const XM_WRITER_TEST_VIBRATO_RATE: u8 = 5;
pub const XM_WRITER_TEST_VOLUME_FADEOUT: u16 = 480;
pub const XM_WRITER_TEST_SAMPLE_VOLUME_255: u8 = 192;
pub const XM_WRITER_TEST_SAMPLE_VOLUME_64: u8 = 48;
pub const XM_WRITER_TEST_SAMPLE_PANNING: u8 = 96;
pub const XM_WRITER_TEST_SAMPLE_FINETUNE: i8 = -12;
pub const XM_WRITER_TEST_SAMPLE_RELATIVE_NOTE: i8 = 7;
pub const XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE: u8 = 1;
pub const XM_WRITER_16_BIT_SAMPLE_TYPE: u8 = 0x10;
pub const XM_WRITER_FORWARD_16_BIT_SAMPLE_TYPE: u8 =
    XM_WRITER_16_BIT_SAMPLE_TYPE | XM_WRITER_FORWARD_LOOP_SAMPLE_TYPE;
pub const XM_WRITER_BYTES_PER_16_BIT_SAMPLE: usize = 2;
pub const XM_WRITER_TEST_SAMPLE_LOOP_START: u32 = 1;
pub const XM_WRITER_TEST_SAMPLE_LOOP_LENGTH: u32 = 2;
pub const XM_WRITER_TEST_SAMPLE_VALUES_8: &[i8] = &[1, 9, 22, 41];
pub const XM_WRITER_TEST_SAMPLE_FRAME_COUNT_8: u32 = XM_WRITER_TEST_SAMPLE_VALUES_8.len() as u32;
pub const XM_WRITER_TEST_SAMPLE_DELTAS_8: &[u8] = &[1, 8, 13, 19];
pub const XM_WRITER_TEST_SAMPLE_VALUES_16: &[i16] = &[1_000, 500, 750, -250];
pub const XM_WRITER_TEST_SAMPLE_DELTAS_16: &[u8] = &[0xe8, 0x03, 0x0c, 0xfe, 0xfa, 0x00, 0x18, 0xfc];
pub const XM_WRITER_OVERLONG_SAMPLE_LOOP_START: u32 = u32::MAX;
pub const XM_WRITER_U32_FIELD_MAX: u64 = u32::MAX as u64;
pub const XM_WRITER_OVERLONG_16_BIT_LOOP_START_BYTE_LEN: u64 =
    XM_WRITER_OVERLONG_SAMPLE_LOOP_START as u64 * XM_WRITER_BYTES_PER_16_BIT_SAMPLE as u64;

#[derive(Debug, PartialEq, Eq)]
pub struct ModuleRoundtripSummary {
    pub header: rustytracker_core::ModuleHeader,
    pub orders: Vec<u8>,
    pub patterns: Vec<PatternRoundtripSummary>,
    pub instruments: Vec<InstrumentRoundtripSummary>,
    pub samples: Vec<SampleRoundtripSummary>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PatternRoundtripSummary {
    pub rows: u16,
    pub channels: u16,
    pub effect_slots: u8,
    pub non_empty_cells: usize,
    pub expanded_cell_checksum: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct InstrumentRoundtripSummary {
    pub name: InstrumentName,
    pub sample_slots: Vec<Option<usize>>,
    pub note_sample_map_checksum: u64,
    pub volume_envelope: Envelope,
    pub panning_envelope: Envelope,
    pub vibrato: Vibrato,
    pub volume_fadeout: u16,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SampleRoundtripSummary {
    pub name: SampleName,
    pub length: u32,
    pub loop_start: u32,
    pub loop_length: u32,
    pub loop_kind: SampleLoopKind,
    pub volume: u8,
    pub panning: u8,
    pub flags: u8,
    pub volume_fadeout: u16,
    pub sample_type: u8,
    pub finetune: i8,
    pub relative_note: i8,
    pub data: SampleDataRoundtripSummary,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SampleDataRoundtripSummary {
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

pub fn write_header_and_patterns(module: &Module) -> Vec<u8> {
    let mut bytes = write_xm_header(module).unwrap();
    bytes.extend_from_slice(&write_xm_patterns(module).unwrap());
    bytes
}

pub fn write_header_patterns_and_instruments(module: &Module) -> Vec<u8> {
    let mut bytes = write_header_and_patterns(module);
    bytes.extend_from_slice(&write_xm_instruments(module).unwrap());
    bytes
}

pub fn parse_written_instruments(bytes: &[u8]) -> rustytracker_xm::XmInstrumentSection {
    let header = parse_xm_header(bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;

    parse_xm_instruments(bytes, &header, instrument_offset).unwrap()
}

pub fn module_with_one_named_empty_sample() -> Module {
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

pub fn test_envelope() -> Envelope {
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

pub fn write_single_cell_pattern(effects: Vec<EffectCommand>) -> Vec<u8> {
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

pub fn roundtrip_single_cell_module(effects: Vec<EffectCommand>) -> PatternCell {
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

pub fn first_raw_pattern_cell(bytes: &[u8]) -> &[u8] {
    let header = parse_xm_header(bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(bytes, &header).unwrap();
    let offset = pattern_headers[0].packed_data_offset;

    &bytes[offset..offset + XM_WRITER_UNPACKED_CELL_LEN]
}

pub fn first_decoded_cell(bytes: &[u8]) -> PatternCell {
    let header = parse_xm_header(bytes).unwrap();
    let patterns = decode_xm_patterns(bytes, &header).unwrap();

    patterns[0].cell(0, 0).unwrap().clone()
}

pub fn effect(effect: u8, operand: u8) -> EffectCommand {
    EffectCommand { effect, operand }
}

pub fn module_roundtrip_summary(module: &Module) -> ModuleRoundtripSummary {
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

pub fn assert_module_roundtrip_summary_eq(
    actual: &ModuleRoundtripSummary,
    expected: &ModuleRoundtripSummary,
    fixture: &str,
) {
    assert_eq!(actual.header, expected.header, "{} header", fixture);
    assert_eq!(actual.orders, expected.orders, "{} orders", fixture);
    assert_eq!(
        actual.patterns.len(),
        expected.patterns.len(),
        "{} pattern count",
        fixture
    );
    assert_eq!(
        actual.instruments.len(),
        expected.instruments.len(),
        "{} instrument count",
        fixture
    );
    assert_eq!(
        actual.samples.len(),
        expected.samples.len(),
        "{} sample count",
        fixture
    );

    for (index, (actual, expected)) in actual
        .patterns
        .iter()
        .zip(expected.patterns.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{} pattern {}", fixture, index);
    }

    for (index, (actual, expected)) in actual
        .instruments
        .iter()
        .zip(expected.instruments.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{} instrument {}", fixture, index);
    }

    for (index, (actual, expected)) in actual
        .samples
        .iter()
        .zip(expected.samples.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "{} sample {}", fixture, index);
    }
}

pub fn pattern_roundtrip_summary(pattern: &Pattern) -> PatternRoundtripSummary {
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

pub fn sample_roundtrip_summary(sample: &Sample) -> SampleRoundtripSummary {
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

pub fn instrument_roundtrip_summary(instrument: &Instrument) -> InstrumentRoundtripSummary {
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

pub fn sample_data_roundtrip_summary(data: &SampleData) -> SampleDataRoundtripSummary {
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

pub fn checksum_i8(values: &[i8]) -> u64 {
    values.iter().fold(XM_WRITER_FNV_OFFSET, |checksum, value| {
        fnv_byte(checksum, *value as u8)
    })
}

pub fn checksum_i16(values: &[i16]) -> u64 {
    values.iter().fold(XM_WRITER_FNV_OFFSET, |checksum, value| {
        let bytes = value.to_le_bytes();
        fnv_byte(fnv_byte(checksum, bytes[0]), bytes[1])
    })
}

pub fn checksum_optional_usize(values: &[Option<usize>]) -> u64 {
    values
        .iter()
        .fold(XM_WRITER_FNV_OFFSET, |checksum, value| match value {
            Some(value) => checksum_usize(fnv_byte(checksum, XM_WRITER_OPTION_SOME_TAG), *value),
            None => fnv_byte(checksum, XM_WRITER_OPTION_NONE_TAG),
        })
}

pub fn checksum_usize(mut checksum: u64, value: usize) -> u64 {
    for byte in value.to_le_bytes() {
        checksum = fnv_byte(checksum, byte);
    }

    checksum
}

pub fn fnv_byte(checksum: u64, byte: u8) -> u64 {
    (checksum ^ byte as u64).wrapping_mul(XM_WRITER_FNV_PRIME)
}

mod writer {
    mod module;
    pub mod patterns;
    pub mod instruments;
    pub mod samples;
    pub mod effects;
}
