use crate::app::{ActiveField, RustyTrackerApp};
use eframe::egui;
use egui::Key;
use rustytracker_core::{EffectCommand, Note, NoteName};

impl RustyTrackerApp {
    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
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

            // Live sample preview (jam) — runs in both edit and non-edit mode.
            for key in NOTE_KEYS {
                if input.key_pressed(key) {
                    if let Some(value) = self.note_value_for_key(key) {
                        self.audio_engine
                            .preview_note_on(self.selected_instrument, value, self.mixer_mode);
                        self.preview_key = Some(key);
                    }
                }
            }
            if let Some(active) = self.preview_key {
                if input.key_released(active) {
                    self.audio_engine.preview_note_off();
                    self.preview_key = None;
                }
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
                    for key in NOTE_KEYS {
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
                            cmd = crate::effect_entry::append_effect_digit(cmd, digit);
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
                            cmd = crate::effect_entry::append_effect_digit(cmd, digit);
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

    pub(crate) fn get_active_pattern_index(&self) -> usize {
        let module = self.editor.module();
        match module.orders.get(self.active_order_index) {
            Some(&idx) => idx as usize,
            None => 0,
        }
    }

    pub(crate) fn get_active_pattern_rows(&self) -> u16 {
        let active_pat_idx = self.get_active_pattern_index();
        match self.editor.module().patterns.get(active_pat_idx) {
            Some(p) => p.rows(),
            None => 64,
        }
    }

    pub(crate) fn note_value_for_key(&self, key: Key) -> Option<u8> {
        let (name, octave_offset) = key_to_note_and_octave_offset(key)?;
        let final_octave = (self.octave as i8 + octave_offset).clamp(0, 8) as u8;
        match Note::key(final_octave, name) {
            Ok(Note::Key(value)) => Some(value),
            _ => None,
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

const NOTE_KEYS: [Key; 29] = [
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
];

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
