use std::path::{Path, PathBuf};
use std::process::Command;

use rustytracker_cli::{dump_xm_file_to_json, play_state_xm_file_to_json};

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
const FORMAT_FLAG: &str = "--format";
const JSON_FORMAT: &str = "json";
const ROWS_FLAG: &str = "--rows";
const PLAY_STATE_TEST_ROWS: usize = 3;
const PLAY_STATE_ZERO_ROWS: usize = 0;
const PLAY_STATE_TEST_ROWS_TEXT: &str = "3";
const PLAY_STATE_ZERO_ROWS_TEXT: &str = "0";
const PLAY_STATE_NON_NUMERIC_ROWS_TEXT: &str = "many";
const PLAY_STATE_ROW_COUNT_ERROR: &str = "invalid play-state row count: 0";
const PLAY_STATE_NON_NUMERIC_ROW_COUNT_ERROR: &str = "invalid play-state row count: many";
const PLAY_STATE_MISSING_ROWS_ERROR: &str = "usage: rustytracker";
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
    for (xm_file, golden_file) in FIXTURES {
        let actual = dump_xm_file_to_json(&fixture_path(xm_file)).unwrap();
        let expected = read_cli_fixture(golden_file);

        assert_eq!(actual, expected, "{xm_file}");
    }
}

#[test]
fn binary_writes_json_dump_to_stdout() {
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
    let fixture = fixture_path("milky.xm");

    assert!(play_state_xm_file_to_json(&fixture, PLAY_STATE_ZERO_ROWS)
        .unwrap_err()
        .to_string()
        .contains(PLAY_STATE_ROW_COUNT_ERROR));
}

#[test]
fn binary_writes_play_state_json_to_stdout() {
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
fn schema_file_is_valid_json() {
    let schema = read_cli_fixture("../schema/module-dump.schema.json");
    serde_json::from_str::<serde_json::Value>(&schema).unwrap();
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

fn fixture_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../MilkyTracker/resources/music")
        .join(file_name)
}

fn read_cli_fixture(path: impl AsRef<Path>) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(path),
    )
    .unwrap()
}
