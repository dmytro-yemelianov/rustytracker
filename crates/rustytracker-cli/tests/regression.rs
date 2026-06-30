use rustytracker_cli::{load_module_from_file, play_state_module_to_json};
use rustytracker_play::{PlaybackMixerMode, PlaybackState};
use rustytracker_test_support::{
    milkytracker_fixture_path as fixture_path,
    milkytracker_fixtures_available as fixtures_available,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FIXTURES: &[&str] = &[
    "milky.xm",
    "slumberjack.xm",
    "sv_ttt.xm",
    "theday.xm",
    "universalnetwork2_real.xm",
];

const REGRESSION_ROWS: usize = 16;
const PCM_RENDER_SECONDS: f64 = 3.0;
const SAMPLE_RATE: u32 = 44100;

#[derive(Serialize, Deserialize, Default)]
struct PcmHashesRegistry(BTreeMap<String, BTreeMap<String, String>>);

fn fnv1a_hash(data: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    format!("{:016x}", hash)
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

#[test]
fn run_regression_fixtures() {
    let update_golden = std::env::var("UPDATE_GOLDEN").is_ok();
    let golden_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden");

    let pcm_hashes_path = golden_dir.join("pcm_hashes.json");
    let mut pcm_registry = if update_golden {
        PcmHashesRegistry::default()
    } else {
        let content = std::fs::read_to_string(&pcm_hashes_path).expect(
            "Failed to read golden pcm_hashes.json. Run with UPDATE_GOLDEN=1 to create it.",
        );
        serde_json::from_str(&content).expect("Failed to parse golden pcm_hashes.json")
    };

    // 1. Run MilkyTracker bundled fixtures (if available)
    if fixtures_available() {
        for xm_file in FIXTURES {
            let path = fixture_path(xm_file);
            let (module, format) = load_module_from_file(&path).unwrap();

            // Verify/Update state transitions (play-state)
            let actual_play_state =
                play_state_module_to_json(&module, format, REGRESSION_ROWS).unwrap();
            let play_state_file = golden_dir.join(format!("{xm_file}.play_state.json"));

            if update_golden {
                std::fs::write(&play_state_file, &actual_play_state).unwrap();
            } else {
                let expected_play_state = std::fs::read_to_string(&play_state_file).expect(
                    "Failed to read golden play-state file. Run with UPDATE_GOLDEN=1 to create it.",
                );
                assert_eq!(
                    actual_play_state, expected_play_state,
                    "Play-state transition mismatch for {xm_file}"
                );
            }

            // Verify/Update PCM output hashes
            let mut file_hashes = BTreeMap::new();
            for mixer_mode in PlaybackMixerMode::ALL {
                let mode_name = mixer_mode.cli_name().to_string();

                let mut playback =
                    PlaybackState::start_with_mixer_mode(&module, mixer_mode).unwrap();
                let mut pcm_bytes = Vec::new();
                let total_frames = (PCM_RENDER_SECONDS * SAMPLE_RATE as f64) as usize;

                for _ in 0..total_frames {
                    let (left_i32, right_i32) = playback
                        .render_raw_stereo_frame(&module, SAMPLE_RATE)
                        .unwrap();
                    let left_i16 = left_i32.clamp(-32768, 32767) as i16;
                    let right_i16 = right_i32.clamp(-32768, 32767) as i16;
                    pcm_bytes.extend_from_slice(&left_i16.to_le_bytes());
                    pcm_bytes.extend_from_slice(&right_i16.to_le_bytes());
                }

                let hash = fnv1a_hash(&pcm_bytes);
                if update_golden {
                    file_hashes.insert(mode_name, hash);
                } else {
                    let expected_hash = pcm_registry
                        .0
                        .get(*xm_file)
                        .and_then(|modes| modes.get(&mode_name))
                        .expect(&format!(
                            "Missing golden PCM hash for {xm_file} in mode {mode_name}"
                        ));
                    assert_eq!(
                        &hash, expected_hash,
                        "PCM output mismatch for {xm_file} in mode {mode_name}"
                    );
                }
            }

            if update_golden {
                pcm_registry.0.insert(xm_file.to_string(), file_hashes);
            }
        }
    }

    // 2. Run synthetic MOD regression fixture (always runs)
    {
        let temp_mod_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("temp_synthetic_regression.mod");

        write_looped_sine_mod_file(&temp_mod_path);

        let (module, format) = load_module_from_file(&temp_mod_path).unwrap();

        // Verify/Update state transitions (play-state)
        let actual_play_state = play_state_module_to_json(&module, format, 4).unwrap();
        let play_state_file = golden_dir.join("synthetic_mod.play_state.json");

        if update_golden {
            std::fs::write(&play_state_file, &actual_play_state).unwrap();
        } else {
            let expected_play_state = std::fs::read_to_string(&play_state_file).expect(
                "Failed to read golden play-state file. Run with UPDATE_GOLDEN=1 to create it.",
            );
            assert_eq!(
                actual_play_state, expected_play_state,
                "Play-state transition mismatch for synthetic_mod"
            );
        }

        // Verify/Update PCM output hashes
        let mut file_hashes = BTreeMap::new();
        for mixer_mode in PlaybackMixerMode::ALL {
            let mode_name = mixer_mode.cli_name().to_string();

            let mut playback = PlaybackState::start_with_mixer_mode(&module, mixer_mode).unwrap();
            let mut pcm_bytes = Vec::new();
            let total_frames = SAMPLE_RATE as usize;

            for _ in 0..total_frames {
                let (left_i32, right_i32) = playback
                    .render_raw_stereo_frame(&module, SAMPLE_RATE)
                    .unwrap();
                let left_i16 = left_i32.clamp(-32768, 32767) as i16;
                let right_i16 = right_i32.clamp(-32768, 32767) as i16;
                pcm_bytes.extend_from_slice(&left_i16.to_le_bytes());
                pcm_bytes.extend_from_slice(&right_i16.to_le_bytes());
            }

            let hash = fnv1a_hash(&pcm_bytes);
            if update_golden {
                file_hashes.insert(mode_name, hash);
            } else {
                let expected_hash = pcm_registry
                    .0
                    .get("synthetic_mod")
                    .and_then(|modes| modes.get(&mode_name))
                    .expect(&format!(
                        "Missing golden PCM hash for synthetic_mod in mode {mode_name}"
                    ));
                assert_eq!(
                    &hash, expected_hash,
                    "PCM output mismatch for synthetic_mod in mode {mode_name}"
                );
            }
        }

        if update_golden {
            pcm_registry
                .0
                .insert("synthetic_mod".to_string(), file_hashes);
        }

        let _ = std::fs::remove_file(temp_mod_path);
    }

    if update_golden {
        let serialized = serde_json::to_string_pretty(&pcm_registry).unwrap();
        std::fs::write(&pcm_hashes_path, serialized + "\n").unwrap();
    }
}
