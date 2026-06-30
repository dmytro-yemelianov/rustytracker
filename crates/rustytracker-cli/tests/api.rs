use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustytracker_cli::run_cli;
use rustytracker_core::Note;
use rustytracker_test_support::{
    milkytracker_fixture_path as fixture_path,
    milkytracker_fixtures_available as fixtures_available,
};
use rustytracker_xm::parse_xm_module;
use serde_json::json;
use std::path::PathBuf;

const API_COMMAND: &str = "api";
const API_REQUEST_JSON_FLAG: &str = "--request-json";
const API_REQUEST_FILE_FLAG: &str = "--request-file";

#[test]
fn api_ping_returns_ok() {
    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        json!({"method":"ping"}).to_string(),
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert_eq!(value["method"].as_str().unwrap(), "ping");
}

#[test]
fn api_module_apply_patch_reflects_edit() {
    if !fixtures_available() {
        return;
    }

    let request = json!({
        "id": "patch-1",
        "method": "module.apply_patch",
        "params": {
            "module_path": fixture_path("milky.xm"),
            "patch": [
                {
                    "op": "set_note",
                    "pattern": 0,
                    "channel": 0,
                    "row": 0,
                    "note": 49,
                }
            ]
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    let module_bytes = value
        .get("result")
        .and_then(|result| result.get("module_bytes_b64"))
        .and_then(|value| value.as_str())
        .unwrap();
    let module = parse_xm_module(&STANDARD.decode(module_bytes).unwrap()).unwrap();

    assert_eq!(module.patterns[0].cell(0, 0).unwrap().note, Note::Key(49));
}

#[test]
fn api_module_new_creates_structure() {
    let request = json!({
        "id": "new-1",
        "method": "module.new",
        "params": {
            "module_channel_count": 6,
            "module_title": "LLM Builder",
            "patch": [
                {
                    "op": "insert_track",
                    "index": 0
                },
                {
                    "op": "create_pattern",
                    "rows": 32
                },
                {
                    "op": "create_sample",
                    "sample": {
                        "name": "Sine",
                        "loop_start": 0,
                        "loop_kind": "none",
                        "data": {
                            "kind": "pcm8",
                            "data_b64": "AA=="
                        }
                    }
                },
                {
                    "op": "create_instrument",
                    "name": "Lead",
                    "default_sample_index": 0,
                    "index": 0
                },
                {
                    "op": "rename_sample",
                    "index": 0,
                    "name": "Kick"
                },
            ]
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert_eq!(value["id"].as_str().unwrap(), "new-1");
    assert_eq!(value["result"]["format"].as_str().unwrap(), "xm");

    let module_bytes = value["result"]["module_bytes_b64"].as_str().unwrap();
    let module = parse_xm_module(&STANDARD.decode(module_bytes).unwrap()).unwrap();

    assert_eq!(module.header.channel_count, 7);
    assert_eq!(module.patterns.len(), 2);
    assert_eq!(module.header.title.as_str(), "LLM Builder");
    assert_eq!(module.patterns[1].rows(), 32);
    assert_eq!(module.instruments[0].name.as_str(), "Lead");
    assert_eq!(module.samples[0].name.as_str(), "Kick");
}

#[test]
fn api_module_new_rejects_bad_pcm16_payload() {
    let request = json!({
        "method": "module.new",
        "params": {
            "patch": [
                {
                    "op": "create_sample",
                        "sample": {
                            "name": "Odd bytes",
                            "data": {
                                "kind": "pcm16",
                                "data_b64": "AAAA"
                            }
                        }
                    }
            ]
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(!value["ok"].as_bool().unwrap());
    assert_eq!(value["error"]["code"].as_str().unwrap(), "invalid_request");
    assert!(value["error"]["message"]
        .as_str()
        .unwrap()
        .contains("pcm16 sample data must contain an even number of bytes"));
}

#[test]
fn api_module_new_supports_empty_sample_kind() {
    let request = json!({
        "method": "module.new",
        "params": {
            "patch": [
                {
                    "op": "create_sample",
                    "sample": {
                        "name": "Empty",
                        "data": {
                            "kind": "empty"
                        }
                    }
                }
            ]
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());

    let samples = value["result"]["module"]["samples"].as_array().unwrap();
    let has_named_sample = samples.iter().any(|sample| sample["name"] == "Empty");
    assert!(has_named_sample);

    let has_empty_sample = samples
        .iter()
        .any(|sample| sample["name"] == "Empty" && sample["data"]["frames"].as_u64().unwrap() == 0);
    assert!(has_empty_sample);
}

#[test]
fn api_module_render_wav_returns_audio() {
    if !fixtures_available() {
        return;
    }

    let request = json!({
        "method": "module.render_wav",
        "params": {
            "module_path": fixture_path("milky.xm"),
            "sample_rate": 8000,
            "mixer": "hifi"
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert_eq!(value["result"]["sample_rate"].as_u64().unwrap(), 8000);
    assert!(value["result"]["wav_bytes_b64"].as_str().unwrap().len() > 100);
}

#[test]
fn api_returns_parse_error_for_unknown_method() {
    let request = json!({"method":"module.nope"}).to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(!value["ok"].as_bool().unwrap());
    assert_eq!(value["method"].as_str().unwrap(), "parse");
}

#[test]
fn api_methods_lists_capabilities() {
    let request = json!({"method":"api.methods"}).to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    let methods = value["result"]["methods"].as_array().unwrap();
    assert!(methods.iter().any(|method| method == "module.render_wav"));
    assert!(methods.iter().any(|method| method == "module.new"));
    assert_eq!(value["result"]["api_version"].as_u64().unwrap(), 1);
}

#[test]
fn api_methods_exposes_patch_and_payload_capabilities() {
    let request = json!({"method":"api.methods"}).to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());

    let result = &value["result"];
    assert!(result["patch_ops"]
        .as_array()
        .unwrap()
        .iter()
        .any(|op| op == "create_instrument"));
    assert!(result["sample_data_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "empty"));
}

#[test]
fn api_module_render_wav_respects_duration_ms() {
    if !fixtures_available() {
        return;
    }

    let request = json!({
        "method": "module.render_wav",
        "params": {
            "module_path": fixture_path("milky.xm"),
            "sample_rate": 8000,
            "duration_ms": 250
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert_eq!(value["result"]["sample_rate"].as_u64().unwrap(), 8000);

    let wav = STANDARD
        .decode(value["result"]["wav_bytes_b64"].as_str().unwrap())
        .unwrap();
    let frames = parse_wav_data_frames(&wav);
    assert_eq!(value["result"]["frame_limit"], json!(2000));
    assert!(frames <= 2000);
    assert!(frames > 0);
}

#[test]
fn api_module_render_wav_outputs_file_without_blob_when_requested() {
    if !fixtures_available() {
        return;
    }

    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output_path = std::env::temp_dir()
        .join(format!("rustytracker_api_render_test_{unique_suffix}.wav"))
        .to_string_lossy()
        .into_owned();

    let request = json!({
        "method": "module.render_wav",
        "params": {
            "module_path": fixture_path("milky.xm"),
            "sample_rate": 8000,
            "duration_ms": 120,
            "include_wav": false,
            "output_path": output_path
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert!(value["result"].get("output_path").is_some());
    assert_eq!(
        value["result"]["output_path"].as_str().unwrap(),
        output_path
    );
    assert!(value["result"]["wav_bytes_b64"].is_null());
    assert!(PathBuf::from(&output_path).exists());
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 44);

    assert!(std::fs::remove_file(output_path).is_ok());
}

#[test]
fn api_module_launch_ui_requires_binary_or_fails_fast() {
    if !fixtures_available() {
        return;
    }

    let request = json!({
        "method": "module.launch_ui",
        "params": {
            "module_path": fixture_path("milky.xm"),
            "ui_binary": "__does_not_exist__"
        }
    })
    .to_string();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_JSON_FLAG.to_string(),
        request,
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["ok"].as_bool().unwrap(), false);
    assert_eq!(value["error"]["code"].as_str().unwrap(), "ui_launch_failed");
}

#[test]
fn api_request_file_path_is_supported() {
    if !fixtures_available() {
        return;
    }

    let request = json!({
        "id": "from-file",
        "method": "module.load",
        "params": {
            "module_path": fixture_path("milky.xm")
        }
    })
    .to_string();

    let request_path = std::env::temp_dir()
        .join(format!(
            "rustytracker_api_request_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .to_string_lossy()
        .into_owned();
    std::fs::write(&request_path, request).unwrap();

    let output = run_cli(vec![
        API_COMMAND.to_string(),
        API_REQUEST_FILE_FLAG.to_string(),
        request_path.clone(),
    ])
    .unwrap();

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(value["ok"].as_bool().unwrap());
    assert_eq!(value["id"].as_str().unwrap(), "from-file");
    assert_eq!(value["result"]["format"].as_str().unwrap(), "xm");

    assert!(std::fs::remove_file(request_path).is_ok());
}

fn parse_wav_data_frames(wav: &[u8]) -> u64 {
    assert!(wav.len() >= 44);
    let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]) as u64;
    data_size / 4
}
