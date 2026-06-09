use std::path::Path;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui;
use egui::{Color32, Key, RichText, Ui};
use rustytracker_core::{EffectCommand, Module, Note, NoteName, PatternCell};
use rustytracker_edit::ModuleEditor;
use rustytracker_play::PlaybackState;

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
        Box::new(|_cc| Box::new(RustyTrackerApp::new()) as Box<dyn eframe::App>),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveField {
    Note,
    Instrument,
    Effect0,
    Effect1,
}

struct AudioEngineState {
    playback: Option<PlaybackState>,
    module: Option<Module>,
    is_playing: bool,
    sample_rate: u32,
}

struct AudioPlaybackEngine {
    state: Arc<Mutex<AudioEngineState>>,
    _stream: Option<cpal::Stream>,
}

impl AudioPlaybackEngine {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("No default audio output device found!");
                return Self {
                    state: Arc::new(Mutex::new(AudioEngineState {
                        playback: None,
                        module: None,
                        is_playing: false,
                        sample_rate: 44100,
                    })),
                    _stream: None,
                };
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get default output config: {e}");
                return Self {
                    state: Arc::new(Mutex::new(AudioEngineState {
                        playback: None,
                        module: None,
                        is_playing: false,
                        sample_rate: 44100,
                    })),
                    _stream: None,
                };
            }
        };

        let sample_rate = config.sample_rate().0;
        let state = Arc::new(Mutex::new(AudioEngineState {
            playback: None,
            module: None,
            is_playing: false,
            sample_rate,
        }));

        let state_clone = Arc::clone(&state);
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
            _ => Err(cpal::BuildStreamError::DeviceNotAvailable),
        };

        let stream = match stream {
            Ok(s) => {
                let _ = s.play();
                Some(s)
            }
            Err(e) => {
                eprintln!("Failed to build audio output stream: {e}");
                None
            }
        };

        Self { state, _stream: stream }
    }
}

fn write_audio<T>(output: &mut [T], state_lock: &Arc<Mutex<AudioEngineState>>)
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    let mut state_guard = match state_lock.lock() {
        Ok(s) => s,
        Err(_) => {
            for sample in output.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            return;
        }
    };

    let state = &mut *state_guard;
    if !state.is_playing {
        for sample in output.iter_mut() {
            *sample = T::from_sample(0.0);
        }
        return;
    }

    let AudioEngineState {
        playback,
        module,
        sample_rate,
        ..
    } = state;

    let playback = match playback {
        Some(pb) => pb,
        None => {
            for sample in output.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            return;
        }
    };

    let module = match module {
        Some(m) => m,
        None => {
            for sample in output.iter_mut() {
                *sample = T::from_sample(0.0);
            }
            return;
        }
    };

    let sample_rate = *sample_rate;
    let mut song_ended = false;

    for frame in output.chunks_mut(2) {
        let sample = if !song_ended {
            match playback.render_raw_mono_pcm(module, sample_rate, 1) {
                Ok(frames) => {
                    if playback.song_ended() {
                        song_ended = true;
                        0.0
                    } else {
                        let raw = frames.first().copied().unwrap_or(0);
                        (raw.clamp(-32768, 32767) as f32) / 32768.0
                    }
                }
                Err(_) => {
                    song_ended = true;
                    0.0
                }
            }
        } else {
            0.0
        };

        let cpal_sample = T::from_sample(sample);
        for channel_out in frame {
            *channel_out = cpal_sample;
        }
    }

    if song_ended {
        state.is_playing = false;
        state.playback = None;
    }
}

struct RustyTrackerApp {
    editor: ModuleEditor,
    audio_engine: AudioPlaybackEngine,
    edit_mode: bool,

    // Cursor position
    active_order_index: usize,
    active_row: u16,
    active_channel: u16,
    active_field: ActiveField,

    // Input state
    selected_instrument: u8,
    octave: u8,
}

impl RustyTrackerApp {
    pub fn new() -> Self {
        let editor = ModuleEditor::new(Module::empty());
        let audio_engine = AudioPlaybackEngine::new();
        {
            if let Ok(mut state) = audio_engine.state.lock() {
                state.module = Some(editor.module().clone());
            }
        }
        Self {
            editor,
            audio_engine,
            edit_mode: false,
            active_order_index: 0,
            active_row: 0,
            active_channel: 0,
            active_field: ActiveField::Note,
            selected_instrument: 1,
            octave: 4,
        }
    }

    fn commit_edit_to_audio(&mut self) {
        if let Ok(mut state) = self.audio_engine.state.lock() {
            state.module = Some(self.editor.module().clone());
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

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_pattern_editor(ui);
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
        });
    }

    fn render_controls_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Playback controls
            let is_playing = {
                if let Ok(state) = self.audio_engine.state.lock() {
                    state.is_playing
                } else {
                    false
                }
            };

            if is_playing {
                if ui
                    .button(RichText::new("⏸ Pause").color(Color32::LIGHT_BLUE))
                    .clicked()
                {
                    if let Ok(mut state) = self.audio_engine.state.lock() {
                        state.is_playing = false;
                    }
                }
            } else if ui
                .button(RichText::new("▶ Play").color(Color32::LIGHT_GREEN))
                .clicked()
            {
                if let Ok(mut state) = self.audio_engine.state.lock() {
                    state.is_playing = true;
                    state.module = Some(self.editor.module().clone());
                    if state.playback.is_none() {
                        state.playback = PlaybackState::start(self.editor.module()).ok();
                    }
                }
            }

            if ui.button("⏹ Stop").clicked() {
                if let Ok(mut state) = self.audio_engine.state.lock() {
                    state.is_playing = false;
                    state.playback = None;
                }
                self.active_row = 0;
                self.active_order_index = 0;
            }

            ui.separator();

            // Edit Mode Indicator
            let edit_text = if self.edit_mode {
                RichText::new("● EDIT MODE").color(Color32::LIGHT_RED)
            } else {
                RichText::new("○ RECORD").color(Color32::GRAY)
            };
            if ui.button(edit_text).clicked() {
                self.edit_mode = !self.edit_mode;
            }

            ui.separator();

            // Octave Selector
            ui.label("Octave:");
            ui.add(egui::DragValue::new(&mut self.octave).clamp_range(0..=8));

            ui.separator();

            // BPM / Speed display
            let module = self.editor.module();
            ui.label(format!("BPM: {}", module.header.bpm));
            ui.label(format!("Speed: {}", module.header.tick_speed));
            ui.label(format!("Channels: {}", module.header.channel_count));

            ui.separator();

            // Display title of the loaded module
            ui.label(format!(
                "Song: {}",
                if module.header.title.as_str().is_empty() {
                    "Untitled"
                } else {
                    module.header.title.as_str()
                }
            ));
        });
    }

    fn render_order_list(&mut self, ui: &mut Ui) {
        ui.heading("Order List");
        ui.separator();

        let orders_len = self.editor.module().orders.len();

        ui.horizontal(|ui| {
            if ui.button("+ Add").clicked() {
                let _ = self.editor.insert_duplicate_order(self.active_order_index);
                self.commit_edit_to_audio();
            }
            if ui.button("- Del").clicked() {
                let _ = self.editor.delete_order(self.active_order_index);
                self.commit_edit_to_audio();
                if self.active_order_index >= self.editor.module().orders.len() {
                    self.active_order_index =
                        self.editor.module().orders.len().saturating_sub(1);
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for i in 0..orders_len {
                let pattern_val = self.editor.module().orders[i];
                let is_selected = i == self.active_order_index;

                let text = format!("Order {i:02X} : Pattern {pattern_val:02X}");

                let mut label = RichText::new(text).monospace();
                if is_selected {
                    label = label.color(Color32::YELLOW).strong();
                }

                let response = ui.selectable_label(is_selected, label);
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
        ui.heading("Instruments");
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
                let text = format!("{ins_num:02X} : {name}");

                let mut label = RichText::new(text).monospace();
                if is_selected {
                    label = label.color(Color32::LIGHT_BLUE).strong();
                }

                let response = ui.selectable_label(is_selected, label);
                if response.clicked() {
                    self.selected_instrument = ins_num;
                }
            }
        });
    }

    fn render_pattern_editor(&mut self, ui: &mut Ui) {
        let module = self.editor.module();
        let active_pattern_idx = match module.orders.get(self.active_order_index) {
            Some(&idx) => idx as usize,
            None => 0,
        };

        let pattern = match module.patterns.get(active_pattern_idx) {
            Some(p) => p,
            None => return,
        };

        let rows = pattern.rows();
        let channels = pattern.channels();

        ui.heading(format!("Pattern Editor : Pattern {active_pattern_idx:02X}"));
        ui.separator();

        let default_cell = PatternCell::default();
        let mut clicked_cell = None;

        // Editor table
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("pattern_grid")
                    .striped(true)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        // Render Column Headers
                        ui.label(RichText::new("Row").color(Color32::GRAY));
                        for c in 0..channels {
                            ui.label(
                                RichText::new(format!("Channel {:02}", c + 1))
                                    .color(Color32::LIGHT_BLUE)
                                    .strong(),
                            );
                        }
                        ui.end_row();

                        // Render Rows
                        for r in 0..rows {
                            let is_row_active = r == self.active_row;

                            // Display row index (e.g. "00")
                            let mut row_num_text = RichText::new(format!("{r:02X}")).monospace();
                            if is_row_active {
                                row_num_text = row_num_text.color(Color32::YELLOW).strong();
                            } else {
                                row_num_text = row_num_text.color(Color32::GRAY);
                            }
                            ui.label(row_num_text);

                            // Display channel cells
                            for c in 0..channels {
                                let cell = pattern.cell(c, r).unwrap_or(&default_cell);
                                
                                let note_str = format_note(cell.note);
                                let ins_str = format_instrument(cell.instrument);

                                let eff0_cmd = cell.effects.first().copied().unwrap_or_default();
                                let eff1_cmd = cell.effects.get(1).copied().unwrap_or_default();

                                let eff0_str = format_effect(eff0_cmd);
                                let eff1_str = format_effect(eff1_cmd);

                                let cell_text = format!("{note_str} {ins_str} {eff0_str} {eff1_str}");
                                let mut rich_text = RichText::new(cell_text).monospace();

                                // Highlight cells or cursors
                                let is_cursor_here = c == self.active_channel && r == self.active_row;

                                let response = if is_cursor_here {
                                    rich_text = rich_text.color(Color32::BLACK);
                                    let bg_color = if self.edit_mode {
                                        Color32::LIGHT_RED
                                    } else {
                                        Color32::YELLOW
                                    };
                                    ui.colored_label(bg_color, rich_text)
                                } else {
                                    if is_row_active {
                                        rich_text = rich_text.color(Color32::WHITE).strong();
                                    } else {
                                        rich_text = rich_text.color(Color32::GRAY);
                                    }
                                    ui.selectable_label(false, rich_text)
                                };

                                if response.clicked() {
                                    clicked_cell = Some((c, r));
                                }
                            }
                            ui.end_row();
                        }
                    });
            });

        if let Some((c, r)) = clicked_cell {
            self.active_channel = c;
            self.active_row = r;
        }
    }

    fn load_module_file(&mut self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            let parsed = if bytes.len() >= 17 && &bytes[0..17] == b"Extended Module: " {
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
                    if let Ok(mut state) = self.audio_engine.state.lock() {
                        state.module = Some(self.editor.module().clone());
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

fn format_note(note: Note) -> String {
    match note {
        Note::Empty => "...".to_string(),
        Note::Off => "====".to_string(),
        Note::Key(val) => {
            let val = val.saturating_sub(1);
            let octave = val / 12;
            let name_idx = val % 12;
            let name_str = match name_idx {
                0 => "C-",
                1 => "C#",
                2 => "D-",
                3 => "D#",
                4 => "E-",
                5 => "F-",
                6 => "F#",
                7 => "G-",
                8 => "G#",
                9 => "A-",
                10 => "A#",
                11 => "B-",
                _ => "??",
            };
            format!("{name_str}{octave}")
        }
    }
}

fn format_instrument(ins: u8) -> String {
    if ins == 0 {
        "..".to_string()
    } else {
        format!("{ins:02X}")
    }
}

fn format_effect(cmd: EffectCommand) -> String {
    if cmd.effect == 0 && cmd.operand == 0 {
        "...".to_string()
    } else if cmd.effect >= 0x30 && cmd.effect <= 0x3f {
        let ext_type = cmd.effect - 0x30;
        format!("E{:X}{:X}", ext_type, cmd.operand & 0x0f)
    } else {
        let effect_char = match cmd.effect {
            0x00 => '0',
            0x01 => '1',
            0x02 => '2',
            0x03 => '3',
            0x04 => '4',
            0x05 => '5',
            0x06 => '6',
            0x07 => '7',
            0x08 => '8',
            0x09 => '9',
            0x0a => 'A',
            0x0b => 'B',
            0x0c => 'C',
            0x0d => 'D',
            0x0f => 'F',
            0x20 => '0',
            _ => '?',
        };
        if effect_char == '?' {
            format!("{:02X}{:02X}", cmd.effect, cmd.operand)
        } else {
            format!("{}{:02X}", effect_char, cmd.operand)
        }
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
