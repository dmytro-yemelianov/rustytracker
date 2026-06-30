use std::io::Read;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use rustytracker_core::{
    EffectCommand, Module, ModuleTitle, Note, Pattern, Sample, SampleData, SampleLoopKind,
    SampleName, DEFAULT_SONG_CHANNELS,
};
use rustytracker_edit::ModuleEditor;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};
use rustytracker_xm::parse_xm_module;
use rustytracker_xm::write_xm_module;
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{dump_module_to_json, play_state_module_to_json};
use rustytracker_mod::parse_mod_module;
use rustytracker_mod::write_mod_module;

const API_PROTOCOL_VERSION: u16 = 1;
const API_DEFAULT_SAMPLE_RATE: u32 = 44_100;
const API_MAX_WAV_RENDER_SECONDS: u32 = 3600;
const WAV_HEADER_SIZE: usize = 44;

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    schema_version: u16,
    ok: bool,
    id: Option<String>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ApiError>,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    code: String,
    message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ApiMethod {
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "module.new")]
    ModuleNew,
    #[serde(rename = "module.load")]
    ModuleLoad,
    #[serde(rename = "module.dump")]
    ModuleDump,
    #[serde(rename = "module.play_state")]
    ModulePlayState,
    #[serde(rename = "module.render_wav")]
    ModuleRenderWav,
    #[serde(rename = "module.launch_ui")]
    ModuleLaunchUi,
    #[serde(rename = "module.write")]
    ModuleWrite,
    #[serde(rename = "module.apply_patch")]
    ModuleApplyPatch,
    #[serde(rename = "module.write_xm")]
    ModuleWriteXm,
    #[serde(rename = "module.write_mod")]
    ModuleWriteMod,
    #[serde(rename = "api.methods")]
    ApiMethods,
}

impl ApiMethod {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::ModuleNew => "module.new",
            Self::ModuleLoad => "module.load",
            Self::ModuleDump => "module.dump",
            Self::ModulePlayState => "module.play_state",
            Self::ModuleRenderWav => "module.render_wav",
            Self::ModuleLaunchUi => "module.launch_ui",
            Self::ModuleWrite => "module.write",
            Self::ModuleApplyPatch => "module.apply_patch",
            Self::ModuleWriteXm => "module.write_xm",
            Self::ModuleWriteMod => "module.write_mod",
            Self::ApiMethods => "api.methods",
        }
    }

    fn requires_module(&self) -> bool {
        !matches!(self, Self::Ping | Self::ApiMethods | Self::ModuleNew)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    pub id: Option<String>,
    pub method: ApiMethod,
    #[serde(default)]
    pub params: ApiParams,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiParams {
    pub module_path: Option<String>,
    pub module_bytes_b64: Option<String>,
    pub module_format_hint: Option<String>,
    pub module_title: Option<String>,
    pub module_channel_count: Option<u16>,
    pub rows: Option<usize>,
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub max_frames: Option<u64>,
    pub mixer: Option<String>,
    #[serde(default)]
    pub ui_binary: Option<String>,
    #[serde(default)]
    pub output_format: Option<String>,
    #[serde(default)]
    pub output_path: Option<String>,
    #[serde(default)]
    pub include_wav: Option<bool>,
    #[serde(default)]
    pub patch: Vec<ApiPatch>,
}

impl Default for ApiParams {
    fn default() -> Self {
        Self {
            module_path: None,
            module_bytes_b64: None,
            module_format_hint: None,
            module_title: None,
            module_channel_count: None,
            rows: None,
            sample_rate: None,
            duration_ms: None,
            max_frames: None,
            mixer: None,
            ui_binary: None,
            output_format: None,
            output_path: None,
            include_wav: None,
            patch: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ApiPatch {
    InsertTrack {
        index: u16,
    },
    DeleteTrack {
        index: u16,
    },
    CreateInstrument {
        index: Option<usize>,
        name: Option<String>,
        default_sample_index: Option<usize>,
    },
    DeleteInstrument {
        index: usize,
    },
    RenameInstrument {
        index: usize,
        name: String,
    },
    CreatePattern {
        rows: Option<u16>,
    },
    DeletePattern {
        index: usize,
    },
    CreateSample {
        index: Option<usize>,
        sample: ApiSamplePatch,
    },
    DeleteSample {
        index: usize,
    },
    RenameSample {
        index: usize,
        name: String,
    },
    SetNote {
        pattern: usize,
        channel: u16,
        row: u16,
        note: u8,
    },
    SetInstrument {
        pattern: usize,
        channel: u16,
        row: u16,
        instrument: u8,
    },
    SetEffect {
        pattern: usize,
        channel: u16,
        row: u16,
        slot: u8,
        effect: u8,
        operand: u8,
    },
    ClearCell {
        pattern: usize,
        channel: u16,
        row: u16,
    },
    InsertOrder {
        index: usize,
    },
    DeleteOrder {
        index: usize,
    },
    SetOrderPattern {
        index: usize,
        pattern: u8,
    },
    MoveOrder {
        from_index: usize,
        to_index: usize,
    },
    TransposeSelection {
        pattern: usize,
        start_channel: u16,
        end_channel: u16,
        start_row: u16,
        end_row: u16,
        semitones: i8,
    },
    ClearSelection {
        pattern: usize,
        start_channel: u16,
        end_channel: u16,
        start_row: u16,
        end_row: u16,
        clear_notes: bool,
        clear_instruments: bool,
        clear_effects: bool,
    },
    InsertRow {
        pattern: usize,
        row: u16,
    },
    DeleteRow {
        pattern: usize,
        row: u16,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiSamplePatch {
    pub name: Option<String>,
    pub length: Option<u32>,
    pub loop_start: Option<u32>,
    pub loop_length: Option<u32>,
    pub loop_kind: Option<String>,
    pub volume: Option<u8>,
    pub panning: Option<u8>,
    pub flags: Option<u8>,
    pub volume_fadeout: Option<u16>,
    pub sample_type: Option<u8>,
    pub finetune: Option<i8>,
    pub relative_note: Option<i8>,
    pub data: Option<ApiSampleDataPatch>,
}

impl Default for ApiSamplePatch {
    fn default() -> Self {
        Self {
            name: None,
            length: None,
            loop_start: None,
            loop_length: None,
            loop_kind: None,
            volume: None,
            panning: None,
            flags: None,
            volume_fadeout: None,
            sample_type: None,
            finetune: None,
            relative_note: None,
            data: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiSampleDataPatch {
    pub kind: Option<String>,
    pub data_b64: Option<String>,
}

impl Default for ApiSampleDataPatch {
    fn default() -> Self {
        Self {
            kind: None,
            data_b64: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    Xm,
    Mod,
}

impl OutputFormat {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Xm => "xm",
            Self::Mod => "mod",
        }
    }
}

impl TryFrom<&str> for OutputFormat {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase().as_str() {
            "xm" => Ok(Self::Xm),
            "mod" => Ok(Self::Mod),
            _ => Err(format!("unsupported output format: {value}")),
        }
    }
}

#[derive(Debug)]
enum ApiStatus {
    Ok(serde_json::Value),
    Err(ApiError),
}

impl ApiResponse {
    fn ok(id: Option<String>, method: &str, result: serde_json::Value) -> Self {
        Self {
            schema_version: API_PROTOCOL_VERSION,
            ok: true,
            id,
            method: method.to_string(),
            result: Some(result),
            error: None,
        }
    }

    fn err(id: Option<String>, method: &str, code: &str, message: String) -> Self {
        Self {
            schema_version: API_PROTOCOL_VERSION,
            ok: false,
            id,
            method: method.to_string(),
            result: None,
            error: Some(ApiError {
                code: code.to_string(),
                message,
            }),
        }
    }
}

pub fn api_request_to_json(request_json: &str) -> String {
    let response = match serde_json::from_str::<ApiRequest>(request_json) {
        Ok(request) => handle_api_request(request),
        Err(error) => ApiResponse::err(None, "parse", "invalid_request", error.to_string()),
    };

    let mut output = serde_json::to_string_pretty(&response)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialization_error\"}".to_string());
    if !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

pub fn api_request_to_json_file(path: &str) -> std::io::Result<String> {
    let request_json = std::fs::read_to_string(path)?;
    Ok(api_request_to_json(&request_json))
}

pub fn api_request_from_stdin() -> std::io::Result<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(api_request_to_json(&input))
}

fn handle_api_request(request: ApiRequest) -> ApiResponse {
    let method = request.method;

    if !method.requires_module() {
        return match method {
            ApiMethod::ModuleNew => handle_new_module_request(request.id, &request.params),
            ApiMethod::Ping => ApiResponse::ok(request.id, method.as_str(), json!({"status":"ok"})),
            ApiMethod::ApiMethods => {
                ApiResponse::ok(request.id, method.as_str(), api_methods_payload())
            }
            _ => ApiResponse::err(
                request.id,
                method.as_str(),
                "invalid_request",
                "unknown non-module method".to_string(),
            ),
        };
    }

    let (module, source_format) = match load_module_from_params(&request.params) {
        Ok(value) => value,
        Err(error) => {
            return ApiResponse::err(request.id, method.as_str(), "module_load_failed", error.1)
        }
    };

    let method_label = method.as_str();
    let result = match dispatch_method(method, &request.params, module, source_format) {
        Ok(value) => ApiStatus::Ok(value),
        Err(error) => ApiStatus::Err(ApiError {
            code: error.0,
            message: error.1,
        }),
    };

    match result {
        ApiStatus::Ok(value) => ApiResponse::ok(request.id, method_label, value),
        ApiStatus::Err(error) => {
            ApiResponse::err(request.id, method_label, &error.code, error.message)
        }
    }
}

fn handle_new_module_request(id: Option<String>, params: &ApiParams) -> ApiResponse {
    let mut module = match Module::empty_with_channels(
        params.module_channel_count.unwrap_or(DEFAULT_SONG_CHANNELS),
    ) {
        Ok(module) => module,
        Err(error) => {
            return ApiResponse::err(
                id,
                "module.new",
                "module_new_failed",
                format!("invalid module initialization: {error:?}"),
            )
        }
    };

    if let Some(title) = params.module_title.as_deref() {
        module.header.title = ModuleTitle::new(title);
    }

    if let Err(error) = normalize_new_module_patterns(&mut module) {
        return ApiResponse::err(id, "module.new", "module_new_failed", error);
    }

    let output_format = match parse_output_format(params.output_format.as_deref(), Some("xm")) {
        Ok(format) => format,
        Err(error) => {
            return ApiResponse::err(id, "module.new", &error.0, error.1);
        }
    };
    module.header.is_mod = matches!(output_format, OutputFormat::Mod);

    let mut editor = ModuleEditor::new(module);
    for patch in &params.patch {
        if let Err(error) = apply_patch(&mut editor, patch) {
            return ApiResponse::err(id, "module.new", &error.0, error.1);
        }
    }
    module = editor.into_module();

    let bytes = match write_module_bytes(&module, output_format) {
        Ok(bytes) => bytes,
        Err(error) => {
            return ApiResponse::err(id, "module.new", &error.0, error.1);
        }
    };

    let dump = match parse_module_dump(&module, output_format.as_str()) {
        Ok(value) => value,
        Err(error) => return ApiResponse::err(id, "module.new", &error.0, error.1),
    };

    ApiResponse::ok(
        id,
        "module.new",
        json!({
            "format": output_format.as_str(),
            "module_bytes_b64": STANDARD.encode(&bytes),
            "module": dump,
        }),
    )
}

fn normalize_new_module_patterns(module: &mut Module) -> Result<(), String> {
    let channel_count = module.header.channel_count;
    for pattern in &mut module.patterns {
        if pattern.channels() == channel_count {
            continue;
        }

        let rows = pattern.rows();
        let effect_slots = pattern.effect_slots();
        let mut resized = Pattern::new(rows, channel_count, effect_slots);
        let copy_channels = pattern.channels().min(channel_count);

        for row in 0..rows {
            for channel in 0..copy_channels {
                let source = pattern
                    .cell(channel, row)
                    .map_err(|error| format!("invalid default pattern state: {error:?}"))?;
                resized
                    .set_cell(channel, row, source.clone())
                    .map_err(|error| format!("invalid default pattern state: {error:?}"))?;
            }
        }

        *pattern = resized;
    }

    Ok(())
}

fn dispatch_method(
    method: ApiMethod,
    params: &ApiParams,
    mut module: Module,
    source_format: &'static str,
) -> Result<serde_json::Value, (String, String)> {
    match method {
        ApiMethod::ModuleLoad | ApiMethod::ModuleDump => {
            let dump = parse_module_dump(&module, source_format)?;
            Ok(json!({"format": source_format, "module": dump}))
        }
        ApiMethod::ModulePlayState => {
            let rows = params.rows.ok_or((
                "invalid_request".to_string(),
                "rows is required for module.play_state".to_string(),
            ))?;
            let json = play_state_module_to_json(&module, source_format, rows)
                .map_err(|error| ("play_state_failed".to_string(), error.to_string()))?;
            let value: serde_json::Value = serde_json::from_str(&json).map_err(|error| {
                (
                    "serialization_failed".to_string(),
                    format!("play-state dump deserialization failed: {error}"),
                )
            })?;
            Ok(value)
        }
        ApiMethod::ModuleRenderWav => {
            let sample_rate = params.sample_rate.unwrap_or(API_DEFAULT_SAMPLE_RATE);
            let mixer_mode = params
                .mixer
                .as_deref()
                .map(PlaybackMixerMode::from_name)
                .unwrap_or(Some(PlaybackMixerMode::default()))
                .ok_or((
                    "invalid_request".to_string(),
                    "invalid mixer mode".to_string(),
                ))?;
            let mut playback = PlaybackState::start_with_settings(
                &module,
                PlaybackSettings::with_mixer_mode(mixer_mode),
            )
            .map_err(|error| {
                (
                    "playback_failed".to_string(),
                    format!("failed to initialize playback: {error:?}"),
                )
            })?;
            let max_frames = render_max_frames(params.max_frames, params.duration_ms, sample_rate)?;
            let wav = match max_frames {
                Some(frame_count) => {
                    render_to_wav_with_frame_limit(&mut playback, &module, sample_rate, frame_count)
                        .map_err(|error| {
                            (
                                "playback_failed".to_string(),
                                format!("failed to render WAV: {error:?}"),
                            )
                        })?
                }
                None => playback
                    .render_to_wav(&module, sample_rate)
                    .map_err(|error| {
                        (
                            "playback_failed".to_string(),
                            format!("failed to render WAV: {error:?}"),
                        )
                    })?,
            };
            let include_wav = params.include_wav.unwrap_or(true);
            let mut result = json!({
                "format": source_format,
                "sample_rate": sample_rate,
                "mixer": mixer_mode.cli_name(),
                "frame_limit": max_frames,
                "source_format": source_format,
                "wav_bytes_b64": serde_json::Value::Null
            });

            if include_wav {
                let encoded = STANDARD.encode(&wav);
                result["wav_bytes_b64"] = json!(encoded);
            }

            if let Some(path) = &params.output_path {
                std::fs::write(path, &wav).map_err(|error| {
                    (
                        "render_failed".to_string(),
                        format!("failed to write rendered wav to '{path}': {error}"),
                    )
                })?;
                result["output_path"] = json!(path);
            }

            Ok(result)
        }
        ApiMethod::ModuleLaunchUi => {
            let output_format =
                parse_output_format(params.output_format.as_deref(), Some(source_format))?;
            let output_path = params
                .output_path
                .clone()
                .unwrap_or_else(|| make_temp_module_path(".tracker_api", output_format.as_str()));
            let bytes = write_module_bytes(&module, output_format).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("failed writing {}: {}", output_format.as_str(), error.1,),
                )
            })?;

            std::fs::write(&output_path, &bytes).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("failed to write module file '{output_path}': {error}"),
                )
            })?;

            let launch_command = params
                .ui_binary
                .clone()
                .unwrap_or_else(|| "rustytracker-ui".to_string());
            let child = Command::new(&launch_command)
                .arg(&output_path)
                .spawn()
                .map_err(|error| {
                    (
                        "ui_launch_failed".to_string(),
                        format!("failed to launch '{launch_command}' for '{output_path}': {error}"),
                    )
                })?;

            let pid = child.id();

            Ok(json!({
                "format": output_format.as_str(),
                "mode": "headed",
                "ui_binary": launch_command,
                "module_path": output_path,
                "pid": pid,
            }))
        }
        ApiMethod::ModuleWrite => {
            let output_format =
                parse_output_format(params.output_format.as_deref(), Some(source_format))?;
            let bytes = write_module_bytes(&module, output_format).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("failed writing {}: {}", output_format.as_str(), error.1),
                )
            })?;

            Ok(json!({
                "format": output_format.as_str(),
                "module_bytes_b64": STANDARD.encode(bytes),
            }))
        }
        ApiMethod::ModuleApplyPatch => {
            let mut editor = ModuleEditor::new(module);
            for patch in &params.patch {
                apply_patch(&mut editor, patch)?;
            }
            module = editor.into_module();
            let output_format =
                parse_output_format(params.output_format.as_deref(), Some(source_format))?;
            let bytes = write_module_bytes(&module, output_format).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("failed writing {}: {}", output_format.as_str(), error.1),
                )
            })?;
            Ok(json!({
                "format": output_format.as_str(),
                "patch_count": params.patch.len(),
                "module_bytes_b64": STANDARD.encode(bytes),
                "module": parse_module_dump(&module, output_format.as_str())?,
            }))
        }
        ApiMethod::ModuleWriteXm => {
            let bytes = write_xm_module(&module).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("xm write failed: {error:?}"),
                )
            })?;
            Ok(json!({"format":"xm","module_bytes_b64":STANDARD.encode(bytes)}))
        }
        ApiMethod::ModuleWriteMod => {
            let bytes = write_mod_module(&module).map_err(|error| {
                (
                    "module_write_failed".to_string(),
                    format!("mod write failed: {error:?}"),
                )
            })?;
            Ok(json!({"format":"mod","module_bytes_b64":STANDARD.encode(bytes)}))
        }
        ApiMethod::ModuleNew => unreachable!("module.new is handled before module loading"),
        ApiMethod::ApiMethods => unreachable!("api.methods is handled before module loading"),
        ApiMethod::Ping => unreachable!("ping is handled earlier"),
    }
}

fn parse_module_dump(module: &Module, format: &str) -> Result<serde_json::Value, (String, String)> {
    let format = match format {
        "xm" => "xm",
        "mod" => "mod",
        _ => {
            return Err((
                "invalid_request".to_string(),
                "invalid module format".to_string(),
            ))
        }
    };

    let dump = dump_module_to_json(module, format)
        .map_err(|error| ("dump_failed".to_string(), error.to_string()))?;
    serde_json::from_str(&dump).map_err(|error| {
        (
            "serialization_failed".to_string(),
            format!("module dump deserialization failed: {error}"),
        )
    })
}

fn api_methods_payload() -> serde_json::Value {
    json!({
        "api_version": API_PROTOCOL_VERSION,
        "methods": [
            "ping",
            "module.new",
            "api.methods",
            "module.load",
            "module.dump",
            "module.play_state",
            "module.render_wav",
            "module.launch_ui",
            "module.apply_patch",
            "module.write",
            "module.write_xm",
            "module.write_mod",
        ],
        "formats": ["xm", "mod"],
        "sample_data_kinds": [
            crate::SAMPLE_DATA_EMPTY,
            crate::SAMPLE_DATA_PCM8,
            crate::SAMPLE_DATA_PCM16,
        ],
        "patch_ops": [
            "insert_track",
            "delete_track",
            "create_instrument",
            "delete_instrument",
            "rename_instrument",
            "create_pattern",
            "delete_pattern",
            "create_sample",
            "delete_sample",
            "rename_sample",
            "set_note",
            "set_instrument",
            "set_effect",
            "clear_cell",
            "insert_order",
            "delete_order",
            "set_order_pattern",
            "move_order",
            "transpose_selection",
            "clear_selection",
            "insert_row",
            "delete_row",
        ],
        "mixer_modes": [
            "hifi",
            "rustysynth",
            "amiga",
            "protracker"
        ],
        "headless": [
            "module.new",
            "module.load",
            "module.dump",
            "module.play_state",
            "module.render_wav",
            "module.apply_patch",
            "module.write",
            "module.write_xm",
            "module.write_mod",
        ],
        "headed": ["module.launch_ui"],
    })
}

fn render_max_frames(
    explicit_max_frames: Option<u64>,
    duration_ms: Option<u64>,
    sample_rate: u32,
) -> Result<Option<usize>, (String, String)> {
    if explicit_max_frames.is_none() && duration_ms.is_none() {
        return Ok(None);
    }

    if sample_rate == 0 {
        return Err((
            "invalid_request".to_string(),
            "sample_rate must be > 0".to_string(),
        ));
    }

    let max_seconds = sample_rate as u64 * API_MAX_WAV_RENDER_SECONDS as u64;
    let max_frames = if let Some(frames) = explicit_max_frames {
        if frames == 0 {
            return Err((
                "invalid_request".to_string(),
                "max_frames must be > 0".to_string(),
            ));
        }
        if frames > max_seconds {
            return Err((
                "invalid_request".to_string(),
                format!("max_frames exceeds safety limit of {max_seconds}"),
            ));
        }
        frames
    } else {
        let duration_ms = duration_ms.unwrap_or_default();
        if duration_ms == 0 {
            return Err((
                "invalid_request".to_string(),
                "duration_ms must be > 0".to_string(),
            ));
        }
        let duration_frames = sample_rate as u128 * duration_ms as u128 / 1000;
        if duration_frames == 0 {
            return Err((
                "invalid_request".to_string(),
                "duration_ms too small for requested sample rate".to_string(),
            ));
        }
        if duration_frames > max_seconds as u128 {
            return Err((
                "invalid_request".to_string(),
                format!("duration_ms exceeds safety limit of {API_MAX_WAV_RENDER_SECONDS} seconds"),
            ));
        }
        duration_frames as u64
    };

    usize::try_from(max_frames).map(Some).map_err(|_| {
        (
            "invalid_request".to_string(),
            "max_frames does not fit in usize".to_string(),
        )
    })
}

fn render_to_wav_with_frame_limit(
    playback: &mut PlaybackState,
    module: &Module,
    sample_rate: u32,
    max_frames: usize,
) -> Result<Vec<u8>, (String, String)> {
    use std::io::{Cursor, Seek, SeekFrom, Write};

    if max_frames == 0 {
        return Err((
            "invalid_request".to_string(),
            "max_frames must be > 0".to_string(),
        ));
    }

    let mut buffer = Cursor::new(Vec::new());
    buffer.write_all(&[0u8; WAV_HEADER_SIZE]).map_err(|error| {
        (
            "playback_failed".to_string(),
            format!("failed to initialize wav buffer: {error}"),
        )
    })?;

    let mut total_frames_written: u32 = 0;
    for _ in 0..max_frames {
        if playback.song_ended() {
            break;
        }

        let (left_i32, right_i32) = playback
            .render_raw_stereo_frame(module, sample_rate)
            .map_err(|error| ("playback_failed".to_string(), format!("{error:?}")))?;
        let left_i16 = left_i32.clamp(-32768, 32767) as i16;
        let right_i16 = right_i32.clamp(-32768, 32767) as i16;

        buffer
            .write_all(&left_i16.to_le_bytes())
            .map_err(|error| ("playback_failed".to_string(), format!("{error}")))?;
        buffer
            .write_all(&right_i16.to_le_bytes())
            .map_err(|error| ("playback_failed".to_string(), format!("{error}")))?;
        total_frames_written += 1;
    }

    let data_size = total_frames_written * 4;
    let file_size = data_size + 36;
    let byte_rate = sample_rate * 4;
    let block_align: u16 = 4;
    let bits_per_sample: u16 = 16;
    let num_channels: u16 = 2;

    let mut header = [0u8; WAV_HEADER_SIZE];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&file_size.to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    let subchunk1_size: u32 = 16;
    header[16..20].copy_from_slice(&subchunk1_size.to_le_bytes());
    let audio_format: u16 = 1; // PCM
    header[20..22].copy_from_slice(&audio_format.to_le_bytes());
    header[22..24].copy_from_slice(&num_channels.to_le_bytes());
    header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    header[32..34].copy_from_slice(&block_align.to_le_bytes());
    header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_size.to_le_bytes());

    buffer
        .seek(SeekFrom::Start(0))
        .map_err(|error| ("playback_failed".to_string(), format!("{error}")))?;
    buffer
        .write_all(&header)
        .map_err(|error| ("playback_failed".to_string(), format!("{error}")))?;

    Ok(buffer.into_inner())
}

fn make_temp_module_path(prefix: &str, extension: &str) -> String {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_nanos())
        .unwrap_or_default();
    let file_name = format!("{prefix}_{suffix}.{extension}");
    std::env::temp_dir()
        .join(file_name)
        .to_string_lossy()
        .into_owned()
}

fn parse_output_format(
    requested: Option<&str>,
    source_format: Option<&str>,
) -> Result<OutputFormat, (String, String)> {
    if let Some(format) = requested {
        OutputFormat::try_from(format).map_err(|error| ("invalid_request".to_string(), error))
    } else if let Some(source_format) = source_format {
        OutputFormat::try_from(source_format).or_else(|_| Ok(OutputFormat::Xm))
    } else {
        Ok(OutputFormat::Xm)
    }
}

fn write_module_bytes(module: &Module, format: OutputFormat) -> Result<Vec<u8>, (String, String)> {
    match format {
        OutputFormat::Xm => write_xm_module(module).map_err(|error| {
            (
                "module_write_failed".to_string(),
                format!("xm write failed: {error:?}"),
            )
        }),
        OutputFormat::Mod => write_mod_module(module).map_err(|error| {
            (
                "module_write_failed".to_string(),
                format!("mod write failed: {error:?}"),
            )
        }),
    }
}

fn apply_patch(editor: &mut ModuleEditor, patch: &ApiPatch) -> Result<(), (String, String)> {
    match patch {
        ApiPatch::InsertTrack { index } => editor.insert_track(*index).map(|_| ()),
        ApiPatch::DeleteTrack { index } => editor.delete_track(*index),
        ApiPatch::CreateInstrument {
            index,
            name,
            default_sample_index,
        } => editor
            .create_instrument(*index, name.clone(), *default_sample_index)
            .map(|_| ()),
        ApiPatch::DeleteInstrument { index } => editor.delete_instrument(*index),
        ApiPatch::RenameInstrument { index, name } => {
            editor.rename_instrument(*index, name.clone())
        }
        ApiPatch::CreatePattern { rows } => editor.create_pattern(*rows).map(|_| ()),
        ApiPatch::DeletePattern { index } => editor.delete_pattern(*index),
        ApiPatch::CreateSample { index, sample } => editor
            .create_sample(*index, parse_sample_patch(sample)?)
            .map(|_| ()),
        ApiPatch::DeleteSample { index } => editor.delete_sample(*index),
        ApiPatch::RenameSample { index, name } => editor.rename_sample(*index, name.clone()),
        ApiPatch::SetNote {
            pattern,
            channel,
            row,
            note,
        } => editor.set_note(*pattern, *channel, *row, note_to_enum(*note)?),
        ApiPatch::SetInstrument {
            pattern,
            channel,
            row,
            instrument,
        } => editor.set_instrument(*pattern, *channel, *row, *instrument),
        ApiPatch::SetEffect {
            pattern,
            channel,
            row,
            slot,
            effect,
            operand,
        } => editor.set_effect(
            *pattern,
            *channel,
            *row,
            *slot,
            EffectCommand {
                effect: *effect,
                operand: *operand,
            },
        ),
        ApiPatch::ClearCell {
            pattern,
            channel,
            row,
        } => editor.clear_cell(*pattern, *channel, *row),
        ApiPatch::InsertOrder { index } => editor.insert_duplicate_order(*index),
        ApiPatch::DeleteOrder { index } => editor.delete_order(*index),
        ApiPatch::SetOrderPattern { index, pattern } => editor.set_order_pattern(*index, *pattern),
        ApiPatch::MoveOrder {
            from_index,
            to_index,
        } => editor.move_order(*from_index, *to_index),
        ApiPatch::TransposeSelection {
            pattern,
            start_channel,
            end_channel,
            start_row,
            end_row,
            semitones,
        } => editor.transpose_selection(
            *pattern,
            rustytracker_edit::Selection {
                start_channel: *start_channel,
                end_channel: *end_channel,
                start_row: *start_row,
                end_row: *end_row,
            },
            *semitones,
        ),
        ApiPatch::ClearSelection {
            pattern,
            start_channel,
            end_channel,
            start_row,
            end_row,
            clear_notes,
            clear_instruments,
            clear_effects,
        } => editor.clear_selection(
            *pattern,
            rustytracker_edit::Selection {
                start_channel: *start_channel,
                end_channel: *end_channel,
                start_row: *start_row,
                end_row: *end_row,
            },
            *clear_notes,
            *clear_instruments,
            *clear_effects,
        ),
        ApiPatch::InsertRow { pattern, row } => editor.insert_row(*pattern, *row),
        ApiPatch::DeleteRow { pattern, row } => editor.delete_row(*pattern, *row),
    }
    .map_err(|error| ("patch_failed".to_string(), format!("{error:?}")))
}

fn parse_sample_patch(sample: &ApiSamplePatch) -> Result<Sample, (String, String)> {
    let mut data = Sample::default();

    if let Some(name) = &sample.name {
        data.name = SampleName::new(name);
    }
    if let Some(value) = sample.length {
        data.length = value;
    }
    if let Some(value) = sample.loop_start {
        data.loop_start = value;
    }
    if let Some(value) = sample.loop_length {
        data.loop_length = value;
    }
    if let Some(value) = sample.volume {
        data.volume = value;
    }
    if let Some(value) = sample.panning {
        data.panning = value;
    }
    if let Some(value) = sample.flags {
        data.flags = value;
    }
    if let Some(value) = sample.volume_fadeout {
        data.volume_fadeout = value;
    }
    if let Some(value) = sample.sample_type {
        data.sample_type = value;
    }
    if let Some(value) = sample.finetune {
        data.finetune = value;
    }
    if let Some(value) = sample.relative_note {
        data.relative_note = value;
    }
    if let Some(value) = &sample.loop_kind {
        data.loop_kind = parse_sample_loop_kind(value.as_str())?;
    }
    if let Some(sample_data) = &sample.data {
        data.data = parse_sample_data(sample_data)?;
    }

    if data.length == 0 {
        data.length = data.data.frame_count() as u32;
    }

    Ok(data)
}

fn parse_sample_data(sample_data: &ApiSampleDataPatch) -> Result<SampleData, (String, String)> {
    let kind = sample_data
        .kind
        .as_deref()
        .unwrap_or("pcm8")
        .to_ascii_lowercase();
    match kind.as_str() {
        crate::SAMPLE_DATA_EMPTY => Ok(SampleData::default()),
        crate::SAMPLE_DATA_PCM8 | crate::SAMPLE_DATA_PCM16 => {
            let data_b64 = sample_data.data_b64.as_deref().ok_or_else(|| {
                (
                    "invalid_request".to_string(),
                    "sample.data_b64 is required for pcm8/pcm16".to_string(),
                )
            })?;

            if kind == crate::SAMPLE_DATA_PCM8 {
                parse_pcm8_frames(data_b64)
            } else {
                parse_pcm16_frames(data_b64)
            }
        }
        other => Err((
            "invalid_request".to_string(),
            format!("unsupported sample kind: {other}"),
        )),
    }
}

fn parse_pcm8_frames(data_b64: &str) -> Result<SampleData, (String, String)> {
    let bytes = STANDARD.decode(data_b64).map_err(|error| {
        (
            "invalid_request".to_string(),
            format!("failed to decode sample data_b64: {error}"),
        )
    })?;
    Ok(SampleData::pcm8(
        bytes.into_iter().map(|value| value as i8).collect(),
    ))
}

fn parse_pcm16_frames(data_b64: &str) -> Result<SampleData, (String, String)> {
    let bytes = STANDARD.decode(data_b64).map_err(|error| {
        (
            "invalid_request".to_string(),
            format!("failed to decode sample data_b64: {error}"),
        )
    })?;

    if bytes.len() % 2 != 0 {
        return Err((
            "invalid_request".to_string(),
            "pcm16 sample data must contain an even number of bytes".to_string(),
        ));
    }

    let values = bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    Ok(SampleData::pcm16(values))
}

fn parse_sample_loop_kind(value: &str) -> Result<SampleLoopKind, (String, String)> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(SampleLoopKind::None),
        "forward" => Ok(SampleLoopKind::Forward),
        "ping-pong" | "pingpong" => Ok(SampleLoopKind::PingPong),
        _ => Err((
            "invalid_request".to_string(),
            format!("invalid loop_kind: {value}. expected one of none, forward, ping-pong"),
        )),
    }
}

fn note_to_enum(raw: u8) -> Result<Note, (String, String)> {
    match raw {
        0 => Ok(Note::Empty),
        1..=96 => Ok(Note::Key(raw)),
        121 => Ok(Note::Off),
        _ => Err((
            "invalid_request".to_string(),
            "note must be 0 (empty), 1..96, or 121 (note-off)".to_string(),
        )),
    }
}

fn load_module_from_params(params: &ApiParams) -> Result<(Module, &'static str), (String, String)> {
    let bytes = if let Some(path) = &params.module_path {
        std::fs::read(path).map_err(|error| {
            (
                "module_load_failed".to_string(),
                format!("failed reading module file: {error}"),
            )
        })?
    } else if let Some(base64) = &params.module_bytes_b64 {
        STANDARD.decode(base64).map_err(|error| {
            (
                "module_load_failed".to_string(),
                format!("failed to decode module_bytes_b64: {error}"),
            )
        })?
    } else {
        return Err((
            "module_load_failed".to_string(),
            "module_path or module_bytes_b64 is required".to_string(),
        ));
    };

    let format_hint = params
        .module_format_hint
        .as_ref()
        .map(|format| format.to_ascii_lowercase());

    let (module, format) = match format_hint.as_deref() {
        Some("xm") => parse_module_xm(&bytes)?,
        Some("mod") => parse_module_mod(&bytes)?,
        Some(other) => {
            return Err((
                "invalid_request".to_string(),
                format!("unsupported module_format_hint: {other}"),
            ))
        }
        None => {
            if bytes.len() >= XM_HEADER_SIGNATURE_LENGTH
                && &bytes[..XM_HEADER_SIGNATURE_LENGTH] == XM_HEADER_SIGNATURE
            {
                parse_module_xm(&bytes)?
            } else {
                parse_module_mod(&bytes)?
            }
        }
    };

    Ok((module, format))
}

fn parse_module_xm(bytes: &[u8]) -> Result<(Module, &'static str), (String, String)> {
    parse_xm_module(bytes)
        .map(|module| (module, "xm"))
        .map_err(|error| {
            (
                "module_load_failed".to_string(),
                format!("xm parse failed: {error:?}"),
            )
        })
}

fn parse_module_mod(bytes: &[u8]) -> Result<(Module, &'static str), (String, String)> {
    parse_mod_module(bytes)
        .map(|module| (module, "mod"))
        .map_err(|error| {
            (
                "module_load_failed".to_string(),
                format!("mod parse failed: {error:?}"),
            )
        })
}
