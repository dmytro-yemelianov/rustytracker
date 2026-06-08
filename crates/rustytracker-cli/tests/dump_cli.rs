use std::path::{Path, PathBuf};
use std::process::Command;

use rustytracker_cli::dump_xm_file_to_json;

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
const FORMAT_FLAG: &str = "--format";
const JSON_FORMAT: &str = "json";

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
