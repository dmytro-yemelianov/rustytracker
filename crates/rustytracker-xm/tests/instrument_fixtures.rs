use std::fs;
use std::path::PathBuf;

use rustytracker_xm::{
    parse_xm_header, parse_xm_instruments, parse_xm_pattern_headers, XmParseError,
};

#[derive(Debug)]
struct ExpectedInstrumentSection {
    file_name: &'static str,
    instrument_count: usize,
    instrument_end: usize,
    empty_instrument_count: usize,
    sample_count: usize,
    sample_data_len: usize,
    max_samples_per_instrument: u16,
    sample_counts: &'static [u16],
    first_names: &'static [&'static str],
    first_sample: ExpectedSample,
    first_instrument_volume_fadeout: Option<u16>,
    first_instrument_vibrato_depth: Option<u8>,
}

#[derive(Debug)]
struct ExpectedSample {
    instrument_index: usize,
    sample_index: usize,
    length: u32,
    loop_start: u32,
    loop_length: u32,
    volume_64: u8,
    volume: u8,
    finetune: i8,
    sample_type: u8,
    panning: u8,
    relative_note: i8,
    name: &'static str,
}

const FIXTURES: &[ExpectedInstrumentSection] = &[
    ExpectedInstrumentSection {
        file_name: "milky.xm",
        instrument_count: 7,
        instrument_end: 28_716,
        empty_instrument_count: 3,
        sample_count: 4,
        sample_data_len: 10_185,
        max_samples_per_instrument: 1,
        sample_counts: &[1, 1, 1, 1, 0, 0, 0],
        first_names: &[
            "",
            " raina . CoolPHat",
            "",
            " trying out",
            " milkytracker",
            "",
            " 2005",
        ],
        first_sample: ExpectedSample {
            instrument_index: 0,
            sample_index: 0,
            length: 997,
            loop_start: 613,
            loop_length: 384,
            volume_64: 64,
            volume: 255,
            finetune: -28,
            sample_type: 1,
            panning: 128,
            relative_note: -7,
            name: "beng",
        },
        first_instrument_volume_fadeout: Some(736),
        first_instrument_vibrato_depth: Some(0),
    },
    ExpectedInstrumentSection {
        file_name: "slumberjack.xm",
        instrument_count: 7,
        instrument_end: 24_689,
        empty_instrument_count: 4,
        sample_count: 7,
        sample_data_len: 9_099,
        max_samples_per_instrument: 5,
        sample_counts: &[1, 1, 5, 0, 0, 0, 0],
        first_names: &[
            "",
            " raina",
            "",
            " made parallel in Ft2",
            " and MilkyTracker",
            "",
            " 2005",
        ],
        first_sample: ExpectedSample {
            instrument_index: 0,
            sample_index: 0,
            length: 446,
            loop_start: 414,
            loop_length: 32,
            volume_64: 48,
            volume: 192,
            finetune: 50,
            sample_type: 1,
            panning: 96,
            relative_note: 0,
            name: "",
        },
        first_instrument_volume_fadeout: Some(480),
        first_instrument_vibrato_depth: Some(16),
    },
    ExpectedInstrumentSection {
        file_name: "sv_ttt.xm",
        instrument_count: 44,
        instrument_end: 35_849,
        empty_instrument_count: 38,
        sample_count: 6,
        sample_data_len: 10_871,
        max_samples_per_instrument: 1,
        sample_counts: &[
            1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
        first_names: &[
            "svenzzon of titan",
            "",
            "The Titan Turrican",
            "",
            "Made as intro tune",
            "for titan artpack2",
            "and for newest version",
            "of MilkyTracker!",
        ],
        first_sample: ExpectedSample {
            instrument_index: 0,
            sample_index: 0,
            length: 132,
            loop_start: 0,
            loop_length: 132,
            volume_64: 50,
            volume: 200,
            finetune: 0,
            sample_type: 1,
            panning: 128,
            relative_note: 7,
            name: "",
        },
        first_instrument_volume_fadeout: Some(0),
        first_instrument_vibrato_depth: Some(0),
    },
    ExpectedInstrumentSection {
        file_name: "theday.xm",
        instrument_count: 7,
        instrument_end: 75_000,
        empty_instrument_count: 0,
        sample_count: 7,
        sample_data_len: 28_466,
        max_samples_per_instrument: 1,
        sample_counts: &[1, 1, 1, 1, 1, 1, 1],
        first_names: &[
            "2                    0",
            " Ampli,",
            "       Kmuland &",
            "                raina",
            "",
            "  Trio Internacional",
            "0                    8",
        ],
        first_sample: ExpectedSample {
            instrument_index: 0,
            sample_index: 0,
            length: 87,
            loop_start: 2,
            loop_length: 85,
            volume_64: 64,
            volume: 255,
            finetune: 0,
            sample_type: 1,
            panning: 100,
            relative_note: 0,
            name: "",
        },
        first_instrument_volume_fadeout: Some(0),
        first_instrument_vibrato_depth: Some(0),
    },
    ExpectedInstrumentSection {
        file_name: "universalnetwork2_real.xm",
        instrument_count: 16,
        instrument_end: 95_071,
        empty_instrument_count: 0,
        sample_count: 16,
        sample_data_len: 67_377,
        max_samples_per_instrument: 1,
        sample_counts: &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        first_names: &[
            " ...Strobe&Kmuland...",
            "        TiTAN",
            "",
            "      25 May / 2006",
            "",
            "   Love too you all!",
            "",
            "         v1.01",
        ],
        first_sample: ExpectedSample {
            instrument_index: 0,
            sample_index: 0,
            length: 2_345,
            loop_start: 0,
            loop_length: 0,
            volume_64: 64,
            volume: 255,
            finetune: 0,
            sample_type: 0,
            panning: 128,
            relative_note: -8,
            name: "",
        },
        first_instrument_volume_fadeout: Some(0),
        first_instrument_vibrato_depth: Some(0),
    },
];

#[test]
fn parses_milkytracker_bundled_xm_instrument_sections() {
    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture.file_name)).unwrap();
        let header = parse_xm_header(&bytes).unwrap();
        let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
        let instrument_offset = pattern_headers.last().unwrap().next_offset;
        let section = parse_xm_instruments(&bytes, &header, instrument_offset).unwrap();

        assert_eq!(
            section.instruments.len(),
            fixture.instrument_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section.next_offset, fixture.instrument_end,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .filter(|instrument| instrument.sample_count == 0)
                .count(),
            fixture.empty_instrument_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .flat_map(|instrument| &instrument.samples)
                .count(),
            fixture.sample_count,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .flat_map(|instrument| &instrument.samples)
                .map(|sample| sample.length as usize)
                .sum::<usize>(),
            fixture.sample_data_len,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .map(|instrument| instrument.sample_count)
                .max()
                .unwrap_or_default(),
            fixture.max_samples_per_instrument,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .map(|instrument| instrument.sample_count)
                .collect::<Vec<_>>(),
            fixture.sample_counts,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            section
                .instruments
                .iter()
                .take(fixture.first_names.len())
                .map(|instrument| instrument.name.as_str())
                .collect::<Vec<_>>(),
            fixture.first_names,
            "{}",
            fixture.file_name
        );
        assert!(
            section
                .instruments
                .iter()
                .filter(|instrument| instrument.sample_header_size.is_some())
                .all(|instrument| instrument.sample_header_size == Some(40)),
            "{}",
            fixture.file_name
        );

        let instrument = &section.instruments[fixture.first_sample.instrument_index];
        assert_eq!(
            instrument.volume_fadeout, fixture.first_instrument_volume_fadeout,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            instrument.vibrato_depth, fixture.first_instrument_vibrato_depth,
            "{}",
            fixture.file_name
        );
        assert_eq!(
            instrument.note_sample_map.as_ref().map(Vec::len),
            instrument.sample_header_size.map(|_| 96),
            "{}",
            fixture.file_name
        );

        let sample = &instrument.samples[fixture.first_sample.sample_index];
        assert_eq!(sample.index, fixture.first_sample.sample_index);
        assert_eq!(sample.length, fixture.first_sample.length);
        assert_eq!(sample.loop_start, fixture.first_sample.loop_start);
        assert_eq!(sample.loop_length, fixture.first_sample.loop_length);
        assert_eq!(sample.volume_64, fixture.first_sample.volume_64);
        assert_eq!(sample.volume, fixture.first_sample.volume);
        assert_eq!(sample.finetune, fixture.first_sample.finetune);
        assert_eq!(sample.sample_type, fixture.first_sample.sample_type);
        assert_eq!(sample.panning, fixture.first_sample.panning);
        assert_eq!(sample.relative_note, fixture.first_sample.relative_note);
        assert_eq!(sample.name, fixture.first_sample.name);
    }
}

#[test]
fn rejects_truncated_instrument_header() {
    let bytes = fs::read(fixture_path("milky.xm")).unwrap();
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    let truncated = &bytes[..instrument_offset + 2];

    assert!(matches!(
        parse_xm_instruments(truncated, &header, instrument_offset),
        Err(XmParseError::InstrumentHeaderTooShort {
            instrument_index: 0,
            ..
        })
    ));
}

#[test]
fn rejects_truncated_sample_data() {
    let mut bytes = fs::read(fixture_path("milky.xm")).unwrap();
    let header = parse_xm_header(&bytes).unwrap();
    let pattern_headers = parse_xm_pattern_headers(&bytes, &header).unwrap();
    let instrument_offset = pattern_headers.last().unwrap().next_offset;
    bytes.truncate(18_531);

    assert!(matches!(
        parse_xm_instruments(&bytes, &header, instrument_offset),
        Err(XmParseError::SampleDataTooShort {
            instrument_index: 0,
            sample_index: 0,
            ..
        })
    ));
}

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../MilkyTracker/resources/music")
        .join(file_name)
}
