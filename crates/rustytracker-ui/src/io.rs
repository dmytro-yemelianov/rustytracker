use rustytracker_core::Module;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperation {
    LoadModule,
    SaveModule,
    ExportWav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOperationOutcome {
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileOperationStatus {
    pub(crate) operation: FileOperation,
    pub(crate) outcome: FileOperationOutcome,
    pub(crate) path: PathBuf,
    pub(crate) message: String,
    pub(crate) details: Vec<String>,
}

impl FileOperationStatus {
    pub(crate) fn success(
        operation: FileOperation,
        path: &Path,
        message: impl Into<String>,
    ) -> Self {
        Self::success_with_details(operation, path, message, Vec::new())
    }

    pub(crate) fn success_with_details(
        operation: FileOperation,
        path: &Path,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self::new(
            operation,
            FileOperationOutcome::Success,
            path,
            message,
            details,
        )
    }

    pub(crate) fn failure(
        operation: FileOperation,
        path: &Path,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            operation,
            FileOperationOutcome::Failure,
            path,
            message,
            Vec::new(),
        )
    }

    pub(crate) fn is_success(&self) -> bool {
        self.outcome == FileOperationOutcome::Success
    }

    pub(crate) fn is_failure(&self) -> bool {
        self.outcome == FileOperationOutcome::Failure
    }

    fn new(
        operation: FileOperation,
        outcome: FileOperationOutcome,
        path: &Path,
        message: impl Into<String>,
        details: Vec<String>,
    ) -> Self {
        Self {
            operation,
            outcome,
            path: path.to_path_buf(),
            message: message.into(),
            details,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SaveModuleFileReport {
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn load_module_file(path: &Path) -> Result<Module, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{e:?}"))?;
    if bytes.len() >= XM_HEADER_SIGNATURE_LENGTH
        && &bytes[..XM_HEADER_SIGNATURE_LENGTH] == XM_HEADER_SIGNATURE
    {
        rustytracker_xm::parse_xm_module(&bytes).map_err(|e| format!("{e:?}"))
    } else {
        rustytracker_mod::parse_mod_module(&bytes).map_err(|e| format!("{e:?}"))
    }
}

pub(crate) fn save_module_file(
    module: &Module,
    path: &Path,
) -> Result<SaveModuleFileReport, String> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let path_val = rustytracker_core::validation::validate_export_path(path, &extension);
    if !path_val.is_valid() {
        return Err(format!(
            "Failed to save module: {}",
            path_val.errors.join("; ")
        ));
    }

    let module_val = rustytracker_core::validation::validate_module_for_export(module, &extension);
    if !module_val.is_valid() {
        return Err(format!(
            "Failed to save module: {}",
            module_val.errors.join("; ")
        ));
    }

    for warning in &module_val.warnings {
        eprintln!("WARNING: {}", warning);
    }
    let warnings = module_val.warnings;

    let bytes = if extension == "xm" {
        rustytracker_xm::write_xm_module(module).map_err(|e| format!("{e:?}"))
    } else if extension == "mod" {
        rustytracker_mod::write_mod_module(module).map_err(|e| format!("{e:?}"))
    } else {
        return Err("Unsupported file format. Please use .xm or .mod extension.".to_string());
    }?;

    std::fs::write(path, bytes)
        .map(|()| SaveModuleFileReport { warnings })
        .map_err(|e| format!("{e:?}"))
}

pub(crate) fn export_to_wav_file(
    module: &Module,
    mixer_mode: PlaybackMixerMode,
    path: &Path,
) -> Result<(), String> {
    let path_val = rustytracker_core::validation::validate_export_path(path, "wav");
    if !path_val.is_valid() {
        return Err(format!(
            "Failed to export WAV: {}",
            path_val.errors.join("; ")
        ));
    }

    let mut playback =
        PlaybackState::start_with_settings(module, PlaybackSettings::with_mixer_mode(mixer_mode))
            .map_err(|e| format!("Failed to start playback for WAV rendering: {e:?}"))?;

    let wav_bytes = playback
        .render_to_wav(module, 44100)
        .map_err(|_| "Failed to render WAV bytes".to_string())?;

    std::fs::write(path, wav_bytes).map_err(|e| format!("Failed to write WAV file: {e:?}"))
}
