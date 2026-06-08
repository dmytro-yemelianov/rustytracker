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
const PLAY_STATE_EXPECTED_FORMAT: &str = "play_state";
const PLAY_STATE_EXPECTED_SCHEMA_VERSION: u64 = 1;
const PLAY_STATE_EXPECTED_CHANNELS: usize = 10;
const PLAY_STATE_FIRST_ORDER: u64 = 0;
const PLAY_STATE_FIRST_ROW: u64 = 0;

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
        value["rows"][0]["order_index"].as_u64().unwrap(),
        PLAY_STATE_FIRST_ORDER
    );
    assert_eq!(
        value["rows"][0]["row"].as_u64().unwrap(),
        PLAY_STATE_FIRST_ROW
    );
    assert_eq!(
        value["rows"][0]["channels"].as_array().unwrap().len(),
        PLAY_STATE_EXPECTED_CHANNELS
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
fn schema_file_is_valid_json() {
    let schema = read_cli_fixture("../schema/module-dump.schema.json");
    serde_json::from_str::<serde_json::Value>(&schema).unwrap();
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
