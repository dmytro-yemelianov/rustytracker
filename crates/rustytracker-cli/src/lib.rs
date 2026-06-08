use std::fmt;
use std::path::Path;

use rustytracker_core::{
    Envelope, FrequencyTable, Module, Pattern, Sample, SampleData, SampleLoopKind,
};
use serde::Serialize;

const DUMP_SCHEMA_VERSION: u16 = 1;
const DUMP_FORMAT_XM: &str = "xm";
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EXPANDED_PATTERN_CELL_BYTES: usize = 6;
const SAMPLE_PREFIX_FRAMES: usize = 16;
const JSON_TRAILING_NEWLINE: &str = "\n";
const FREQUENCY_TABLE_AMIGA: &str = "amiga";
const FREQUENCY_TABLE_LINEAR: &str = "linear";
const SAMPLE_LOOP_NONE: &str = "none";
const SAMPLE_LOOP_FORWARD: &str = "forward";
const SAMPLE_LOOP_PING_PONG: &str = "ping_pong";
const SAMPLE_DATA_EMPTY: &str = "empty";
const SAMPLE_DATA_PCM8: &str = "pcm8";
const SAMPLE_DATA_PCM16: &str = "pcm16";

#[derive(Debug)]
pub enum DumpError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Xm(rustytracker_xm::XmParseError),
    InvalidArguments,
    UnsupportedFormat(String),
}

impl fmt::Display for DumpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Xm(error) => write!(formatter, "XM parse error: {error:?}"),
            Self::InvalidArguments => write!(
                formatter,
                "usage: rustytracker dump <module.xm> --format json"
            ),
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported dump format: {format}")
            }
        }
    }
}

impl std::error::Error for DumpError {}

impl From<std::io::Error> for DumpError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DumpError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<rustytracker_xm::XmParseError> for DumpError {
    fn from(error: rustytracker_xm::XmParseError) -> Self {
        Self::Xm(error)
    }
}

#[derive(Debug, Serialize)]
pub struct ModuleDump {
    schema_version: u16,
    format: &'static str,
    header: HeaderDump,
    orders: Vec<u8>,
    patterns: Vec<PatternDump>,
    instruments: Vec<InstrumentDump>,
    samples: Vec<SampleDump>,
}

#[derive(Debug, Serialize)]
struct HeaderDump {
    title: String,
    channel_count: u16,
    frequency_table: &'static str,
    bpm: u16,
    tick_speed: u16,
    main_volume: u16,
    restart_position: u16,
}

#[derive(Debug, Serialize)]
struct PatternDump {
    index: usize,
    rows: u16,
    channels: u16,
    effect_slots: u8,
    non_empty_cells: usize,
    expanded_cell_checksum: u64,
}

#[derive(Debug, Serialize)]
struct InstrumentDump {
    index: usize,
    name: String,
    sample_slots: Vec<Option<usize>>,
    note_sample_map_checksum: u64,
    volume_envelope: EnvelopeDump,
    panning_envelope: EnvelopeDump,
    vibrato: VibratoDump,
    volume_fadeout: u16,
}

#[derive(Debug, Serialize)]
struct EnvelopeDump {
    point_count: u8,
    sustain_point: u8,
    loop_start_point: u8,
    loop_end_point: u8,
    flags: u8,
    points: Vec<EnvelopePointDump>,
}

#[derive(Debug, Serialize)]
struct EnvelopePointDump {
    frame: u16,
    value: u16,
}

#[derive(Debug, Serialize)]
struct VibratoDump {
    waveform: u8,
    sweep: u8,
    depth: u8,
    rate: u8,
}

#[derive(Debug, Serialize)]
struct SampleDump {
    index: usize,
    name: String,
    length: u32,
    loop_start: u32,
    loop_length: u32,
    loop_kind: &'static str,
    volume: u8,
    panning: u8,
    flags: u8,
    volume_fadeout: u16,
    sample_type: u8,
    finetune: i8,
    relative_note: i8,
    data: SampleDataDump,
}

#[derive(Debug, Serialize)]
struct SampleDataDump {
    kind: &'static str,
    frames: usize,
    checksum: u64,
    prefix_i8: Vec<i8>,
    prefix_i16: Vec<i16>,
}

pub fn dump_xm_file_to_json(path: &Path) -> Result<String, DumpError> {
    let bytes = std::fs::read(path)?;
    let module = rustytracker_xm::parse_xm_module(&bytes)?;
    dump_module_to_json(&module)
}

pub fn dump_module_to_json(module: &Module) -> Result<String, DumpError> {
    let mut json = serde_json::to_string_pretty(&module_dump(module))?;
    json.push_str(JSON_TRAILING_NEWLINE);
    Ok(json)
}

pub fn run_cli<I>(args: I) -> Result<String, DumpError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let command = args.next().ok_or(DumpError::InvalidArguments)?;
    let path = args.next().ok_or(DumpError::InvalidArguments)?;
    let format_flag = args.next().ok_or(DumpError::InvalidArguments)?;
    let format = args.next().ok_or(DumpError::InvalidArguments)?;

    if args.next().is_some() || command != "dump" || format_flag != "--format" {
        return Err(DumpError::InvalidArguments);
    }

    if format != "json" {
        return Err(DumpError::UnsupportedFormat(format));
    }

    dump_xm_file_to_json(Path::new(&path))
}

fn module_dump(module: &Module) -> ModuleDump {
    ModuleDump {
        schema_version: DUMP_SCHEMA_VERSION,
        format: DUMP_FORMAT_XM,
        header: HeaderDump {
            title: module.header.title.as_str().to_owned(),
            channel_count: module.header.channel_count,
            frequency_table: frequency_table_name(module.header.frequency_table),
            bpm: module.header.bpm,
            tick_speed: module.header.tick_speed,
            main_volume: module.header.main_volume,
            restart_position: module.header.restart_position,
        },
        orders: module.orders.clone(),
        patterns: module
            .patterns
            .iter()
            .enumerate()
            .map(pattern_dump)
            .collect(),
        instruments: module
            .instruments
            .iter()
            .enumerate()
            .map(|(index, instrument)| InstrumentDump {
                index,
                name: instrument.name.as_str().to_owned(),
                sample_slots: instrument.sample_slots.clone(),
                note_sample_map_checksum: option_usize_checksum(&instrument.note_sample_map),
                volume_envelope: envelope_dump(&instrument.volume_envelope),
                panning_envelope: envelope_dump(&instrument.panning_envelope),
                vibrato: VibratoDump {
                    waveform: instrument.vibrato.waveform,
                    sweep: instrument.vibrato.sweep,
                    depth: instrument.vibrato.depth,
                    rate: instrument.vibrato.rate,
                },
                volume_fadeout: instrument.volume_fadeout,
            })
            .collect(),
        samples: module.samples.iter().enumerate().map(sample_dump).collect(),
    }
}

fn pattern_dump((index, pattern): (usize, &Pattern)) -> PatternDump {
    let mut non_empty_cells = 0;
    let mut checksum = FNV_OFFSET;

    for row in 0..pattern.rows() {
        for channel in 0..pattern.channels() {
            let cell = pattern
                .cell(channel, row)
                .expect("dump walks cells inside pattern bounds");
            let expanded = [
                cell.note.raw(),
                cell.instrument,
                cell.effects[0].effect,
                cell.effects[0].operand,
                cell.effects[1].effect,
                cell.effects[1].operand,
            ];

            if expanded != [0; EXPANDED_PATTERN_CELL_BYTES] {
                non_empty_cells += 1;
            }

            for byte in expanded {
                checksum = fnv_byte(checksum, byte);
            }
        }
    }

    PatternDump {
        index,
        rows: pattern.rows(),
        channels: pattern.channels(),
        effect_slots: pattern.effect_slots(),
        non_empty_cells,
        expanded_cell_checksum: checksum,
    }
}

fn envelope_dump(envelope: &Envelope) -> EnvelopeDump {
    EnvelopeDump {
        point_count: envelope.point_count,
        sustain_point: envelope.sustain_point,
        loop_start_point: envelope.loop_start_point,
        loop_end_point: envelope.loop_end_point,
        flags: envelope.flags,
        points: envelope
            .points
            .iter()
            .map(|point| EnvelopePointDump {
                frame: point.frame,
                value: point.value,
            })
            .collect(),
    }
}

fn sample_dump((index, sample): (usize, &Sample)) -> SampleDump {
    SampleDump {
        index,
        name: sample.name.as_str().to_owned(),
        length: sample.length,
        loop_start: sample.loop_start,
        loop_length: sample.loop_length,
        loop_kind: sample_loop_kind_name(sample.loop_kind),
        volume: sample.volume,
        panning: sample.panning,
        flags: sample.flags,
        volume_fadeout: sample.volume_fadeout,
        sample_type: sample.sample_type,
        finetune: sample.finetune,
        relative_note: sample.relative_note,
        data: sample_data_dump(&sample.data),
    }
}

fn sample_data_dump(data: &SampleData) -> SampleDataDump {
    match data {
        SampleData::Empty => SampleDataDump {
            kind: SAMPLE_DATA_EMPTY,
            frames: data.frame_count(),
            checksum: FNV_OFFSET,
            prefix_i8: Vec::new(),
            prefix_i16: Vec::new(),
        },
        SampleData::Pcm8(values) => SampleDataDump {
            kind: SAMPLE_DATA_PCM8,
            frames: values.len(),
            checksum: checksum_i8(values),
            prefix_i8: values.iter().take(SAMPLE_PREFIX_FRAMES).copied().collect(),
            prefix_i16: Vec::new(),
        },
        SampleData::Pcm16(values) => SampleDataDump {
            kind: SAMPLE_DATA_PCM16,
            frames: values.len(),
            checksum: checksum_i16(values),
            prefix_i8: Vec::new(),
            prefix_i16: values.iter().take(SAMPLE_PREFIX_FRAMES).copied().collect(),
        },
    }
}

fn option_usize_checksum(values: &[Option<usize>]) -> u64 {
    let mut checksum = FNV_OFFSET;

    for value in values {
        match value {
            Some(value) => {
                checksum = fnv_byte(checksum, 1);
                for byte in (*value as u64).to_le_bytes() {
                    checksum = fnv_byte(checksum, byte);
                }
            }
            None => {
                checksum = fnv_byte(checksum, 0);
            }
        }
    }

    checksum
}

fn checksum_i8(values: &[i8]) -> u64 {
    values.iter().fold(FNV_OFFSET, |checksum, value| {
        fnv_byte(checksum, *value as u8)
    })
}

fn checksum_i16(values: &[i16]) -> u64 {
    let mut checksum = FNV_OFFSET;

    for value in values {
        for byte in value.to_le_bytes() {
            checksum = fnv_byte(checksum, byte);
        }
    }

    checksum
}

fn fnv_byte(checksum: u64, byte: u8) -> u64 {
    (checksum ^ byte as u64).wrapping_mul(FNV_PRIME)
}

fn frequency_table_name(frequency_table: FrequencyTable) -> &'static str {
    match frequency_table {
        FrequencyTable::Amiga => FREQUENCY_TABLE_AMIGA,
        FrequencyTable::Linear => FREQUENCY_TABLE_LINEAR,
    }
}

fn sample_loop_kind_name(loop_kind: SampleLoopKind) -> &'static str {
    match loop_kind {
        SampleLoopKind::None => SAMPLE_LOOP_NONE,
        SampleLoopKind::Forward => SAMPLE_LOOP_FORWARD,
        SampleLoopKind::PingPong => SAMPLE_LOOP_PING_PONG,
    }
}
