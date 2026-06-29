use std::path::Path;

use eframe::egui;
use egui::{Key, Ui};
use rustytracker_core::{
    EffectCommand, Envelope, Instrument, InstrumentName, Module, Note, NoteName, Sample,
    SampleLoopKind, SampleName,
};
use rustytracker_edit::ModuleEditor;
use rustytracker_play::PlaybackState;
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};

mod audio;
mod tracker_ui;

use audio::AudioPlaybackEngine;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RustyTracker")
            .with_inner_size([1100.0, 750.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustyTracker",
        options,
        Box::new(|cc| Box::new(RustyTrackerApp::new(&cc.egui_ctx)) as Box<dyn eframe::App>),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActiveField {
    Note,
    Instrument,
    Effect0,
    Effect1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    PatternEditor,
    InstrumentEditor,
}

#[derive(Debug, Clone, Copy, Default)]
struct TrackerUiSettings {
    palette: tracker_ui::TrackerPalette,
}

#[derive(Default)]
struct InstrumentEditorEdits {
    instrument_name: Option<InstrumentName>,
    instrument_volume_fadeout: Option<u16>,
    volume_envelope: Option<Envelope>,
    sample_name: Option<SampleName>,
    sample_volume: Option<u8>,
    sample_panning: Option<u8>,
    sample_finetune: Option<i8>,
    sample_relative_note: Option<i8>,
    sample_loop_kind: Option<SampleLoopKind>,
    sample_loop_start: Option<u32>,
    sample_loop_length: Option<u32>,
}

impl InstrumentEditorEdits {
    fn has_changes(&self) -> bool {
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

    fn apply(self, instrument: &mut Instrument, sample: Option<&mut Sample>) {
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

struct RustyTrackerApp {
    editor: ModuleEditor,
    audio_engine: AudioPlaybackEngine,
    tracker_resources: tracker_ui::TrackerUiResources,
    ui_settings: TrackerUiSettings,
    edit_mode: bool,
    is_mod: bool,

    // Cursor position
    active_order_index: usize,
    active_row: u16,
    active_channel: u16,
    active_field: ActiveField,

    // Input state
    selected_instrument: u8,
    octave: u8,
    view_mode: ViewMode,
}

impl RustyTrackerApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let editor = ModuleEditor::new(Module::empty());
        let audio_engine = AudioPlaybackEngine::new();
        let tracker_resources = tracker_ui::TrackerUiResources::new(ctx);
        let ui_settings = TrackerUiSettings {
            palette: tracker_resources.palette(),
        };
        {
            if let Ok(mut state) = audio_engine.state.lock() {
                state.module = Some(editor.module().clone());
            }
        }
        Self {
            editor,
            audio_engine,
            tracker_resources,
            ui_settings,
            edit_mode: false,
            is_mod: false,
            active_order_index: 0,
            active_row: 0,
            active_channel: 0,
            active_field: ActiveField::Note,
            selected_instrument: 1,
            octave: 4,
            view_mode: ViewMode::PatternEditor,
        }
    }

    fn commit_edit_to_audio(&mut self) {
        let cloned_module = self.editor.module().clone();
        if let Ok(mut state) = self.audio_engine.state.lock() {
            state.module = Some(cloned_module);
        }
    }

    fn sync_playhead_position(&mut self) {
        if let Ok(state) = self.audio_engine.state.lock() {
            if state.is_playing {
                if let (Some(playback), Some(module)) = (&state.playback, &state.module) {
                    if let Ok(pos) = playback.clock().position(module) {
                        self.active_order_index = pos.order_index;
                        self.active_row = pos.row;
                    }
                }
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
        let is_playing = {
            if let Ok(state) = self.audio_engine.state.lock() {
                state.is_playing
            } else {
                false
            }
        };

        if is_playing {
            ctx.request_repaint();
        }
    }
}

impl RustyTrackerApp {
    fn render_menu_bar(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open Module (XM/MOD)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Tracker Modules (*.xm, *.mod)", &["xm", "mod"])
                        .pick_file()
                    {
                        self.load_module_file(&path);
                    }
                    ui.close_menu();
                }
                if ui.button("Save As (XM/MOD)...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Extended Module (*.xm)", &["xm"])
                        .add_filter("ProTracker Module (*.mod)", &["mod"])
                        .save_file()
                    {
                        self.save_module_file(&path);
                    }
                    ui.close_menu();
                }
                if ui.button("Export to WAV...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("WAVE Audio (*.wav)", &["wav"])
                        .save_file()
                    {
                        self.export_to_wav_file(&path);
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                let can_undo = self.editor.can_undo();
                let can_redo = self.editor.can_redo();

                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo (Ctrl+Z)"))
                    .clicked()
                {
                    self.editor.undo();
                    self.commit_edit_to_audio();
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo (Ctrl+Y)"))
                    .clicked()
                {
                    self.editor.redo();
                    self.commit_edit_to_audio();
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                ui.label("Palette");
                for palette in tracker_ui::TrackerPalette::ALL {
                    if ui
                        .selectable_label(self.ui_settings.palette == palette, palette.label())
                        .clicked()
                    {
                        self.ui_settings.palette = palette;
                        self.tracker_resources.set_palette(palette);
                        ui.close_menu();
                    }
                }
            });
        });
    }

    fn render_controls_bar(&mut self, ui: &mut Ui) {
        let theme = self.tracker_resources.theme();

        ui.horizontal(|ui| {
            let is_playing = {
                if let Ok(state) = self.audio_engine.state.lock() {
                    state.is_playing
                } else {
                    false
                }
            };

            if is_playing {
                if tracker_ui::show_toolbar_button(
                    ui,
                    &self.tracker_resources,
                    "PAUSE",
                    true,
                    theme.pattern_instrument,
                )
                .clicked()
                {
                    if let Ok(mut state) = self.audio_engine.state.lock() {
                        state.is_playing = false;
                    }
                }
            } else if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "PLAY",
                false,
                theme.pattern_note,
            )
            .clicked()
            {
                let cloned_module = self.editor.module().clone();
                let playback_state = PlaybackState::start_with_config(&cloned_module, self.is_mod).ok();
                if let Ok(mut state) = self.audio_engine.state.lock() {
                    state.is_playing = true;
                    state.module = Some(cloned_module);
                    if state.playback.is_none() {
                        state.playback = playback_state;
                    }
                }
            }

            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "STOP",
                false,
                theme.pattern_effect,
            )
            .clicked()
            {
                if let Ok(mut state) = self.audio_engine.state.lock() {
                    state.is_playing = false;
                    state.playback = None;
                }
                self.active_row = 0;
                self.active_order_index = 0;
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);

            let edit_text = if self.edit_mode {
                "EDIT ON"
            } else {
                "EDIT OFF"
            };
            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                edit_text,
                self.edit_mode,
                theme.cursor_line_highlight,
            )
            .clicked()
            {
                self.edit_mode = !self.edit_mode;
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);

            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "OCT -",
                false,
                theme.foreground,
            )
            .clicked()
            {
                self.octave = self.octave.saturating_sub(1);
            }
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("OCT {}", self.octave),
                theme.pattern_instrument,
            );
            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "OCT +",
                false,
                theme.foreground,
            )
            .clicked()
            {
                self.octave = (self.octave + 1).min(8);
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);

            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "PATTERN",
                self.view_mode == ViewMode::PatternEditor,
                theme.pattern_note,
            )
            .clicked()
            {
                self.view_mode = ViewMode::PatternEditor;
            }
            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "INSTR",
                self.view_mode == ViewMode::InstrumentEditor,
                theme.pattern_instrument,
            )
            .clicked()
            {
                self.view_mode = ViewMode::InstrumentEditor;
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);

            let module = self.editor.module();
            let song_title = if module.header.title.as_str().is_empty() {
                "UNTITLED"
            } else {
                module.header.title.as_str()
            };
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("BPM {}", module.header.bpm),
                theme.foreground,
            );
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("SPD {}", module.header.tick_speed),
                theme.foreground,
            );
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("CHN {}", module.header.channel_count),
                theme.foreground,
            );
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("SONG {song_title}"),
                theme.foreground,
            );
        });
    }

    fn render_order_list(&mut self, ui: &mut Ui) {
        let theme = self.tracker_resources.theme();

        tracker_ui::show_list_heading(ui, &self.tracker_resources, "ORDER LIST");
        ui.separator();

        ui.horizontal(|ui| {
            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "ADD",
                false,
                theme.pattern_note,
            )
            .clicked()
            {
                let _ = self.editor.insert_duplicate_order(self.active_order_index);
                self.commit_edit_to_audio();
            }
            if tracker_ui::show_toolbar_button(
                ui,
                &self.tracker_resources,
                "DEL",
                false,
                theme.cursor_line_highlight,
            )
            .clicked()
            {
                let _ = self.editor.delete_order(self.active_order_index);
                self.commit_edit_to_audio();
                if self.active_order_index >= self.editor.module().orders.len() {
                    self.active_order_index = self.editor.module().orders.len().saturating_sub(1);
                }
            }
        });

        ui.separator();

        let orders_len = self.editor.module().orders.len();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for i in 0..orders_len {
                let pattern_val = self.editor.module().orders[i];
                let is_selected = i == self.active_order_index;

                let text = format!("ORD {i:02X}  PAT {pattern_val:02X}");
                let response = tracker_ui::show_list_row(
                    ui,
                    &self.tracker_resources,
                    &text,
                    is_selected,
                    theme.pattern_note,
                );
                if response.clicked() {
                    self.active_order_index = i;
                    if let Ok(mut state) = self.audio_engine.state.lock() {
                        state.playback = None; // Reset playback position
                    }
                }
            }
        });
    }

    fn render_instrument_list(&mut self, ui: &mut Ui) {
        let theme = self.tracker_resources.theme();

        tracker_ui::show_list_heading(ui, &self.tracker_resources, "INSTRUMENTS");
        ui.separator();

        let instruments = &self.editor.module().instruments;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, ins) in instruments.iter().enumerate() {
                let ins_num = (i + 1) as u8;
                let is_selected = ins_num == self.selected_instrument;

                let name = if ins.name.as_str().is_empty() {
                    "<empty>"
                } else {
                    ins.name.as_str()
                };

                let text = format!("{ins_num:02X}  {name}");
                let response = tracker_ui::show_list_row(
                    ui,
                    &self.tracker_resources,
                    &text,
                    is_selected,
                    theme.pattern_instrument,
                );
                if response.clicked() {
                    self.selected_instrument = ins_num;
                }
            }
        });
    }

    fn render_pattern_editor(&mut self, ui: &mut Ui) {
        let clicked_cell = {
            let module = self.editor.module();
            let active_pattern_idx = match module.orders.get(self.active_order_index) {
                Some(&idx) => idx as usize,
                None => 0,
            };

            let Some(pattern) = module.patterns.get(active_pattern_idx) else {
                return;
            };

            tracker_ui::show_pattern_editor(
                ui,
                &self.tracker_resources,
                pattern,
                tracker_ui::PatternView {
                    active_pattern_index: active_pattern_idx,
                    active_row: self.active_row,
                    active_channel: self.active_channel,
                    active_field: self.active_field,
                    edit_mode: self.edit_mode,
                },
            )
        };

        if let Some((channel, row)) = clicked_cell {
            self.active_channel = channel;
            self.active_row = row;
        }
    }

    fn load_module_file(&mut self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            let (parsed, is_mod) = if bytes.len() >= XM_HEADER_SIGNATURE_LENGTH
                && &bytes[..XM_HEADER_SIGNATURE_LENGTH] == XM_HEADER_SIGNATURE
            {
                (
                    rustytracker_xm::parse_xm_module(&bytes).map_err(|e| format!("{e:?}")),
                    false,
                )
            } else {
                (
                    rustytracker_mod::parse_mod_module(&bytes).map_err(|e| format!("{e:?}")),
                    true,
                )
            };

            match parsed {
                Ok(module) => {
                    self.editor = ModuleEditor::new(module);
                    self.active_row = 0;
                    self.active_order_index = 0;
                    self.active_channel = 0;
                    self.is_mod = is_mod;
                    let cloned_module = self.editor.module().clone();
                    if let Ok(mut state) = self.audio_engine.state.lock() {
                        state.module = Some(cloned_module);
                        state.playback = None;
                        state.is_playing = false;
                    }
                }
                Err(err) => {
                    eprintln!("Failed to parse module: {err}");
                }
            }
        }
    }

    fn export_to_wav_file(&self, path: &Path) {
        let module = self.editor.module();
        if let Ok(mut playback) = PlaybackState::start_with_config(module, self.is_mod) {
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

    fn save_module_file(&self, path: &Path) {
        let module = self.editor.module();
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

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

    fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            // 1. Navigation Keys (always active)
            if input.key_pressed(Key::ArrowDown) {
                let rows = self.get_active_pattern_rows();
                self.active_row = (self.active_row + 1) % rows;
            }
            if input.key_pressed(Key::ArrowUp) {
                let rows = self.get_active_pattern_rows();
                if self.active_row == 0 {
                    self.active_row = rows - 1;
                } else {
                    self.active_row -= 1;
                }
            }
            if input.key_pressed(Key::ArrowRight) {
                self.navigate_field_right();
            }
            if input.key_pressed(Key::ArrowLeft) {
                self.navigate_field_left();
            }

            // Page Up / Down jumping by 16 rows
            if input.key_pressed(Key::PageDown) {
                let rows = self.get_active_pattern_rows();
                self.active_row = (self.active_row + 16).min(rows - 1);
            }
            if input.key_pressed(Key::PageUp) {
                self.active_row = self.active_row.saturating_sub(16);
            }

            // Edit Mode toggle with Space
            if input.key_pressed(Key::Space) {
                self.edit_mode = !self.edit_mode;
            }

            // Edit operations (requires edit mode)
            if self.edit_mode {
                // Delete cell with Delete / Backspace
                if input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace) {
                    let active_pattern_idx = self.get_active_pattern_index();
                    match self.active_field {
                        ActiveField::Note => {
                            let _ = self.editor.set_note(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                Note::Empty,
                            );
                        }
                        ActiveField::Instrument => {
                            let _ = self.editor.set_instrument(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                0,
                            );
                        }
                        ActiveField::Effect0 => {
                            let _ = self.editor.set_effect(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                0,
                                EffectCommand::default(),
                            );
                        }
                        ActiveField::Effect1 => {
                            let _ = self.editor.set_effect(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                1,
                                EffectCommand::default(),
                            );
                        }
                    }
                    self.commit_edit_to_audio();
                }

                // Check for Note Off key (Num1)
                if input.key_pressed(Key::Num1) && self.active_field == ActiveField::Note {
                    let active_pattern_idx = self.get_active_pattern_index();
                    let _ = self.editor.set_note(
                        active_pattern_idx,
                        self.active_channel,
                        self.active_row,
                        Note::Off,
                    );
                    self.commit_edit_to_audio();
                    self.advance_row_after_edit();
                }

                // Check for note input keys
                if self.active_field == ActiveField::Note {
                    for key in [
                        Key::Z,
                        Key::S,
                        Key::X,
                        Key::D,
                        Key::C,
                        Key::V,
                        Key::G,
                        Key::B,
                        Key::H,
                        Key::N,
                        Key::J,
                        Key::M,
                        Key::Q,
                        Key::Num2,
                        Key::W,
                        Key::Num3,
                        Key::E,
                        Key::R,
                        Key::Num5,
                        Key::T,
                        Key::Num6,
                        Key::Y,
                        Key::Num7,
                        Key::U,
                        Key::I,
                        Key::Num9,
                        Key::O,
                        Key::Num0,
                        Key::P,
                    ] {
                        if input.key_pressed(key) {
                            if let Some((note_name, octave_offset)) =
                                key_to_note_and_octave_offset(key)
                            {
                                let final_octave =
                                    (self.octave as i8 + octave_offset).clamp(0, 8) as u8;
                                if let Ok(note) = Note::key(final_octave, note_name) {
                                    let active_pattern_idx = self.get_active_pattern_index();

                                    // Write note
                                    let _ = self.editor.set_note(
                                        active_pattern_idx,
                                        self.active_channel,
                                        self.active_row,
                                        note,
                                    );
                                    // Write selected instrument
                                    let _ = self.editor.set_instrument(
                                        active_pattern_idx,
                                        self.active_channel,
                                        self.active_row,
                                        self.selected_instrument,
                                    );
                                    self.commit_edit_to_audio();
                                    self.advance_row_after_edit();
                                }
                            }
                        }
                    }
                }

                // Check for hex key input (0-9, A-F) on instrument, volume, effect
                let hex_value = get_hex_key_value(input);
                if let Some(digit) = hex_value {
                    let active_pattern_idx = self.get_active_pattern_index();
                    let pattern = &self.editor.module().patterns[active_pattern_idx];
                    let cell = pattern
                        .cell(self.active_channel, self.active_row)
                        .cloned()
                        .unwrap_or_default();

                    match self.active_field {
                        ActiveField::Instrument => {
                            let new_ins = (cell.instrument << 4) | digit;
                            let _ = self.editor.set_instrument(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                new_ins,
                            );
                            self.commit_edit_to_audio();
                        }
                        ActiveField::Effect0 => {
                            let mut cmd = cell.effects.first().copied().unwrap_or_default();
                            cmd.operand = (cmd.operand << 4) | digit;
                            let _ = self.editor.set_effect(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                0,
                                cmd,
                            );
                            self.commit_edit_to_audio();
                        }
                        ActiveField::Effect1 => {
                            let mut cmd = cell.effects.get(1).copied().unwrap_or_default();
                            cmd.operand = (cmd.operand << 4) | digit;
                            let _ = self.editor.set_effect(
                                active_pattern_idx,
                                self.active_channel,
                                self.active_row,
                                1,
                                cmd,
                            );
                            self.commit_edit_to_audio();
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    fn get_active_pattern_index(&self) -> usize {
        let module = self.editor.module();
        match module.orders.get(self.active_order_index) {
            Some(&idx) => idx as usize,
            None => 0,
        }
    }

    fn get_active_pattern_rows(&self) -> u16 {
        let active_pat_idx = self.get_active_pattern_index();
        match self.editor.module().patterns.get(active_pat_idx) {
            Some(p) => p.rows(),
            None => 64,
        }
    }

    fn navigate_field_right(&mut self) {
        let pattern_idx = self.get_active_pattern_index();
        let channels = match self.editor.module().patterns.get(pattern_idx) {
            Some(p) => p.channels(),
            None => 4,
        };

        match self.active_field {
            ActiveField::Note => self.active_field = ActiveField::Instrument,
            ActiveField::Instrument => self.active_field = ActiveField::Effect0,
            ActiveField::Effect0 => self.active_field = ActiveField::Effect1,
            ActiveField::Effect1 => {
                self.active_field = ActiveField::Note;
                self.active_channel = (self.active_channel + 1) % channels;
            }
        }
    }

    fn navigate_field_left(&mut self) {
        let pattern_idx = self.get_active_pattern_index();
        let channels = match self.editor.module().patterns.get(pattern_idx) {
            Some(p) => p.channels(),
            None => 4,
        };

        match self.active_field {
            ActiveField::Note => {
                self.active_field = ActiveField::Effect1;
                if self.active_channel == 0 {
                    self.active_channel = channels - 1;
                } else {
                    self.active_channel -= 1;
                }
            }
            ActiveField::Instrument => self.active_field = ActiveField::Note,
            ActiveField::Effect0 => self.active_field = ActiveField::Instrument,
            ActiveField::Effect1 => self.active_field = ActiveField::Effect0,
        }
    }

    fn advance_row_after_edit(&mut self) {
        let rows = self.get_active_pattern_rows();
        self.active_row = (self.active_row + 1) % rows;
    }
}

fn key_to_note_and_octave_offset(key: Key) -> Option<(NoteName, i8)> {
    match key {
        // Lower octave (Z keyboard row)
        Key::Z => Some((NoteName::C, 0)),
        Key::S => Some((NoteName::CSharp, 0)),
        Key::X => Some((NoteName::D, 0)),
        Key::D => Some((NoteName::DSharp, 0)),
        Key::C => Some((NoteName::E, 0)),
        Key::V => Some((NoteName::F, 0)),
        Key::G => Some((NoteName::FSharp, 0)),
        Key::B => Some((NoteName::G, 0)),
        Key::H => Some((NoteName::GSharp, 0)),
        Key::N => Some((NoteName::A, 0)),
        Key::J => Some((NoteName::ASharp, 0)),
        Key::M => Some((NoteName::B, 0)),

        // Upper octave (Q keyboard row)
        Key::Q => Some((NoteName::C, 1)),
        Key::Num2 => Some((NoteName::CSharp, 1)),
        Key::W => Some((NoteName::D, 1)),
        Key::Num3 => Some((NoteName::DSharp, 1)),
        Key::E => Some((NoteName::E, 1)),
        Key::R => Some((NoteName::F, 1)),
        Key::Num5 => Some((NoteName::FSharp, 1)),
        Key::T => Some((NoteName::G, 1)),
        Key::Num6 => Some((NoteName::GSharp, 1)),
        Key::Y => Some((NoteName::A, 1)),
        Key::Num7 => Some((NoteName::ASharp, 1)),
        Key::U => Some((NoteName::B, 1)),
        Key::I => Some((NoteName::C, 2)),
        Key::Num9 => Some((NoteName::CSharp, 2)),
        Key::O => Some((NoteName::D, 2)),
        Key::Num0 => Some((NoteName::DSharp, 2)),
        Key::P => Some((NoteName::E, 2)),
        _ => None,
    }
}

fn get_hex_key_value(input: &egui::InputState) -> Option<u8> {
    if input.key_pressed(Key::Num0) {
        return Some(0);
    }
    if input.key_pressed(Key::Num1) {
        return Some(1);
    }
    if input.key_pressed(Key::Num2) {
        return Some(2);
    }
    if input.key_pressed(Key::Num3) {
        return Some(3);
    }
    if input.key_pressed(Key::Num4) {
        return Some(4);
    }
    if input.key_pressed(Key::Num5) {
        return Some(5);
    }
    if input.key_pressed(Key::Num6) {
        return Some(6);
    }
    if input.key_pressed(Key::Num7) {
        return Some(7);
    }
    if input.key_pressed(Key::Num8) {
        return Some(8);
    }
    if input.key_pressed(Key::Num9) {
        return Some(9);
    }
    if input.key_pressed(Key::A) {
        return Some(10);
    }
    if input.key_pressed(Key::B) {
        return Some(11);
    }
    if input.key_pressed(Key::C) {
        return Some(12);
    }
    if input.key_pressed(Key::D) {
        return Some(13);
    }
    if input.key_pressed(Key::E) {
        return Some(14);
    }
    if input.key_pressed(Key::F) {
        return Some(15);
    }
    None
}

impl RustyTrackerApp {
    fn render_instrument_editor(&mut self, ui: &mut Ui) {
        let theme = self.tracker_resources.theme();
        let ins_idx = (self.selected_instrument as usize).saturating_sub(1);

        let (edits, sample_idx) = {
            let module = self.editor.module();
            let Some(instrument) = module.instruments.get(ins_idx) else {
                tracker_ui::show_status_label(
                    ui,
                    &self.tracker_resources,
                    "SELECTED INSTRUMENT OUT OF RANGE",
                    theme.cursor_line_highlight,
                );
                return;
            };

            let mapped_sample_idx = instrument.sample_slots.first().and_then(|slot| *slot);
            let fallback_sample_idx = (ins_idx < module.samples.len()).then_some(ins_idx);
            let sample_idx = mapped_sample_idx.or(fallback_sample_idx);
            let sample = sample_idx.and_then(|index| module.samples.get(index));

            let mut edits = InstrumentEditorEdits::default();
            let mut name_str = instrument.name.as_str().to_string();
            let mut volume_fadeout = instrument.volume_fadeout;
            let mut volume_envelope = instrument.volume_envelope.clone();
            let mut volume_envelope_changed = false;

            ui.vertical(|ui| {
                tracker_ui::show_list_heading(
                    ui,
                    &self.tracker_resources,
                    "INSTRUMENT & SAMPLE EDITOR",
                );
                ui.separator();

                ui.columns(2, |columns| {
                    // Column 0: Instrument & Envelopes
                    columns[0].vertical(|ui| {
                        tracker_ui::show_panel(
                            ui,
                            &self.tracker_resources,
                            "INSTRUMENT SETTINGS",
                            |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Name:");
                                    if ui.text_edit_singleline(&mut name_str).changed() {
                                        let name = InstrumentName::new(&name_str);
                                        if name != instrument.name {
                                            edits.instrument_name = Some(name);
                                        }
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Volume Fadeout:");
                                    if ui
                                        .add(egui::Slider::new(&mut volume_fadeout, 0..=65535))
                                        .changed()
                                        && volume_fadeout != instrument.volume_fadeout
                                    {
                                        edits.instrument_volume_fadeout = Some(volume_fadeout);
                                    }
                                });
                            },
                        );

                        // Volume Envelope Settings
                        tracker_ui::show_panel(
                            ui,
                            &self.tracker_resources,
                            "VOLUME ENVELOPE",
                            |ui| {
                                let env = &mut volume_envelope;

                                let env_active = (env.flags & 1) != 0;
                                if tracker_ui::show_toolbar_button(
                                    ui,
                                    &self.tracker_resources,
                                    "VOLUME ENV",
                                    env_active,
                                    theme.pattern_note,
                                )
                                .clicked()
                                {
                                    if env_active {
                                        env.flags &= !1;
                                    } else {
                                        env.flags |= 1;
                                    }
                                    volume_envelope_changed = true;
                                }

                                let env_sustain = (env.flags & 2) != 0;
                                if tracker_ui::show_toolbar_button(
                                    ui,
                                    &self.tracker_resources,
                                    "SUSTAIN",
                                    env_sustain,
                                    theme.pattern_instrument,
                                )
                                .clicked()
                                {
                                    if env_sustain {
                                        env.flags &= !2;
                                    } else {
                                        env.flags |= 2;
                                    }
                                    volume_envelope_changed = true;
                                }
                                if env_sustain {
                                    ui.horizontal(|ui| {
                                        ui.label("Sustain Point:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut env.sustain_point)
                                                    .clamp_range(
                                                        0..=env.point_count.saturating_sub(1),
                                                    ),
                                            )
                                            .changed()
                                        {
                                            volume_envelope_changed = true;
                                        }
                                    });
                                }

                                let env_loop = (env.flags & 4) != 0;
                                if tracker_ui::show_toolbar_button(
                                    ui,
                                    &self.tracker_resources,
                                    "ENV LOOP",
                                    env_loop,
                                    theme.pattern_instrument,
                                )
                                .clicked()
                                {
                                    if env_loop {
                                        env.flags &= !4;
                                    } else {
                                        env.flags |= 4;
                                    }
                                    volume_envelope_changed = true;
                                }
                                if env_loop {
                                    ui.horizontal(|ui| {
                                        ui.label("Loop Start:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut env.loop_start_point)
                                                    .clamp_range(
                                                        0..=env.point_count.saturating_sub(1),
                                                    ),
                                            )
                                            .changed()
                                        {
                                            volume_envelope_changed = true;
                                        }
                                        ui.label("Loop End:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut env.loop_end_point)
                                                    .clamp_range(
                                                        0..=env.point_count.saturating_sub(1),
                                                    ),
                                            )
                                            .changed()
                                        {
                                            volume_envelope_changed = true;
                                        }
                                    });
                                }

                                ui.separator();
                                ui.label("Envelope Points:");

                                let mut to_remove = None;
                                for idx in 0..env.point_count as usize {
                                    if let Some(pt) = env.points.get_mut(idx) {
                                        ui.horizontal(|ui| {
                                            ui.label(format!("Pt {idx}:"));
                                            ui.label("Frame:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut pt.frame)
                                                        .clamp_range(0..=32767),
                                                )
                                                .changed()
                                            {
                                                volume_envelope_changed = true;
                                            }
                                            ui.label("Val:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut pt.value)
                                                        .clamp_range(0..=64),
                                                )
                                                .changed()
                                            {
                                                volume_envelope_changed = true;
                                            }

                                            if tracker_ui::show_toolbar_button(
                                                ui,
                                                &self.tracker_resources,
                                                "DEL",
                                                false,
                                                theme.cursor_line_highlight,
                                            )
                                            .clicked()
                                            {
                                                to_remove = Some(idx);
                                            }
                                        });
                                    }
                                }

                                if let Some(idx) = to_remove {
                                    env.points.remove(idx);
                                    env.point_count = env.points.len() as u8;
                                    volume_envelope_changed = true;
                                }

                                if env.point_count < 12
                                    && tracker_ui::show_toolbar_button(
                                        ui,
                                        &self.tracker_resources,
                                        "ADD POINT",
                                        false,
                                        theme.pattern_note,
                                    )
                                    .clicked()
                                {
                                    let last_frame =
                                        env.points.last().map(|p| p.frame + 10).unwrap_or(0);
                                    env.points.push(rustytracker_core::EnvelopePoint {
                                        frame: last_frame,
                                        value: 64,
                                    });
                                    env.point_count = env.points.len() as u8;
                                    volume_envelope_changed = true;
                                }
                            },
                        );
                    });

                    // Column 1: Sample Settings & Waveform
                    columns[1].vertical(|ui| {
                        if let Some(sample) = sample {
                            tracker_ui::show_panel(
                                ui,
                                &self.tracker_resources,
                                "SAMPLE SETTINGS",
                                |ui| {
                                    let mut s_name = sample.name.as_str().to_string();
                                    ui.horizontal(|ui| {
                                        ui.label("Name:");
                                        if ui.text_edit_singleline(&mut s_name).changed() {
                                            let name = SampleName::new(&s_name);
                                            if name != sample.name {
                                                edits.sample_name = Some(name);
                                            }
                                        }
                                    });

                                    let mut volume = sample.volume;
                                    ui.horizontal(|ui| {
                                        ui.label("Volume:");
                                        if ui.add(egui::Slider::new(&mut volume, 0..=255)).changed()
                                            && volume != sample.volume
                                        {
                                            edits.sample_volume = Some(volume);
                                        }
                                    });

                                    let mut panning = sample.panning;
                                    ui.horizontal(|ui| {
                                        ui.label("Panning:");
                                        if ui
                                            .add(egui::Slider::new(&mut panning, 0..=255))
                                            .changed()
                                            && panning != sample.panning
                                        {
                                            edits.sample_panning = Some(panning);
                                        }
                                    });

                                    let mut finetune = sample.finetune;
                                    let mut relative_note = sample.relative_note;
                                    ui.horizontal(|ui| {
                                        ui.label("Finetune:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut finetune)
                                                    .clamp_range(-128..=127),
                                            )
                                            .changed()
                                            && finetune != sample.finetune
                                        {
                                            edits.sample_finetune = Some(finetune);
                                        }
                                        ui.label("Relative Note:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut relative_note)
                                                    .clamp_range(-96..=95),
                                            )
                                            .changed()
                                            && relative_note != sample.relative_note
                                        {
                                            edits.sample_relative_note = Some(relative_note);
                                        }
                                    });

                                    // Loop settings
                                    let mut loop_kind = sample.loop_kind;
                                    ui.horizontal(|ui| {
                                        ui.label("Loop Mode:");
                                        for (kind, label) in [
                                            (SampleLoopKind::None, "NONE"),
                                            (SampleLoopKind::Forward, "FORWARD"),
                                            (SampleLoopKind::PingPong, "PINGPONG"),
                                        ] {
                                            if tracker_ui::show_toolbar_button(
                                                ui,
                                                &self.tracker_resources,
                                                label,
                                                loop_kind == kind,
                                                theme.pattern_instrument,
                                            )
                                            .clicked()
                                            {
                                                loop_kind = kind;
                                            }
                                        }
                                    });
                                    if loop_kind != sample.loop_kind {
                                        edits.sample_loop_kind = Some(loop_kind);
                                    }

                                    if loop_kind != SampleLoopKind::None {
                                        let mut loop_start = sample.loop_start;
                                        let mut loop_length = sample.loop_length;
                                        ui.horizontal(|ui| {
                                            ui.label("Loop Start:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut loop_start)
                                                        .clamp_range(0..=sample.length),
                                                )
                                                .changed()
                                                && loop_start != sample.loop_start
                                            {
                                                edits.sample_loop_start = Some(loop_start);
                                            }
                                            ui.label("Loop Length:");
                                            if ui
                                                .add(
                                                    egui::DragValue::new(&mut loop_length)
                                                        .clamp_range(0..=sample.length),
                                                )
                                                .changed()
                                                && loop_length != sample.loop_length
                                            {
                                                edits.sample_loop_length = Some(loop_length);
                                            }
                                        });
                                    }
                                },
                            );

                            // Waveform visualization
                            tracker_ui::show_panel(
                                ui,
                                &self.tracker_resources,
                                "SAMPLE WAVEFORM",
                                |ui| {
                                    let visible_loop_kind =
                                        edits.sample_loop_kind.unwrap_or(sample.loop_kind);
                                    let visible_loop_start =
                                        edits.sample_loop_start.unwrap_or(sample.loop_start);
                                    let visible_loop_length =
                                        edits.sample_loop_length.unwrap_or(sample.loop_length);

                                    tracker_ui::show_waveform(
                                        ui,
                                        &self.tracker_resources,
                                        tracker_ui::WaveformView {
                                            data: &sample.data,
                                            sample_length: sample.length,
                                            loop_kind: visible_loop_kind,
                                            loop_start: visible_loop_start,
                                            loop_length: visible_loop_length,
                                        },
                                    );
                                },
                            );
                        } else {
                            tracker_ui::show_panel(ui, &self.tracker_resources, "SAMPLE", |ui| {
                                tracker_ui::show_status_label(
                                    ui,
                                    &self.tracker_resources,
                                    "NO SAMPLE MAPPED",
                                    theme.muted_foreground,
                                );
                            });
                        }
                    });
                });
            });

            if volume_envelope_changed && volume_envelope != instrument.volume_envelope {
                edits.volume_envelope = Some(volume_envelope);
            }

            (edits, sample_idx)
        };

        if edits.has_changes()
            && self
                .editor
                .edit_instrument_and_sample_with_undo(ins_idx, sample_idx, |instrument, sample| {
                    edits.apply(instrument, sample);
                })
                .is_ok()
        {
            self.commit_edit_to_audio();
        }
    }
}
