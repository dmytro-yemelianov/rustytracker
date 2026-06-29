use crate::app::{InstrumentEditorEdits, RustyTrackerApp, ViewMode};
use crate::tracker_ui;
use eframe::egui;
use egui::Ui;
use rustytracker_core::{InstrumentName, SampleLoopKind, SampleName};
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};

impl RustyTrackerApp {
    pub(crate) fn render_menu_bar(&mut self, ui: &mut Ui) {
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

    pub(crate) fn render_controls_bar(&mut self, ui: &mut Ui) {
        let theme = self.tracker_resources.theme();

        ui.horizontal(|ui| {
            let is_playing = self.audio_engine.is_playing();

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
                    self.audio_engine.pause();
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
                self.audio_engine.play();
                self.audio_engine.update_module(cloned_module.clone());
                if !is_playing {
                    if let Ok(pb) = PlaybackState::start_with_settings(
                        &cloned_module,
                        PlaybackSettings::with_mixer_mode(self.mixer_mode),
                    ) {
                        self.audio_engine.set_playback(Some(pb));
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
                self.audio_engine.stop();
                self.active_row = 0;
                self.active_order_index = 0;
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);

            tracker_ui::show_status_label(ui, &self.tracker_resources, "MIX", theme.foreground);
            let mut selected_mixer_mode = self.mixer_mode;
            egui::ComboBox::from_id_salt("mixer_mode_combo")
                .selected_text(selected_mixer_mode.label())
                .show_ui(ui, |ui| {
                    for mixer_mode in PlaybackMixerMode::ALL {
                        ui.selectable_value(
                            &mut selected_mixer_mode,
                            mixer_mode,
                            mixer_mode.label(),
                        );
                    }
                });
            if selected_mixer_mode != self.mixer_mode {
                self.mixer_mode = selected_mixer_mode;
                if is_playing {
                    let cloned_module = self.editor.module().clone();
                    self.audio_engine.update_module(cloned_module.clone());
                    if let Ok(pb) = PlaybackState::start_with_settings(
                        &cloned_module,
                        PlaybackSettings::with_mixer_mode(self.mixer_mode),
                    ) {
                        self.audio_engine.set_playback(Some(pb));
                    }
                    self.active_row = 0;
                    self.active_order_index = 0;
                }
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

    pub(crate) fn render_order_list(&mut self, ui: &mut Ui) {
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
                    self.audio_engine.set_playback(None);
                }
            }
        });
    }

    pub(crate) fn render_instrument_list(&mut self, ui: &mut Ui) {
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

    pub(crate) fn render_pattern_editor(&mut self, ui: &mut Ui) {
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

    pub(crate) fn render_instrument_editor(&mut self, ui: &mut Ui) {
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

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    let name_changed = ui.text_edit_singleline(&mut name_str).changed();
                    if name_changed {
                        edits.instrument_name = Some(InstrumentName::new(&name_str));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Fadeout:");
                    let changed = ui
                        .add(egui::Slider::new(&mut volume_fadeout, 0..=65535))
                        .changed();
                    if changed {
                        edits.instrument_volume_fadeout = Some(volume_fadeout);
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Volume Envelope:");
                    let env_active = (volume_envelope.flags & 0x01) != 0;
                    let mut check = env_active;
                    if ui.checkbox(&mut check, "Enabled").changed() {
                        if check {
                            volume_envelope.flags |= 0x01;
                        } else {
                            volume_envelope.flags &= !0x01;
                        }
                        volume_envelope_changed = true;
                    }

                    let env_sustain = (volume_envelope.flags & 0x02) != 0;
                    let mut check = env_sustain;
                    if ui.checkbox(&mut check, "Sustain").changed() {
                        if check {
                            volume_envelope.flags |= 0x02;
                        } else {
                            volume_envelope.flags &= !0x02;
                        }
                        volume_envelope_changed = true;
                    }

                    let env_loop = (volume_envelope.flags & 0x04) != 0;
                    let mut check = env_loop;
                    if ui.checkbox(&mut check, "Loop").changed() {
                        if check {
                            volume_envelope.flags |= 0x04;
                        } else {
                            volume_envelope.flags &= !0x04;
                        }
                        volume_envelope_changed = true;
                    }
                });

                if volume_envelope_changed {
                    edits.volume_envelope = Some(volume_envelope.clone());
                }

                if let Some(sample) = sample {
                    ui.separator();
                    tracker_ui::show_list_heading(ui, &self.tracker_resources, "SAMPLE PARAMETERS");
                    ui.separator();

                    let mut s_name = sample.name.as_str().to_string();
                    ui.horizontal(|ui| {
                        ui.label("Sample Name:");
                        let name_changed = ui.text_edit_singleline(&mut s_name).changed();
                        if name_changed {
                            edits.sample_name = Some(SampleName::new(&s_name));
                        }
                    });

                    let mut s_volume = sample.volume;
                    ui.horizontal(|ui| {
                        ui.label("Default Volume:");
                        let changed = ui.add(egui::Slider::new(&mut s_volume, 0..=64)).changed();
                        if changed {
                            edits.sample_volume = Some(s_volume);
                        }
                    });

                    let mut s_panning = sample.panning;
                    ui.horizontal(|ui| {
                        ui.label("Default Panning:");
                        let changed = ui.add(egui::Slider::new(&mut s_panning, 0..=255)).changed();
                        if changed {
                            edits.sample_panning = Some(s_panning);
                        }
                    });

                    let mut s_finetune = sample.finetune;
                    ui.horizontal(|ui| {
                        ui.label("Finetune:");
                        let changed = ui
                            .add(egui::Slider::new(&mut s_finetune, -128..=127))
                            .changed();
                        if changed {
                            edits.sample_finetune = Some(s_finetune);
                        }
                    });

                    let mut s_rel_note = sample.relative_note;
                    ui.horizontal(|ui| {
                        ui.label("Relative Note:");
                        let changed = ui
                            .add(egui::Slider::new(&mut s_rel_note, -96..=95))
                            .changed();
                        if changed {
                            edits.sample_relative_note = Some(s_rel_note);
                        }
                    });

                    let mut loop_mode = sample.loop_kind;
                    ui.horizontal(|ui| {
                        ui.label("Loop Mode:");
                        let prev_mode = loop_mode;
                        egui::ComboBox::from_id_salt("loop_mode_combo")
                            .selected_text(match loop_mode {
                                SampleLoopKind::None => "No Loop",
                                SampleLoopKind::Forward => "Forward",
                                SampleLoopKind::PingPong => "Ping-Pong",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut loop_mode,
                                    SampleLoopKind::None,
                                    "No Loop",
                                );
                                ui.selectable_value(
                                    &mut loop_mode,
                                    SampleLoopKind::Forward,
                                    "Forward",
                                );
                                ui.selectable_value(
                                    &mut loop_mode,
                                    SampleLoopKind::PingPong,
                                    "Ping-Pong",
                                );
                            });
                        if loop_mode != prev_mode {
                            edits.sample_loop_kind = Some(loop_mode);
                        }
                    });

                    if loop_mode != SampleLoopKind::None {
                        let max_len = sample.length.saturating_sub(1);
                        let mut loop_start = sample.loop_start;
                        let mut loop_length = sample.loop_length;

                        ui.horizontal(|ui| {
                            ui.label("Loop Start:");
                            let changed = ui
                                .add(egui::Slider::new(&mut loop_start, 0..=max_len))
                                .changed();
                            if changed {
                                edits.sample_loop_start = Some(loop_start);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Loop Length:");
                            let changed = ui
                                .add(egui::Slider::new(&mut loop_length, 0..=sample.length))
                                .changed();
                            if changed {
                                edits.sample_loop_length = Some(loop_length);
                            }
                        });
                    }

                    ui.separator();
                    tracker_ui::show_list_heading(ui, &self.tracker_resources, "WAVEFORM PREVIEW");
                    ui.separator();

                    let visible_loop_kind = edits.sample_loop_kind.unwrap_or(sample.loop_kind);
                    let visible_loop_start = edits.sample_loop_start.unwrap_or(sample.loop_start);
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
                }
            });

            (edits, sample_idx)
        };

        if edits.has_changes() {
            let _ = self.editor.edit_instrument_and_sample_with_undo(
                ins_idx,
                sample_idx,
                |instrument, sample| {
                    edits.apply(instrument, sample);
                },
            );
            self.commit_edit_to_audio();
        }
    }
}
