use crate::app::{InstrumentEditorEdits, RustyTrackerApp, ViewMode};
use crate::io;
use crate::tracker_ui;
use eframe::egui;
use egui::Ui;
use rustytracker_core::{
    Envelope, EnvelopePoint, InstrumentName, Note, NoteName, Sample, SampleData, SampleLoopKind,
    SampleName,
};
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};

const SAMPLE_VOLUME_MAX: u8 = 64;
const SAMPLE_VOLUME_CORE_MAX: u16 = 255;
const SAMPLE_VOLUME_TO_CORE_SCALE: u32 = 261_120;
const SAMPLE_VOLUME_TO_CORE_ROUNDING: u32 = 65_535;
const SAMPLE_VOLUME_TO_CORE_SHIFT: u32 = 16;
const ENVELOPE_MAX_POINTS: usize = 12;
const ENVELOPE_FLAG_ENABLED: u8 = 0x01;
const ENVELOPE_FLAG_SUSTAIN: u8 = 0x02;
const ENVELOPE_FLAG_LOOP: u8 = 0x04;
const ENVELOPE_VALUE_MAX: u16 = 64;
const ENVELOPE_VALUE_SHIFT: u16 = 2;
const ENVELOPE_VALUE_FULL_SCALE: u16 = ENVELOPE_VALUE_MAX << ENVELOPE_VALUE_SHIFT;
const ENVELOPE_VALUE_CENTER: u16 = 32 << ENVELOPE_VALUE_SHIFT;
const TEXT_EDIT_NAME_WIDTH: f32 = 220.0;
const COMPACT_SLIDER_WIDTH: f32 = 140.0;

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
        let playback_status = self.audio_engine.playback_status();
        let is_playing = playback_status.transport.is_playing();

        ui.horizontal(|ui| {
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
                self.audio_engine.preview_note_off();
                self.active_row = 0;
                self.active_order_index = 0;
            }

            if self.audio_engine.device_error() {
                tracker_ui::show_status_label(
                    ui,
                    &self.tracker_resources,
                    "AUDIO ERROR",
                    theme.pattern_effect,
                );
            }

            tracker_ui::show_toolbar_separator(ui, &self.tracker_resources);
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                transport_label(playback_status.transport),
                transport_status_color(playback_status.transport, &theme),
            );
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!("POS {:02}/{:03}", playback_status.order_index, playback_status.row),
                theme.foreground,
            );
            tracker_ui::show_status_label(
                ui,
                &self.tracker_resources,
                &format!(
                    "L{:03}% R{:03}%",
                    peak_to_percent(playback_status.meters.master_left_peak),
                    peak_to_percent(playback_status.meters.master_right_peak),
                ),
                theme.pattern_note,
            );

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
                self.audio_engine.preview_note_off();
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
                self.audio_engine.preview_note_off();
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
            match self.last_file_operation_status() {
                Some(status) => {
                    let (status_text, status_color) =
                        file_operation_status_text_and_color(status, &theme);
                    tracker_ui::show_status_label(ui, &self.tracker_resources, &status_text, status_color);
                }
                None => {
                    tracker_ui::show_status_label(
                        ui,
                        &self.tracker_resources,
                        "READY",
                        theme.pattern_instrument,
                    );
                }
            }
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
                    if let Ok(Note::Key(value)) = Note::key(4, NoteName::C) {
                        self.audio_engine
                            .preview_note_on(ins_num, value, self.mixer_mode);
                    }
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
            let mut panning_envelope = instrument.panning_envelope.clone();
            normalize_envelope_for_editor(&mut volume_envelope, ENVELOPE_VALUE_FULL_SCALE);
            normalize_envelope_for_editor(&mut panning_envelope, ENVELOPE_VALUE_CENTER);

            ui.vertical(|ui| {
                tracker_ui::show_list_heading(
                    ui,
                    &self.tracker_resources,
                    "INSTRUMENT & SAMPLE EDITOR",
                );
                ui.separator();

                render_instrument_controls(
                    ui,
                    self.selected_instrument,
                    &mut name_str,
                    &mut volume_fadeout,
                    &mut edits,
                );

                ui.separator();
                tracker_ui::show_list_heading(ui, &self.tracker_resources, "VOLUME ENVELOPE");
                if render_envelope_editor(
                    ui,
                    &mut volume_envelope,
                    ENVELOPE_VALUE_FULL_SCALE,
                    "Vol",
                ) {
                    edits.volume_envelope = Some(volume_envelope.clone());
                }

                ui.separator();
                tracker_ui::show_list_heading(ui, &self.tracker_resources, "PANNING ENVELOPE");
                if render_envelope_editor(ui, &mut panning_envelope, ENVELOPE_VALUE_CENTER, "Pan") {
                    edits.panning_envelope = Some(panning_envelope.clone());
                }

                if let Some(sample) = sample {
                    ui.separator();
                    tracker_ui::show_list_heading(ui, &self.tracker_resources, "SAMPLE PARAMETERS");
                    ui.separator();

                    render_sample_controls(ui, sample_idx, sample, &mut edits);

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

fn render_instrument_controls(
    ui: &mut Ui,
    selected_instrument: u8,
    name: &mut String,
    volume_fadeout: &mut u16,
    edits: &mut InstrumentEditorEdits,
) {
    egui::Grid::new("instrument_editor_instrument_grid")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Ins");
            ui.monospace(format!("{selected_instrument:02X}"));
            ui.label("Name");
            if ui
                .add(egui::TextEdit::singleline(name).desired_width(TEXT_EDIT_NAME_WIDTH))
                .changed()
            {
                edits.instrument_name = Some(InstrumentName::new(name));
            }
            ui.end_row();

            ui.label("Fade");
            let drag_changed = ui
                .add(
                    egui::DragValue::new(volume_fadeout)
                        .range(0..=u16::MAX)
                        .speed(32.0),
                )
                .changed();
            let slider_changed = ui
                .add_sized(
                    [COMPACT_SLIDER_WIDTH, 18.0],
                    egui::Slider::new(volume_fadeout, 0..=u16::MAX).show_value(false),
                )
                .changed();
            if drag_changed || slider_changed {
                edits.instrument_volume_fadeout = Some(*volume_fadeout);
            }
            ui.end_row();
        });
}

fn render_sample_controls(
    ui: &mut Ui,
    sample_idx: Option<usize>,
    sample: &Sample,
    edits: &mut InstrumentEditorEdits,
) {
    let mut sample_name = sample.name.as_str().to_string();
    let sample_number = sample_idx.map(|index| format!("{:02X}", index + 1));

    ui.horizontal_wrapped(|ui| {
        ui.label("Smp");
        ui.monospace(sample_number.as_deref().unwrap_or("--"));
        ui.label("Name");
        if ui
            .add(egui::TextEdit::singleline(&mut sample_name).desired_width(TEXT_EDIT_NAME_WIDTH))
            .changed()
        {
            edits.sample_name = Some(SampleName::new(&sample_name));
        }
        ui.separator();
        ui.label("Len");
        ui.monospace(sample.length.to_string());
        ui.label(sample_data_label(&sample.data));
    });

    let mut volume_64 = core_sample_volume_to_tracker(sample.volume);
    let mut panning = sample.panning;
    let mut finetune = sample.finetune;
    let mut relative_note = sample.relative_note;

    egui::Grid::new("instrument_editor_sample_value_grid")
        .num_columns(6)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Vol");
            let volume_drag_changed = ui
                .add(egui::DragValue::new(&mut volume_64).range(0..=SAMPLE_VOLUME_MAX))
                .changed();
            let volume_slider_changed = ui
                .add_sized(
                    [COMPACT_SLIDER_WIDTH, 18.0],
                    egui::Slider::new(&mut volume_64, 0..=SAMPLE_VOLUME_MAX).show_value(false),
                )
                .changed();
            if volume_drag_changed || volume_slider_changed {
                edits.sample_volume = Some(tracker_sample_volume_to_core(volume_64));
            }

            ui.label("Pan");
            let pan_drag_changed = ui
                .add(egui::DragValue::new(&mut panning).range(0..=u8::MAX))
                .changed();
            let pan_slider_changed = ui
                .add_sized(
                    [COMPACT_SLIDER_WIDTH, 18.0],
                    egui::Slider::new(&mut panning, 0..=u8::MAX).show_value(false),
                )
                .changed();
            if pan_drag_changed || pan_slider_changed {
                edits.sample_panning = Some(panning);
            }
            ui.end_row();

            ui.label("Fine");
            if ui
                .add(egui::DragValue::new(&mut finetune).range(i8::MIN..=i8::MAX))
                .changed()
            {
                edits.sample_finetune = Some(finetune);
            }
            ui.label("Rel");
            if ui
                .add(egui::DragValue::new(&mut relative_note).range(-96..=95))
                .changed()
            {
                edits.sample_relative_note = Some(relative_note);
            }
            ui.label("Type");
            ui.monospace(format!("{:02X}", sample.sample_type));
            ui.end_row();
        });

    render_sample_loop_controls(ui, sample, edits);
}

fn render_sample_loop_controls(ui: &mut Ui, sample: &Sample, edits: &mut InstrumentEditorEdits) {
    let mut loop_kind = sample.loop_kind;
    let mut loop_start = sample.loop_start.min(sample.length.saturating_sub(1));
    let mut loop_length = sample
        .loop_length
        .min(sample.length.saturating_sub(loop_start));

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.label("Loop");
        let previous_loop_kind = loop_kind;
        egui::ComboBox::from_id_salt("instrument_editor_loop_kind")
            .selected_text(sample_loop_kind_label(loop_kind))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut loop_kind, SampleLoopKind::None, "Off");
                ui.selectable_value(&mut loop_kind, SampleLoopKind::Forward, "Forward");
                ui.selectable_value(&mut loop_kind, SampleLoopKind::PingPong, "Ping-Pong");
            });

        if loop_kind != previous_loop_kind {
            edits.sample_loop_kind = Some(loop_kind);
            if previous_loop_kind == SampleLoopKind::None
                && loop_kind != SampleLoopKind::None
                && sample.length > 0
                && loop_length == 0
            {
                loop_start = loop_start.min(sample.length.saturating_sub(1));
                loop_length = sample.length.saturating_sub(loop_start);
                edits.sample_loop_start = Some(loop_start);
                edits.sample_loop_length = Some(loop_length);
            }
        }

        let loop_controls_enabled = loop_kind != SampleLoopKind::None && sample.length > 0;
        let max_loop_start = sample.length.saturating_sub(1);

        ui.label("Start");
        if ui
            .add_enabled(
                loop_controls_enabled,
                egui::DragValue::new(&mut loop_start).range(0..=max_loop_start),
            )
            .changed()
        {
            let max_loop_length = sample.length.saturating_sub(loop_start);
            if loop_length > max_loop_length {
                loop_length = max_loop_length;
                edits.sample_loop_length = Some(loop_length);
            }
            edits.sample_loop_start = Some(loop_start);
        }

        ui.label("Len");
        let max_loop_length = sample.length.saturating_sub(loop_start);
        if ui
            .add_enabled(
                loop_controls_enabled,
                egui::DragValue::new(&mut loop_length).range(0..=max_loop_length),
            )
            .changed()
        {
            edits.sample_loop_length = Some(loop_length);
        }

        ui.label("End");
        ui.monospace(
            loop_start
                .saturating_add(loop_length)
                .min(sample.length)
                .to_string(),
        );
    });
}

fn render_envelope_editor(
    ui: &mut Ui,
    envelope: &mut Envelope,
    default_value: u16,
    value_label: &'static str,
) -> bool {
    normalize_envelope_for_editor(envelope, default_value);
    let mut changed = false;

    ui.horizontal_wrapped(|ui| {
        changed |= render_envelope_flag(ui, envelope, ENVELOPE_FLAG_ENABLED, "Enabled");
        changed |= render_envelope_flag(ui, envelope, ENVELOPE_FLAG_SUSTAIN, "Sustain");
        changed |= render_envelope_flag(ui, envelope, ENVELOPE_FLAG_LOOP, "Loop");

        ui.separator();

        let mut point_count = active_envelope_point_count(envelope) as u8;
        ui.label("Pts");
        if ui
            .add(egui::DragValue::new(&mut point_count).range(0..=ENVELOPE_MAX_POINTS as u8))
            .changed()
        {
            ensure_envelope_point_count(envelope, point_count as usize, default_value);
            changed = true;
        }
    });

    let point_count = active_envelope_point_count(envelope);
    let has_points = point_count > 0;
    let max_point_index = point_count.saturating_sub(1) as u8;

    ui.horizontal_wrapped(|ui| {
        ui.label("Sus");
        let mut sustain_point = envelope.sustain_point.min(max_point_index);
        if ui
            .add_enabled(
                has_points,
                egui::DragValue::new(&mut sustain_point).range(0..=max_point_index),
            )
            .changed()
        {
            envelope.sustain_point = sustain_point;
            changed = true;
        }

        ui.label("Loop");
        let mut loop_start_point = envelope.loop_start_point.min(max_point_index);
        if ui
            .add_enabled(
                has_points,
                egui::DragValue::new(&mut loop_start_point).range(0..=max_point_index),
            )
            .changed()
        {
            envelope.loop_start_point = loop_start_point;
            if envelope.loop_end_point < loop_start_point {
                envelope.loop_end_point = loop_start_point;
            }
            changed = true;
        }

        ui.label("to");
        let mut loop_end_point = envelope.loop_end_point.min(max_point_index);
        if ui
            .add_enabled(
                has_points,
                egui::DragValue::new(&mut loop_end_point).range(loop_start_point..=max_point_index),
            )
            .changed()
        {
            envelope.loop_end_point = loop_end_point;
            changed = true;
        }
    });

    render_envelope_point_grid(ui, envelope, value_label, &mut changed);
    render_envelope_preview(ui, envelope);

    if changed {
        clamp_envelope_point_indexes(envelope);
    }
    changed
}

fn render_envelope_flag(
    ui: &mut Ui,
    envelope: &mut Envelope,
    flag: u8,
    label: &'static str,
) -> bool {
    let mut enabled = (envelope.flags & flag) != 0;
    if ui.checkbox(&mut enabled, label).changed() {
        if enabled {
            envelope.flags |= flag;
        } else {
            envelope.flags &= !flag;
        }
        return true;
    }
    false
}

fn render_envelope_point_grid(
    ui: &mut Ui,
    envelope: &mut Envelope,
    value_label: &'static str,
    changed: &mut bool,
) {
    let point_count = active_envelope_point_count(envelope);
    if point_count == 0 {
        return;
    }

    egui::Grid::new("instrument_editor_volume_envelope_points")
        .num_columns(3)
        .spacing([8.0, 3.0])
        .show(ui, |ui| {
            ui.label("#");
            ui.label("Frame");
            ui.label(value_label);
            ui.end_row();

            for point_index in 0..point_count {
                ui.monospace(format!("{point_index:02}"));

                let previous_frame = point_index
                    .checked_sub(1)
                    .and_then(|index| envelope.points.get(index))
                    .map(|point| point.frame)
                    .unwrap_or(0);
                let next_frame = envelope
                    .points
                    .get(point_index + 1)
                    .map(|point| point.frame)
                    .unwrap_or(u16::MAX);
                let min_frame = previous_frame.min(next_frame);
                let max_frame = previous_frame.max(next_frame);

                let mut frame = envelope.points[point_index].frame;
                let mut value = envelope_value_to_tracker(envelope.points[point_index].value);

                if ui
                    .add(
                        egui::DragValue::new(&mut frame)
                            .range(min_frame..=max_frame)
                            .speed(1.0),
                    )
                    .changed()
                {
                    envelope.points[point_index].frame = frame;
                    *changed = true;
                }

                if ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(0..=ENVELOPE_VALUE_MAX)
                            .speed(1.0),
                    )
                    .changed()
                {
                    envelope.points[point_index].value = tracker_envelope_value_to_core(value);
                    *changed = true;
                }
                ui.end_row();
            }
        });
}

fn render_envelope_preview(ui: &mut Ui, envelope: &Envelope) {
    let desired_size = egui::vec2(ui.available_width().min(420.0), 72.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 0.0, visuals.extreme_bg_color);
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let point_count = active_envelope_point_count(envelope);
    if point_count == 0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "NO ENVELOPE POINTS",
            egui::TextStyle::Monospace.resolve(ui.style()),
            visuals.text_color(),
        );
        return;
    }

    let active_points = &envelope.points[..point_count.min(envelope.points.len())];
    let max_frame = active_points
        .iter()
        .map(|point| point.frame)
        .max()
        .unwrap_or(1)
        .max(1);
    let to_screen = |point: EnvelopePoint| {
        let x = rect.left() + rect.width() * (point.frame as f32 / max_frame as f32);
        let value =
            point.value.min(ENVELOPE_VALUE_FULL_SCALE) as f32 / ENVELOPE_VALUE_FULL_SCALE as f32;
        let y = rect.bottom() - rect.height() * value;
        egui::pos2(x, y)
    };

    let stroke = egui::Stroke::new(1.5, visuals.selection.stroke.color);
    for points in active_points.windows(2) {
        painter.line_segment([to_screen(points[0]), to_screen(points[1])], stroke);
    }
    for (index, point) in active_points.iter().copied().enumerate() {
        let center = to_screen(point);
        let fill = if index as u8 == envelope.sustain_point {
            visuals.selection.bg_fill
        } else {
            visuals.widgets.active.bg_fill
        };
        painter.circle_filled(center, 3.0, fill);
    }
}

fn transport_label(transport: crate::playback::PlaybackTransportState) -> &'static str {
    match transport {
        crate::playback::PlaybackTransportState::Stopped => "STOP",
        crate::playback::PlaybackTransportState::Playing => "PLAY",
        crate::playback::PlaybackTransportState::Paused => "PAUSE",
    }
}

fn transport_status_color(
    transport: crate::playback::PlaybackTransportState,
    theme: &tracker_ui::TrackerTheme,
) -> egui::Color32 {
    match transport {
        crate::playback::PlaybackTransportState::Stopped => theme.foreground,
        crate::playback::PlaybackTransportState::Playing => theme.pattern_note,
        crate::playback::PlaybackTransportState::Paused => theme.pattern_instrument,
    }
}

fn peak_to_percent(peak: f32) -> u16 {
    (peak.clamp(0.0, 1.0) * 100.0).round() as u16
}

fn compact_path_name(path: &std::path::Path) -> String {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => path.to_string_lossy().into_owned(),
    }
}

fn trim_to_chars(text: &str, max_chars: usize) -> String {
    let trimmed: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        format!("{trimmed}…")
    } else {
        trimmed
    }
}

fn file_operation_status_text_and_color(
    status: &io::FileOperationStatus,
    theme: &tracker_ui::TrackerTheme,
) -> (String, egui::Color32) {
    let operation = match status.operation {
        io::FileOperation::LoadModule => "LOAD",
        io::FileOperation::SaveModule => "SAVE",
        io::FileOperation::ExportWav => "WAV",
    };

    if status.is_failure() {
        let text = format!(
            "ERR {} {} {}",
            operation,
            compact_path_name(&status.path),
            trim_to_chars(&status.message, 28),
        );
        return (text, theme.pattern_effect);
    }

    if status.details.is_empty() {
        let text = format!("OK {} {}", operation, compact_path_name(&status.path));
        (text, theme.pattern_note)
    } else {
        let text = format!(
            "OK {} {} (+{}warn)",
            operation,
            compact_path_name(&status.path),
            status.details.len(),
        );
        (text, theme.pattern_instrument)
    }
}


fn normalize_envelope_for_editor(envelope: &mut Envelope, default_value: u16) {
    let point_count = active_envelope_point_count(envelope);
    ensure_envelope_point_count(envelope, point_count, default_value);
}

fn ensure_envelope_point_count(envelope: &mut Envelope, point_count: usize, default_value: u16) {
    let point_count = point_count.min(ENVELOPE_MAX_POINTS);
    while envelope.points.len() < point_count {
        envelope
            .points
            .push(next_envelope_point(&envelope.points, default_value));
    }
    envelope.point_count = point_count as u8;
    clamp_envelope_point_indexes(envelope);
}

fn next_envelope_point(points: &[EnvelopePoint], default_value: u16) -> EnvelopePoint {
    match points.last().copied() {
        Some(point) => EnvelopePoint {
            frame: point.frame.saturating_add(1),
            value: point.value,
        },
        None => EnvelopePoint {
            frame: 0,
            value: default_value,
        },
    }
}

fn clamp_envelope_point_indexes(envelope: &mut Envelope) {
    let point_count = active_envelope_point_count(envelope);
    envelope.point_count = point_count as u8;

    if point_count == 0 {
        envelope.sustain_point = 0;
        envelope.loop_start_point = 0;
        envelope.loop_end_point = 0;
        return;
    }

    let max_point_index = point_count.saturating_sub(1) as u8;
    envelope.sustain_point = envelope.sustain_point.min(max_point_index);
    envelope.loop_start_point = envelope.loop_start_point.min(max_point_index);
    envelope.loop_end_point = envelope
        .loop_end_point
        .min(max_point_index)
        .max(envelope.loop_start_point);
}

fn active_envelope_point_count(envelope: &Envelope) -> usize {
    usize::from(envelope.point_count).min(ENVELOPE_MAX_POINTS)
}

fn envelope_value_to_tracker(value: u16) -> u16 {
    (value >> ENVELOPE_VALUE_SHIFT).min(ENVELOPE_VALUE_MAX)
}

fn tracker_envelope_value_to_core(value: u16) -> u16 {
    value.min(ENVELOPE_VALUE_MAX) << ENVELOPE_VALUE_SHIFT
}

fn core_sample_volume_to_tracker(volume: u8) -> u8 {
    ((u16::from(volume) * u16::from(SAMPLE_VOLUME_MAX)) / SAMPLE_VOLUME_CORE_MAX) as u8
}

fn tracker_sample_volume_to_core(volume: u8) -> u8 {
    (((u32::from(volume.min(SAMPLE_VOLUME_MAX)) * SAMPLE_VOLUME_TO_CORE_SCALE
        + SAMPLE_VOLUME_TO_CORE_ROUNDING)
        >> SAMPLE_VOLUME_TO_CORE_SHIFT)
        & u32::from(u8::MAX)) as u8
}

fn sample_loop_kind_label(loop_kind: SampleLoopKind) -> &'static str {
    match loop_kind {
        SampleLoopKind::None => "Off",
        SampleLoopKind::Forward => "Forward",
        SampleLoopKind::PingPong => "Ping-Pong",
    }
}

fn sample_data_label(data: &SampleData) -> &'static str {
    match data {
        SampleData::Empty => "Empty",
        SampleData::Pcm8(_) => "8-bit",
        SampleData::Pcm16(_) => "16-bit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_volume_display_uses_tracker_scale() {
        assert_eq!(core_sample_volume_to_tracker(0), 0);
        assert_eq!(core_sample_volume_to_tracker(255), 64);
        assert_eq!(tracker_sample_volume_to_core(0), 0);
        assert_eq!(tracker_sample_volume_to_core(64), 255);
    }

    #[test]
    fn envelope_point_count_growth_uses_requested_default_value() {
        let mut envelope = Envelope::default();

        ensure_envelope_point_count(&mut envelope, 2, ENVELOPE_VALUE_CENTER);

        assert_eq!(envelope.point_count, 2);
        assert_eq!(
            envelope.points,
            vec![
                EnvelopePoint {
                    frame: 0,
                    value: ENVELOPE_VALUE_CENTER,
                },
                EnvelopePoint {
                    frame: 1,
                    value: ENVELOPE_VALUE_CENTER,
                },
            ]
        );
    }

    #[test]
    fn envelope_indexes_clamp_to_active_points_without_dropping_hidden_points() {
        let mut envelope = Envelope {
            points: vec![
                EnvelopePoint {
                    frame: 0,
                    value: ENVELOPE_VALUE_FULL_SCALE,
                },
                EnvelopePoint {
                    frame: 8,
                    value: ENVELOPE_VALUE_FULL_SCALE / 2,
                },
                EnvelopePoint {
                    frame: 16,
                    value: 0,
                },
            ],
            point_count: 2,
            sustain_point: 9,
            loop_start_point: 9,
            loop_end_point: 1,
            flags: ENVELOPE_FLAG_ENABLED,
        };

        clamp_envelope_point_indexes(&mut envelope);

        assert_eq!(envelope.point_count, 2);
        assert_eq!(envelope.sustain_point, 1);
        assert_eq!(envelope.loop_start_point, 1);
        assert_eq!(envelope.loop_end_point, 1);
        assert_eq!(envelope.points.len(), 3);
    }
}
