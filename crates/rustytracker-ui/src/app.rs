use eframe::egui;
use rustytracker_core::{
    Envelope, Instrument, InstrumentName, Module, Sample, SampleLoopKind, SampleName,
};
use rustytracker_edit::ModuleEditor;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};
use std::path::Path;

use crate::audio::AudioPlaybackEngine;
use crate::tracker_ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveField {
    Note,
    Instrument,
    Effect0,
    Effect1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ViewMode {
    PatternEditor,
    InstrumentEditor,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TrackerUiSettings {
    pub(crate) palette: tracker_ui::TrackerPalette,
}

#[derive(Default)]
pub(crate) struct InstrumentEditorEdits {
    pub(crate) instrument_name: Option<InstrumentName>,
    pub(crate) instrument_volume_fadeout: Option<u16>,
    pub(crate) volume_envelope: Option<Envelope>,
    pub(crate) sample_name: Option<SampleName>,
    pub(crate) sample_volume: Option<u8>,
    pub(crate) sample_panning: Option<u8>,
    pub(crate) sample_finetune: Option<i8>,
    pub(crate) sample_relative_note: Option<i8>,
    pub(crate) sample_loop_kind: Option<SampleLoopKind>,
    pub(crate) sample_loop_start: Option<u32>,
    pub(crate) sample_loop_length: Option<u32>,
}

impl InstrumentEditorEdits {
    pub(crate) fn has_changes(&self) -> bool {
        self.instrument_name.is_some()
            || self.instrument_volume_fadeout.is_some()
            || self.volume_envelope.is_some()
            || self.sample_name.is_some()
            || self.sample_volume.is_some()
            || self.sample_panning.is_some()
            || self.sample_finetune.is_some()
            || self.sample_relative_note.is_some()
            || self.sample_loop_kind.is_some()
            || self.sample_loop_start.is_some()
            || self.sample_loop_length.is_some()
    }

    pub(crate) fn apply(self, instrument: &mut Instrument, sample: Option<&mut Sample>) {
        if let Some(name) = self.instrument_name {
            instrument.name = name;
        }
        if let Some(volume_fadeout) = self.instrument_volume_fadeout {
            instrument.volume_fadeout = volume_fadeout;
        }
        if let Some(volume_envelope) = self.volume_envelope {
            instrument.volume_envelope = volume_envelope;
        }

        if let Some(sample) = sample {
            if let Some(name) = self.sample_name {
                sample.name = name;
            }
            if let Some(volume) = self.sample_volume {
                sample.volume = volume;
            }
            if let Some(panning) = self.sample_panning {
                sample.panning = panning;
            }
            if let Some(finetune) = self.sample_finetune {
                sample.finetune = finetune;
            }
            if let Some(relative_note) = self.sample_relative_note {
                sample.relative_note = relative_note;
            }
            if let Some(loop_kind) = self.sample_loop_kind {
                sample.loop_kind = loop_kind;
            }
            if let Some(loop_start) = self.sample_loop_start {
                sample.loop_start = loop_start;
            }
            if let Some(loop_length) = self.sample_loop_length {
                sample.loop_length = loop_length;
            }
        }
    }
}

pub struct RustyTrackerApp {
    pub(crate) editor: ModuleEditor,
    pub(crate) audio_engine: AudioPlaybackEngine,
    pub(crate) tracker_resources: tracker_ui::TrackerUiResources,
    pub(crate) ui_settings: TrackerUiSettings,
    pub(crate) edit_mode: bool,
    pub(crate) mixer_mode: PlaybackMixerMode,

    // Cursor position
    pub(crate) active_order_index: usize,
    pub(crate) active_row: u16,
    pub(crate) active_channel: u16,
    pub(crate) active_field: ActiveField,

    // Input state
    pub(crate) selected_instrument: u8,
    pub(crate) octave: u8,
    pub(crate) view_mode: ViewMode,
    pub(crate) preview_key: Option<egui::Key>,
}

impl RustyTrackerApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let editor = ModuleEditor::new(Module::empty());
        let audio_engine = AudioPlaybackEngine::new();
        let tracker_resources = tracker_ui::TrackerUiResources::new(ctx);
        let ui_settings = TrackerUiSettings {
            palette: tracker_resources.palette(),
        };
        audio_engine.update_module(editor.module().clone());

        Self {
            editor,
            audio_engine,
            tracker_resources,
            ui_settings,
            edit_mode: false,
            mixer_mode: PlaybackMixerMode::default(),
            active_order_index: 0,
            active_row: 0,
            active_channel: 0,
            active_field: ActiveField::Note,
            selected_instrument: 1,
            octave: 4,
            view_mode: ViewMode::PatternEditor,
            preview_key: None,
        }
    }

    pub(crate) fn commit_edit_to_audio(&mut self) {
        self.audio_engine
            .update_module(self.editor.module().clone());
    }

    pub(crate) fn sync_playhead_position(&mut self) {
        if self.audio_engine.is_playing() {
            let (order_index, row) = self.audio_engine.get_position();
            self.active_order_index = order_index;
            self.active_row = row;
        }
    }

    pub(crate) fn load_module_file(&mut self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            let parsed = if bytes.len() >= XM_HEADER_SIGNATURE_LENGTH
                && &bytes[..XM_HEADER_SIGNATURE_LENGTH] == XM_HEADER_SIGNATURE
            {
                rustytracker_xm::parse_xm_module(&bytes).map_err(|e| format!("{e:?}"))
            } else {
                rustytracker_mod::parse_mod_module(&bytes).map_err(|e| format!("{e:?}"))
            };

            match parsed {
                Ok(module) => {
                    self.editor = ModuleEditor::new(module);
                    self.active_row = 0;
                    self.active_order_index = 0;
                    self.active_channel = 0;
                    self.audio_engine
                        .update_module(self.editor.module().clone());
                    self.audio_engine.stop();
                }
                Err(err) => {
                    eprintln!("Failed to parse module: {err}");
                }
            }
        }
    }

    pub(crate) fn export_to_wav_file(&self, path: &Path) {
        let path_val = rustytracker_core::validation::validate_export_path(path, "wav");
        if !path_val.is_valid() {
            eprintln!("Failed to export WAV: {}", path_val.errors.join("; "));
            return;
        }
        let module = self.editor.module();
        if let Ok(mut playback) = PlaybackState::start_with_settings(
            module,
            PlaybackSettings::with_mixer_mode(self.mixer_mode),
        ) {
            if let Ok(wav_bytes) = playback.render_to_wav(module, 44100) {
                if let Err(e) = std::fs::write(path, wav_bytes) {
                    eprintln!("Failed to write WAV file: {e:?}");
                }
            } else {
                eprintln!("Failed to render WAV bytes");
            }
        } else {
            eprintln!("Failed to start playback for WAV rendering");
        }
    }

    pub(crate) fn save_module_file(&self, path: &Path) {
        let module = self.editor.module();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let path_val = rustytracker_core::validation::validate_export_path(path, &extension);
        if !path_val.is_valid() {
            eprintln!("Failed to save module: {}", path_val.errors.join("; "));
            return;
        }

        let module_val = rustytracker_core::validation::validate_module_for_export(module, &extension);
        if !module_val.is_valid() {
            eprintln!("Failed to save module: {}", module_val.errors.join("; "));
            return;
        }

        for warning in &module_val.warnings {
            eprintln!("WARNING: {}", warning);
        }

        let result = if extension == "xm" {
            rustytracker_xm::write_xm_module(module).map_err(|e| format!("{e:?}"))
        } else if extension == "mod" {
            rustytracker_mod::write_mod_module(module).map_err(|e| format!("{e:?}"))
        } else {
            Err("Unsupported file format. Please use .xm or .mod extension.".to_string())
        };

        match result {
            Ok(bytes) => {
                if let Err(e) = std::fs::write(path, bytes) {
                    eprintln!("Failed to write module file: {e:?}");
                }
            }
            Err(err) => {
                eprintln!("Failed to export module: {err}");
            }
        }
    }
}

impl eframe::App for RustyTrackerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Sync visual cursor with playhead position
        self.sync_playhead_position();

        // 2. Render GUI
        egui::TopBottomPanel::top("top_menu").show(ctx, |ui| {
            self.render_menu_bar(ui);
        });

        egui::TopBottomPanel::top("controls_panel").show(ctx, |ui| {
            self.render_controls_bar(ui);
        });

        egui::SidePanel::left("left_order_panel")
            .resizable(true)
            .default_width(180.0)
            .show(ctx, |ui| {
                self.render_order_list(ui);
            });

        egui::SidePanel::right("right_instrument_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                self.render_instrument_list(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.view_mode {
            ViewMode::PatternEditor => self.render_pattern_editor(ui),
            ViewMode::InstrumentEditor => self.render_instrument_editor(ui),
        });

        // 3. Process keyboard input
        self.handle_keyboard_input(ctx);

        // Keep requesting repaint if audio is playing to scroll playhead smoothly
        let is_playing = self.audio_engine.is_playing();

        if is_playing {
            ctx.request_repaint();
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_save_and_export_validations() {
        let ctx = egui::Context::default();
        let app = RustyTrackerApp::new(&ctx);

        let temp_dir = std::env::temp_dir();
        let invalid_path = temp_dir.join("non_existent_dir_12345/module.invalid");
        
        // This should return early without creating any file
        app.save_module_file(&invalid_path);
        assert!(!invalid_path.exists());

        app.export_to_wav_file(&invalid_path);
        assert!(!invalid_path.exists());
    }
}
