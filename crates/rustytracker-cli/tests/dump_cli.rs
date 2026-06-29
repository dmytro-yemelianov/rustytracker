use std::path::{Path, PathBuf};
use std::process::Command;

use rustytracker_cli::{
    dump_module_to_json, dump_xm_file_to_json, play_state_xm_file_to_json, run_cli,
};
use rustytracker_core::{EffectCommand, Module, Note, Pattern, PatternCell};
use rustytracker_test_support::{
    milkytracker_fixture_path as fixture_path,
    milkytracker_fixtures_available as fixtures_available,
};

const FIXTURES: &[(&str, &str)] = &[
    ("milky.xm", "golden/milky.json"),
    ("slumberjack.xm", "golden/slumberjack.json"),
    ("sv_ttt.xm", "golden/sv_ttt.json"),
    ("theday.xm", "golden/theday.json"),
    (
        "universalnetwork2_real.xm",
        "golden/universalnetwork2_real.json",
    ),
];
const DUMP_COMMAND: &str = "dump";
const PLAY_STATE_COMMAND: &str = "play-state";
const EXPORT_WAV_COMMAND: &str = "export-wav";
const FORMAT_FLAG: &str = "--format";
const JSON_FORMAT: &str = "json";
const ROWS_FLAG: &str = "--rows";
const SAMPLE_RATE_FLAG: &str = "--sample-rate";
const MIXER_FLAG: &str = "--mixer";
const PLAY_STATE_TEST_ROWS: usize = 3;
const PLAY_STATE_ZERO_ROWS: usize = 0;
const PLAY_STATE_TEST_ROWS_TEXT: &str = "3";
const PLAY_STATE_ZERO_ROWS_TEXT: &str = "0";
const PLAY_STATE_NON_NUMERIC_ROWS_TEXT: &str = "many";
const PLAY_STATE_ROW_COUNT_ERROR: &str = "invalid play-state row count: 0";
const PLAY_STATE_NON_NUMERIC_ROW_COUNT_ERROR: &str = "invalid play-state row count: many";
const PLAY_STATE_MISSING_ROWS_ERROR: &str = "usage: rustytracker";
const EXPORT_WAV_ZERO_SAMPLE_RATE_TEXT: &str = "0";
const EXPORT_WAV_INVALID_SAMPLE_RATE_ERROR: &str = "invalid export sample rate: 0";
const EXPORT_WAV_INVALID_MIXER_MODE_TEXT: &str = "muddy";
const EXPORT_WAV_INVALID_MIXER_MODE_ERROR: &str = "invalid export mixer mode: muddy";
const EXPORT_WAV_TEST_SAMPLE_RATE: u32 = 44_100;
const EXPORT_WAV_EXPECTED_C3_PERIOD_MIN: f64 = 336.0;
const EXPORT_WAV_EXPECTED_C3_PERIOD_MAX: f64 = 339.0;
const EXPORT_WAV_EXPECTED_PROTRACKER_C3_PERIOD_MIN: f64 = 339.0;
const EXPORT_WAV_EXPECTED_PROTRACKER_C3_PERIOD_MAX: f64 = 342.0;
const PLAY_STATE_EXPECTED_FORMAT: &str = "play_state";
const PLAY_STATE_EXPECTED_SCHEMA_VERSION: u64 = 1;
const PLAY_STATE_EXPECTED_CHANNELS: usize = 10;
const PLAY_STATE_FIRST_ROW_INDEX: usize = 0;
const PLAY_STATE_FIRST_CHANNEL_INDEX: usize = 0;
const PLAY_STATE_FIRST_ORDER: u64 = 0;
const PLAY_STATE_FIRST_ROW: u64 = 0;
const PLAY_STATE_ROW_START_TICK: u64 = 0;
const PLAY_STATE_FIRST_INSTRUMENT_INDEX: u64 = 0;
const PLAY_STATE_FIRST_SAMPLE_INDEX: u64 = 0;
const PLAY_STATE_SAMPLE_START_FRAME: u64 = 0;
const PLAY_STATE_EXPECTED_PARTIAL_COMPLETED: bool = false;

#[test]
fn golden_dumps_match_bundled_xm_fixtures() {
    if !fixtures_available() {
        return;
    }

    for (xm_file, golden_file) in FIXTURES {
        let actual = dump_xm_file_to_json(&fixture_path(xm_file)).unwrap();
        let expected = read_cli_fixture(golden_file);

        assert_eq!(actual, expected, "{xm_file}");
    }
}

#[test]
fn binary_writes_json_dump_to_stdout() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let expected = dump_xm_file_to_json(&fixture).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(DUMP_COMMAND)
        .arg(&fixture)
        .arg(FORMAT_FLAG)
        .arg(JSON_FORMAT)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    assert!(output.stderr.is_empty());
}

#[test]
fn play_state_dump_reports_first_rows_from_xm_fixture() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let json = play_state_xm_file_to_json(&fixture, PLAY_STATE_TEST_ROWS).unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&json).unwrap();
    let module_dump =
        serde_json::from_str::<serde_json::Value>(&dump_xm_file_to_json(&fixture).unwrap())
            .unwrap();

    assert_eq!(
        value["schema_version"].as_u64().unwrap(),
        PLAY_STATE_EXPECTED_SCHEMA_VERSION
    );
    assert_eq!(
        value["format"].as_str().unwrap(),
        PLAY_STATE_EXPECTED_FORMAT
    );
    assert_eq!(
        value["requested_rows"].as_u64().unwrap(),
        PLAY_STATE_TEST_ROWS as u64
    );
    assert_eq!(
        value["rows"].as_array().unwrap().len(),
        PLAY_STATE_TEST_ROWS
    );
    assert_eq!(
        value["completed"].as_bool().unwrap(),
        PLAY_STATE_EXPECTED_PARTIAL_COMPLETED
    );
    assert_eq!(
        value["timing"]["bpm"].as_u64().unwrap(),
        module_dump["header"]["bpm"].as_u64().unwrap()
    );
    assert_eq!(
        value["timing"]["ticks_per_row"].as_u64().unwrap(),
        module_dump["header"]["tick_speed"].as_u64().unwrap()
    );
    assert_eq!(
        value["timing"]["row_duration_nanos"].as_u64().unwrap(),
        value["timing"]["tick_duration_nanos"].as_u64().unwrap()
            * value["timing"]["ticks_per_row"].as_u64().unwrap()
    );
    for row in value["rows"].as_array().unwrap() {
        assert_eq!(row["tick"].as_u64().unwrap(), PLAY_STATE_ROW_START_TICK);
    }
    assert_eq!(
        value["rows"][PLAY_STATE_FIRST_ROW_INDEX]["order_index"]
            .as_u64()
            .unwrap(),
        PLAY_STATE_FIRST_ORDER
    );
    assert_eq!(
        value["rows"][PLAY_STATE_FIRST_ROW_INDEX]["row"]
            .as_u64()
            .unwrap(),
        PLAY_STATE_FIRST_ROW
    );
    assert_eq!(
        value["rows"][PLAY_STATE_FIRST_ROW_INDEX]["channels"]
            .as_array()
            .unwrap()
            .len(),
        PLAY_STATE_EXPECTED_CHANNELS
    );
    let first_channel =
        &value["rows"][PLAY_STATE_FIRST_ROW_INDEX]["channels"][PLAY_STATE_FIRST_CHANNEL_INDEX];
    assert!(first_channel["state"]["active"].as_bool().unwrap());
    assert_eq!(
        first_channel["state"]["note"].as_u64().unwrap(),
        first_channel["note"].as_u64().unwrap()
    );
    assert_eq!(
        first_channel["state"]["instrument"].as_u64().unwrap(),
        first_channel["instrument"].as_u64().unwrap()
    );
    assert_eq!(
        first_channel["state"]["instrument_index"].as_u64().unwrap(),
        PLAY_STATE_FIRST_INSTRUMENT_INDEX
    );
    assert_eq!(
        first_channel["state"]["sample_index"].as_u64().unwrap(),
        PLAY_STATE_FIRST_SAMPLE_INDEX
    );
    assert_eq!(
        first_channel["state"]["sample_frame"].as_u64().unwrap(),
        PLAY_STATE_SAMPLE_START_FRAME
    );
    assert_eq!(
        first_channel["state"]["volume"].as_u64().unwrap(),
        module_dump["samples"][PLAY_STATE_FIRST_SAMPLE_INDEX as usize]["volume"]
            .as_u64()
            .unwrap()
    );
    assert_eq!(
        first_channel["state"]["panning"].as_u64().unwrap(),
        module_dump["samples"][PLAY_STATE_FIRST_SAMPLE_INDEX as usize]["panning"]
            .as_u64()
            .unwrap()
    );
}

#[test]
fn play_state_dump_rejects_zero_rows() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");

    assert!(play_state_xm_file_to_json(&fixture, PLAY_STATE_ZERO_ROWS)
        .unwrap_err()
        .to_string()
        .contains(PLAY_STATE_ROW_COUNT_ERROR));
}

#[test]
fn binary_writes_play_state_json_to_stdout() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let expected = play_state_xm_file_to_json(&fixture, PLAY_STATE_TEST_ROWS).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(PLAY_STATE_COMMAND)
        .arg(&fixture)
        .arg(ROWS_FLAG)
        .arg(PLAY_STATE_TEST_ROWS_TEXT)
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    assert!(output.stderr.is_empty());
}

#[test]
fn binary_rejects_zero_play_state_rows() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(PLAY_STATE_COMMAND)
        .arg(&fixture)
        .arg(ROWS_FLAG)
        .arg(PLAY_STATE_ZERO_ROWS_TEXT)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(PLAY_STATE_ROW_COUNT_ERROR));
}

#[test]
fn binary_rejects_non_numeric_play_state_rows() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(PLAY_STATE_COMMAND)
        .arg(&fixture)
        .arg(ROWS_FLAG)
        .arg(PLAY_STATE_NON_NUMERIC_ROWS_TEXT)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(PLAY_STATE_NON_NUMERIC_ROW_COUNT_ERROR));
}

#[test]
fn binary_rejects_missing_play_state_rows() {
    if !fixtures_available() {
        return;
    }

    let fixture = fixture_path("milky.xm");
    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(PLAY_STATE_COMMAND)
        .arg(&fixture)
        .arg(ROWS_FLAG)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains(PLAY_STATE_MISSING_ROWS_ERROR));
}

#[test]
fn export_wav_rejects_zero_sample_rate_before_loading_input() {
    let error = run_cli(
        [
            EXPORT_WAV_COMMAND,
            "missing.xm",
            "out.wav",
            SAMPLE_RATE_FLAG,
            EXPORT_WAV_ZERO_SAMPLE_RATE_TEXT,
        ]
        .into_iter()
        .map(String::from),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(EXPORT_WAV_INVALID_SAMPLE_RATE_ERROR));
}

#[test]
fn export_wav_rejects_invalid_mixer_mode_before_loading_input() {
    let error = run_cli(
        [
            EXPORT_WAV_COMMAND,
            "missing.xm",
            "out.wav",
            MIXER_FLAG,
            EXPORT_WAV_INVALID_MIXER_MODE_TEXT,
        ]
        .into_iter()
        .map(String::from),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(EXPORT_WAV_INVALID_MIXER_MODE_ERROR));
}

#[test]
fn export_wav_uses_milkytracker_mod_pitch_clock() {
    let temp_mod_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp_loop_pitch.mod");
    let temp_wav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp_loop_pitch.wav");
    write_looped_sine_mod_file(&temp_mod_path);

    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(EXPORT_WAV_COMMAND)
        .arg(&temp_mod_path)
        .arg(&temp_wav_path)
        .arg(SAMPLE_RATE_FLAG)
        .arg(EXPORT_WAV_TEST_SAMPLE_RATE.to_string())
        .output()
        .unwrap();

    assert!(output.status.success());
    let left_channel = read_wav_left_channel(&temp_wav_path);
    let mean_period = mean_positive_zero_crossing_period(&left_channel);

    assert!(
        (EXPORT_WAV_EXPECTED_C3_PERIOD_MIN..=EXPORT_WAV_EXPECTED_C3_PERIOD_MAX)
            .contains(&mean_period),
        "mean period {mean_period}"
    );

    std::fs::remove_file(temp_mod_path).unwrap();
    std::fs::remove_file(temp_wav_path).unwrap();
}

#[test]
fn export_wav_can_select_protracker_mixer_mode() {
    let temp_mod_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp_loop_pitch_protracker.mod");
    let temp_wav_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp_loop_pitch_protracker.wav");
    write_looped_sine_mod_file(&temp_mod_path);

    let output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(EXPORT_WAV_COMMAND)
        .arg(&temp_mod_path)
        .arg(&temp_wav_path)
        .arg(SAMPLE_RATE_FLAG)
        .arg(EXPORT_WAV_TEST_SAMPLE_RATE.to_string())
        .arg(MIXER_FLAG)
        .arg("protracker")
        .output()
        .unwrap();

    assert!(output.status.success());
    let left_channel = read_wav_left_channel(&temp_wav_path);
    let mean_period = mean_positive_zero_crossing_period(&left_channel);

    assert!(
        (EXPORT_WAV_EXPECTED_PROTRACKER_C3_PERIOD_MIN
            ..=EXPORT_WAV_EXPECTED_PROTRACKER_C3_PERIOD_MAX)
            .contains(&mean_period),
        "mean period {mean_period}"
    );

    std::fs::remove_file(temp_mod_path).unwrap();
    std::fs::remove_file(temp_wav_path).unwrap();
}

#[test]
fn schema_file_is_valid_json() {
    let schema = read_cli_fixture("../schema/module-dump.schema.json");
    serde_json::from_str::<serde_json::Value>(&schema).unwrap();
}

#[test]
fn dump_counts_effects_beyond_second_slot() {
    let mut module = Module::empty_with_channels(1).unwrap();
    let mut pattern = Pattern::new(1, 1, 3);
    pattern
        .set_cell(
            0,
            0,
            PatternCell {
                note: Note::Empty,
                instrument: 0,
                effects: vec![
                    EffectCommand::default(),
                    EffectCommand::default(),
                    EffectCommand {
                        effect: 0x0c,
                        operand: 64,
                    },
                ],
            },
        )
        .unwrap();
    module.patterns = vec![pattern];

    let dump = dump_module_to_json(&module, "xm").unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&dump).unwrap();

    assert_eq!(value["patterns"][0]["effect_slots"].as_u64().unwrap(), 3);
    assert_eq!(value["patterns"][0]["non_empty_cells"].as_u64().unwrap(), 1);
}

#[test]
fn test_cli_dumps_and_plays_mod_format() {
    let temp_mod_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("temp_test_mod.mod");
    write_temp_mod_file(&temp_mod_path);

    // Test 1: dump_xm_file_to_json auto-detects and dumps MOD format
    let actual_json = dump_xm_file_to_json(&temp_mod_path).unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&actual_json).unwrap();
    assert_eq!(value["format"].as_str().unwrap(), "mod");
    assert_eq!(value["header"]["title"].as_str().unwrap(), "Test CLI Mod");

    // Test 2: play_state_xm_file_to_json auto-detects and plays MOD format
    let play_json = play_state_xm_file_to_json(&temp_mod_path, 2).unwrap();
    let play_val = serde_json::from_str::<serde_json::Value>(&play_json).unwrap();
    assert_eq!(play_val["format"].as_str().unwrap(), "play_state");
    assert_eq!(play_val["requested_rows"].as_u64().unwrap(), 2);

    // Test 3: Binary execution for dump command
    let dump_output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(DUMP_COMMAND)
        .arg(&temp_mod_path)
        .arg(FORMAT_FLAG)
        .arg(JSON_FORMAT)
        .output()
        .unwrap();
    assert!(dump_output.status.success());
    let dump_stdout = String::from_utf8(dump_output.stdout).unwrap();
    let dump_val = serde_json::from_str::<serde_json::Value>(&dump_stdout).unwrap();
    assert_eq!(dump_val["format"].as_str().unwrap(), "mod");

    // Test 4: Binary execution for play-state command
    let play_output = Command::new(env!("CARGO_BIN_EXE_rustytracker"))
        .arg(PLAY_STATE_COMMAND)
        .arg(&temp_mod_path)
        .arg(ROWS_FLAG)
        .arg("2")
        .output()
        .unwrap();
    assert!(play_output.status.success());
    let play_stdout = String::from_utf8(play_output.stdout).unwrap();
    let play_stdout_val = serde_json::from_str::<serde_json::Value>(&play_stdout).unwrap();
    assert_eq!(play_stdout_val["format"].as_str().unwrap(), "play_state");

    std::fs::remove_file(temp_mod_path).unwrap();
}

fn write_temp_mod_file(path: &Path) {
    let mut bytes = vec![0u8; 1624];
    bytes[0..12].copy_from_slice(b"Test CLI Mod");
    bytes[20 + 15 * 30] = 1; // Song length
    bytes[20 + 15 * 30 + 2] = 0; // First pattern in order list is 0
    std::fs::write(path, bytes).unwrap();
}

fn write_looped_sine_mod_file(path: &Path) {
    const SAMPLE_LEN: usize = 64;
    const MOD_HEADER_LEN: usize = 20 + 15 * 30 + 2 + 128;
    const MOD_PATTERN_LEN: usize = 64 * 4 * 4;
    const C3_PERIOD: u16 = 428;

    let mut bytes = vec![0u8; MOD_HEADER_LEN + MOD_PATTERN_LEN + SAMPLE_LEN];
    bytes[0..11].copy_from_slice(b"CLI C3 LOOP");

    let sample_header = 20;
    bytes[sample_header..sample_header + 6].copy_from_slice(b"sine64");
    bytes[sample_header + 22..sample_header + 24]
        .copy_from_slice(&((SAMPLE_LEN / 2) as u16).to_be_bytes());
    bytes[sample_header + 25] = 64;
    bytes[sample_header + 28..sample_header + 30]
        .copy_from_slice(&((SAMPLE_LEN / 2) as u16).to_be_bytes());

    bytes[20 + 15 * 30] = 1;
    bytes[20 + 15 * 30 + 2] = 0;

    let pattern_offset = MOD_HEADER_LEN;
    bytes[pattern_offset] = ((C3_PERIOD >> 8) & 0x0f) as u8;
    bytes[pattern_offset + 1] = (C3_PERIOD & 0xff) as u8;
    bytes[pattern_offset + 2] = 0x10;

    let sample_offset = MOD_HEADER_LEN + MOD_PATTERN_LEN;
    for index in 0..SAMPLE_LEN {
        let phase = index as f64 * std::f64::consts::TAU / SAMPLE_LEN as f64;
        bytes[sample_offset + index] = (phase.sin() * 100.0).round() as i8 as u8;
    }

    std::fs::write(path, bytes).unwrap();
}

fn read_wav_left_channel(path: &Path) -> Vec<i16> {
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[36..40], b"data");
    let data_size = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
    let data = &bytes[44..44 + data_size];
    data.chunks_exact(4)
        .map(|frame| i16::from_le_bytes([frame[0], frame[1]]))
        .collect()
}

fn mean_positive_zero_crossing_period(samples: &[i16]) -> f64 {
    let first_nonzero = samples.iter().position(|sample| *sample != 0).unwrap();
    let start = first_nonzero + 2_000;
    let end = (start + EXPORT_WAV_TEST_SAMPLE_RATE as usize * 2).min(samples.len());
    let crossings: Vec<usize> = samples[start..end]
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            if pair[0] < 0 && pair[1] >= 0 {
                Some(start + index)
            } else {
                None
            }
        })
        .collect();
    assert!(crossings.len() > 2);

    let total: usize = crossings.windows(2).map(|pair| pair[1] - pair[0]).sum();
    total as f64 / (crossings.len() - 1) as f64
}

fn read_cli_fixture(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(path),
    )
    .unwrap()
}
