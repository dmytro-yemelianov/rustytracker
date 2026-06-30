use eframe::egui;
use rustytracker_core::{
    Envelope, Instrument, InstrumentName, Module, Sample, SampleLoopKind, SampleName,
};
use rustytracker_edit::ModuleEditor;
use rustytracker_play::{MixerTrackControl, PlaybackMixerMode};
use std::path::Path;

use crate::io;
use crate::playback::AudioPlaybackEngine;
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
    pub(crate) panning_envelope: Option<Envelope>,
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
            || self.panning_envelope.is_some()
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
        if let Some(panning_envelope) = self.panning_envelope {
            instrument.panning_envelope = panning_envelope;
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
    pub(crate) last_file_operation: Option<io::FileOperationStatus>,

    // Cursor position
    pub(crate) active_order_index: usize,
    pub(crate) active_row: u16,
    pub(crate) active_channel: u16,
    pub(crate) active_track: Option<u16>,
    pub(crate) active_field: ActiveField,

    pub(crate) track_controls: Vec<MixerTrackControl>,
    pub(crate) track_activity_mask: u32,

    // Input state
    pub(crate) selected_instrument: u8,
    pub(crate) octave: u8,
    pub(crate) view_mode: ViewMode,
    pub(crate) pressed_keys: Vec<egui::Key>,
}

impl RustyTrackerApp {
    fn track_controls_for_channel_count(channel_count: usize) -> Vec<MixerTrackControl> {
        vec![MixerTrackControl::default(); channel_count]
    }

    pub fn new(ctx: &egui::Context) -> Self {
        let editor = ModuleEditor::new(Module::empty());
        let audio_engine = AudioPlaybackEngine::new();
        let tracker_resources = tracker_ui::TrackerUiResources::new(ctx);
        let ui_settings = TrackerUiSettings {
            palette: tracker_resources.palette(),
        };
        let track_controls =
            Self::track_controls_for_channel_count(editor.module().header.channel_count as usize);
        audio_engine.update_module(editor.module().clone());
        audio_engine.set_track_controls(track_controls.clone());

        Self {
            editor,
            audio_engine,
            tracker_resources,
            ui_settings,
            edit_mode: false,
            mixer_mode: PlaybackMixerMode::default(),
            last_file_operation: None,
            active_order_index: 0,
            active_row: 0,
            active_channel: 0,
            active_track: None,
            active_field: ActiveField::Note,
            track_controls,
            track_activity_mask: 0,
            selected_instrument: 1,
            octave: 4,
            view_mode: ViewMode::PatternEditor,
            pressed_keys: Vec::new(),
        }
    }

    pub(crate) fn last_file_operation_status(&self) -> Option<&io::FileOperationStatus> {
        self.last_file_operation.as_ref()
    }

    pub(crate) fn commit_edit_to_audio(&mut self) {
        self.audio_engine
            .update_module(self.editor.module().clone());
    }

    pub(crate) fn sync_track_controls_to_audio(&mut self) {
        let channel_count = self.editor.module().header.channel_count as usize;
        if self.track_controls.len() != channel_count {
            self.track_controls = Self::track_controls_for_channel_count(channel_count);
        }
        self.audio_engine
            .set_track_controls(self.track_controls.clone());
    }

    pub(crate) fn sync_playhead_position(&mut self) {
        let playback_status = self.audio_engine.playback_status();
        self.track_activity_mask = playback_status.track_activity_mask;
        self.active_track = playback_status.active_track.map(u16::from);

        if self.audio_engine.is_playing() {
            let (order_index, row) = (playback_status.order_index, playback_status.row);
            self.active_order_index = order_index;
            self.active_row = row;
            if let Some(active_track) = playback_status.active_track {
                self.active_channel = u16::from(active_track);
            }
        }
    }

    pub(crate) fn load_module_file(&mut self, path: &Path) {
        match io::load_module_file(path) {
            Ok(module) => {
                self.editor = ModuleEditor::new(module);
                let channel_count = self.editor.module().header.channel_count as usize;
                self.active_row = 0;
                self.active_order_index = 0;
                self.active_channel = 0;
                self.active_track = None;
                self.track_controls = Self::track_controls_for_channel_count(channel_count);
                self.audio_engine
                    .update_module(self.editor.module().clone());
                self.audio_engine
                    .set_track_controls(self.track_controls.clone());
                self.audio_engine.stop();
                self.last_file_operation = Some(io::FileOperationStatus::success(
                    io::FileOperation::LoadModule,
                    path,
                    "Loaded module",
                ));
            }
            Err(err) => {
                let message = file_operation_error_message("Failed to load module", &err);
                eprintln!("{message}");
                self.last_file_operation = Some(io::FileOperationStatus::failure(
                    io::FileOperation::LoadModule,
                    path,
                    message,
                ));
            }
        }
    }

    pub(crate) fn export_to_wav_file(&mut self, path: &Path) {
        match io::export_to_wav_file(self.editor.module(), self.mixer_mode, path) {
            Ok(()) => {
                self.last_file_operation = Some(io::FileOperationStatus::success(
                    io::FileOperation::ExportWav,
                    path,
                    "Exported WAV",
                ));
            }
            Err(err) => {
                let message = file_operation_error_message("Failed to export WAV", &err);
                eprintln!("{message}");
                self.last_file_operation = Some(io::FileOperationStatus::failure(
                    io::FileOperation::ExportWav,
                    path,
                    message,
                ));
            }
        }
    }

    pub(crate) fn save_module_file(&mut self, path: &Path) {
        match io::save_module_file(self.editor.module(), path) {
            Ok(report) => {
                self.last_file_operation = Some(io::FileOperationStatus::success_with_details(
                    io::FileOperation::SaveModule,
                    path,
                    "Saved module",
                    report.warnings,
                ));
            }
            Err(err) => {
                let message = file_operation_error_message("Failed to save module", &err);
                eprintln!("{message}");
                self.last_file_operation = Some(io::FileOperationStatus::failure(
                    io::FileOperation::SaveModule,
                    path,
                    message,
                ));
            }
        }
    }
}

fn file_operation_error_message(prefix: &str, err: &str) -> String {
    if err.starts_with(prefix) {
        err.to_string()
    } else {
        format!("{prefix}: {err}")
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
                self.render_track_controls(ui);
                ui.separator();
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
    use rustytracker_core::Module;

    #[test]
    fn test_app_save_and_export_validations() {
        let ctx = egui::Context::default();
        let mut app = RustyTrackerApp::new(&ctx);

        let temp_dir = std::env::temp_dir();
        let invalid_path = temp_dir.join("non_existent_dir_12345/module.invalid");

        // This should return early without creating any file
        app.save_module_file(&invalid_path);
        assert!(!invalid_path.exists());

        app.export_to_wav_file(&invalid_path);
        assert!(!invalid_path.exists());
    }

    #[test]
    fn app_records_file_operation_failures() {
        let ctx = egui::Context::default();
        let mut app = RustyTrackerApp::new(&ctx);
        let invalid_path = unique_temp_path("module.invalid");

        app.save_module_file(&invalid_path);
        let status = app.last_file_operation_status().unwrap();
        assert_eq!(status.operation, io::FileOperation::SaveModule);
        assert!(status.is_failure());
        assert_eq!(status.path, invalid_path);
        assert!(status.message.contains("Failed to save module"));
        assert!(status.message.contains("Unsupported file format"));
        assert!(!invalid_path.exists());

        app.export_to_wav_file(&invalid_path);
        let status = app.last_file_operation_status().unwrap();
        assert_eq!(status.operation, io::FileOperation::ExportWav);
        assert!(status.is_failure());
        assert_eq!(status.path, invalid_path);
        assert!(status.message.contains("Failed to export WAV"));
        assert!(status.message.contains("expected '.wav'"));
        assert!(!invalid_path.exists());

        let missing_path = unique_temp_path("missing.xm");
        app.load_module_file(&missing_path);
        let status = app.last_file_operation_status().unwrap();
        assert_eq!(status.operation, io::FileOperation::LoadModule);
        assert!(status.is_failure());
        assert_eq!(status.path, missing_path);
        assert!(status.message.contains("Failed to load module"));
    }

    #[test]
    fn app_records_success_status_and_save_warnings() {
        let ctx = egui::Context::default();
        let mut app = RustyTrackerApp::new(&ctx);
        let path = unique_temp_path("module.xm");

        app.save_module_file(&path);
        let status = app.last_file_operation_status().unwrap();
        assert_eq!(status.operation, io::FileOperation::SaveModule);
        assert!(status.is_success());
        assert_eq!(status.path, path);
        assert_eq!(status.message, "Saved module");
        assert!(status
            .details
            .iter()
            .any(|warning| warning == "Module title is empty."));
        assert!(path.exists());

        app.load_module_file(&path);
        let status = app.last_file_operation_status().unwrap();
        assert_eq!(status.operation, io::FileOperation::LoadModule);
        assert!(status.is_success());
        assert_eq!(status.path, path);
        assert_eq!(status.message, "Loaded module");

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn track_controls_for_channel_count_defaults_enabled_tracks() {
        let controls = RustyTrackerApp::track_controls_for_channel_count(4);

        assert_eq!(controls.len(), 4);
        assert!(controls.iter().all(|control| control.armed));
        assert!(controls.iter().all(|control| !control.solo));
        assert!(controls.iter().all(|control| !control.muted));
        assert!(controls.iter().all(|control| !control.stopped));
        assert!(controls.iter().all(|control| control.volume == u8::MAX));
    }

    #[test]
    fn sync_track_controls_resizes_when_channel_count_changes() {
        let ctx = egui::Context::default();
        let mut app = RustyTrackerApp::new(&ctx);
        let expanded = Module::empty_with_channels(8).expect("valid channel count");

        app.track_controls = vec![];
        app.editor = ModuleEditor::new(expanded);
        app.sync_track_controls_to_audio();

        assert_eq!(app.track_controls.len(), 8);
    }

    fn unique_temp_path(file_name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "rustytracker-ui-{}-{unique}-{file_name}",
            std::process::id()
        ))
    }
}
