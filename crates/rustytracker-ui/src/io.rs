use std::path::Path;
use rustytracker_core::Module;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};

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

pub(crate) fn save_module_file(module: &Module, path: &Path) -> Result<(), String> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    let bytes = if extension == "xm" {
        rustytracker_xm::write_xm_module(module).map_err(|e| format!("{e:?}"))
    } else if extension == "mod" {
        rustytracker_mod::write_mod_module(module).map_err(|e| format!("{e:?}"))
    } else {
        return Err("Unsupported file format. Please use .xm or .mod extension.".to_string());
    }?;

    std::fs::write(path, bytes).map_err(|e| format!("{e:?}"))
}

pub(crate) fn export_to_wav_file(
    module: &Module,
    mixer_mode: PlaybackMixerMode,
    path: &Path,
) -> Result<(), String> {
    let mut playback = PlaybackState::start_with_settings(
        module,
        PlaybackSettings::with_mixer_mode(mixer_mode),
    )
    .map_err(|e| format!("Failed to start playback for WAV rendering: {e:?}"))?;

    let wav_bytes = playback
        .render_to_wav(module, 44100)
        .map_err(|_| "Failed to render WAV bytes".to_string())?;

    std::fs::write(path, wav_bytes).map_err(|e| format!("Failed to write WAV file: {e:?}"))
}
