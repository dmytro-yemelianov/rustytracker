use crate::app::{ActiveField, RustyTrackerApp};
use eframe::egui;
use egui::Key;
use rustytracker_core::{EffectCommand, Note, NoteName, PatternCell, MAX_INSTRUMENTS};

const FIELD_ENTRY_STATE_ID: &str = "rustytracker.pattern_input.field_entry_state";
const MAX_OCTAVE: u8 = 8;
const INSTRUMENT_FIELD_DIGITS: u8 = 2;
const EFFECT_FIELD_DIGITS: u8 = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FieldEntryState {
    location: Option<FieldEntryLocation>,
    digits_entered: u8,
}

impl FieldEntryState {
    fn begin_digit(&mut self, location: FieldEntryLocation) -> u8 {
        if self.location != Some(location) {
            self.location = Some(location);
            self.digits_entered = 0;
        }
        self.digits_entered
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FieldEntryLocation {
    order_index: usize,
    row: u16,
    channel: u16,
    field: ActiveField,
}

impl RustyTrackerApp {
    pub(crate) fn handle_keyboard_input(&mut self, ctx: &egui::Context) {
        let mut field_entry_state = ctx
            .data(|data| data.get_temp::<FieldEntryState>(field_entry_state_id()))
            .unwrap_or_default();

        ctx.input(|input| {
            // 1. Navigation Keys (always active)
            if input.key_pressed(Key::ArrowDown) {
                if input.modifiers.alt {
                    self.adjust_selected_instrument(1);
                } else {
                    let rows = self.get_active_pattern_rows().max(1);
                    self.active_row = (self.active_row + 1) % rows;
                }
                field_entry_state.reset();
            }
            if input.key_pressed(Key::ArrowUp) {
                if input.modifiers.alt {
                    self.adjust_selected_instrument(-1);
                } else {
                    let rows = self.get_active_pattern_rows().max(1);
                    if self.active_row == 0 {
                        self.active_row = rows - 1;
                    } else {
                        self.active_row -= 1;
                    }
                }
                field_entry_state.reset();
            }
            if input.key_pressed(Key::ArrowRight) {
                self.navigate_field_right();
                field_entry_state.reset();
            }
            if input.key_pressed(Key::ArrowLeft) {
                self.navigate_field_left();
                field_entry_state.reset();
            }
            if input.key_pressed(Key::Tab) {
                if input.modifiers.shift {
                    self.navigate_field_left();
                } else {
                    self.navigate_field_right();
                }
                field_entry_state.reset();
            }

            // Page Up / Down jumping by 16 rows
            if input.key_pressed(Key::PageDown) {
                let rows = self.get_active_pattern_rows().max(1);
                self.active_row = (self.active_row + 16).min(rows - 1);
                field_entry_state.reset();
            }
            if input.key_pressed(Key::PageUp) {
                self.active_row = self.active_row.saturating_sub(16);
                field_entry_state.reset();
            }
            if input.key_pressed(Key::Home) {
                self.active_row = 0;
                field_entry_state.reset();
            }
            if input.key_pressed(Key::End) {
                let rows = self.get_active_pattern_rows().max(1);
                self.active_row = rows - 1;
                field_entry_state.reset();
            }

            // Edit Mode toggle with Space
            if input.key_pressed(Key::Space) {
                self.edit_mode = !self.edit_mode;
                field_entry_state.reset();
            }

            if let Some(octave) = octave_for_function_key(input) {
                self.octave = octave;
                field_entry_state.reset();
            }
            if !input.modifiers.command && !input.modifiers.ctrl {
                if input.key_pressed(Key::Minus) {
                    self.octave = self.octave.saturating_sub(1);
                    field_entry_state.reset();
                }
                if input.key_pressed(Key::Plus) || input.key_pressed(Key::Equals) {
                    self.octave = (self.octave + 1).min(MAX_OCTAVE);
                    field_entry_state.reset();
                }
            }

            // Live sample preview (jam) — runs in both edit and non-edit mode.
            let mut released = false;
            let old_active_key = self.pressed_keys.last().copied();

            self.pressed_keys.retain(|&k| {
                let down = input.key_down(k);
                if !down {
                    released = true;
                }
                down
            });

            let mut newly_pressed_notes = Vec::new();
            for key in NOTE_KEYS {
                if input.key_pressed(key)
                    && !self.pressed_keys.contains(&key)
                    && self.should_preview_note_key(key)
                {
                    newly_pressed_notes.push(key);
                }
            }

            let mut active_note_changed = false;
            if !newly_pressed_notes.is_empty() {
                for &key in &newly_pressed_notes {
                    self.pressed_keys.push(key);
                }
                active_note_changed = true;
            } else if released {
                let new_active_key = self.pressed_keys.last().copied();
                if new_active_key != old_active_key {
                    active_note_changed = true;
                }
            }

            if active_note_changed {
                if let Some(&active_key) = self.pressed_keys.last() {
                    if let Some(value) = self.note_value_for_key(active_key) {
                        self.audio_engine.preview_note_on(
                            self.selected_instrument,
                            value,
                            self.mixer_mode,
                        );
                    }
                } else {
                    self.audio_engine.preview_note_off();
                }
            }

            // Edit operations (requires edit mode)
            if self.edit_mode {
                // Delete cell with Delete / Backspace
                if input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace) {
                    if self.clear_active_field() {
                        self.commit_edit_to_audio();
                    }
                    field_entry_state.reset();
                }

                // Check for Note Off key (Num1)
                if input.key_pressed(Key::Num1) && self.active_field == ActiveField::Note {
                    if self.write_active_note(Note::Off, None) {
                        self.commit_edit_to_audio();
                        self.advance_row_after_edit();
                    }
                    field_entry_state.reset();
                }

                // Check for note input keys
                if self.active_field == ActiveField::Note {
                    for &key in &newly_pressed_notes {
                        if let Some(value) = self.note_value_for_key(key) {
                            let note = Note::Key(value);
                            if self.write_active_note(note, Some(self.selected_instrument)) {
                                self.commit_edit_to_audio();
                                self.advance_row_after_edit();
                            }
                            field_entry_state.reset();
                        }
                    }
                }

                // Check for hex key input (0-9, A-F) on instrument, volume, effect
                let hex_value = get_hex_key_value(input);
                if let Some(digit) = hex_value {
                    if self.apply_active_hex_digit(digit, &mut field_entry_state) {
                        self.commit_edit_to_audio();
                    }
                }
            }
        });

        ctx.data_mut(|data| data.insert_temp(field_entry_state_id(), field_entry_state));
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
        let rows = self.get_active_pattern_rows().max(1);
        self.active_row = (self.active_row + 1) % rows;
    }

    fn write_active_note(&mut self, note: Note, instrument: Option<u8>) -> bool {
        let active_pattern_idx = self.get_active_pattern_index();
        let note_result = self.editor.set_note(
            active_pattern_idx,
            self.active_channel,
            self.active_row,
            note,
        );
        let instrument_result = instrument.map(|instrument| {
            self.editor.set_instrument(
                active_pattern_idx,
                self.active_channel,
                self.active_row,
                instrument,
            )
        });

        note_result.is_ok() && instrument_result.is_none_or(|result| result.is_ok())
    }

    fn clear_active_field(&mut self) -> bool {
        let active_pattern_idx = self.get_active_pattern_index();
        match self.active_field {
            ActiveField::Note => self
                .editor
                .set_note(
                    active_pattern_idx,
                    self.active_channel,
                    self.active_row,
                    Note::Empty,
                )
                .is_ok(),
            ActiveField::Instrument => self
                .editor
                .set_instrument(active_pattern_idx, self.active_channel, self.active_row, 0)
                .is_ok(),
            ActiveField::Effect0 => self
                .editor
                .set_effect(
                    active_pattern_idx,
                    self.active_channel,
                    self.active_row,
                    0,
                    EffectCommand::default(),
                )
                .is_ok(),
            ActiveField::Effect1 => self
                .editor
                .set_effect(
                    active_pattern_idx,
                    self.active_channel,
                    self.active_row,
                    1,
                    EffectCommand::default(),
                )
                .is_ok(),
        }
    }

    fn apply_active_hex_digit(
        &mut self,
        digit: u8,
        field_entry_state: &mut FieldEntryState,
    ) -> bool {
        let digit = digit & 0x0f;
        let active_pattern_idx = self.get_active_pattern_index();
        let Some(cell) = self.active_cell(active_pattern_idx) else {
            field_entry_state.reset();
            return false;
        };
        let location = self.field_entry_location();
        let digits_entered = field_entry_state.begin_digit(location);
        let edit_succeeded = match self.active_field {
            ActiveField::Instrument => {
                let base_instrument = if digits_entered == 0 {
                    0
                } else {
                    cell.instrument
                };
                let new_instrument = append_instrument_digit(base_instrument, digit);
                self.editor
                    .set_instrument(
                        active_pattern_idx,
                        self.active_channel,
                        self.active_row,
                        new_instrument,
                    )
                    .is_ok()
            }
            ActiveField::Effect0 | ActiveField::Effect1 => {
                let slot = self.active_effect_slot();
                let base_effect = if digits_entered == 0 {
                    EffectCommand::default()
                } else {
                    cell.effects
                        .get(usize::from(slot))
                        .copied()
                        .unwrap_or_default()
                };
                let new_effect = crate::effect_entry::append_effect_digit(base_effect, digit);
                self.editor
                    .set_effect(
                        active_pattern_idx,
                        self.active_channel,
                        self.active_row,
                        slot,
                        new_effect,
                    )
                    .is_ok()
            }
            ActiveField::Note => false,
        };

        if edit_succeeded {
            let digits_needed = match self.active_field {
                ActiveField::Instrument => INSTRUMENT_FIELD_DIGITS,
                ActiveField::Effect0 | ActiveField::Effect1 => EFFECT_FIELD_DIGITS,
                ActiveField::Note => return true,
            };
            self.finish_field_digit(field_entry_state, digits_needed);
        } else {
            field_entry_state.reset();
        }

        edit_succeeded
    }

    fn finish_field_digit(&mut self, field_entry_state: &mut FieldEntryState, digits_needed: u8) {
        field_entry_state.digits_entered += 1;
        if field_entry_state.digits_entered >= digits_needed {
            self.navigate_field_right();
            field_entry_state.reset();
        }
    }

    fn active_cell(&self, active_pattern_idx: usize) -> Option<PatternCell> {
        self.editor
            .module()
            .patterns
            .get(active_pattern_idx)?
            .cell(self.active_channel, self.active_row)
            .ok()
            .cloned()
    }

    fn active_effect_slot(&self) -> u8 {
        match self.active_field {
            ActiveField::Effect1 => 1,
            _ => 0,
        }
    }

    fn field_entry_location(&self) -> FieldEntryLocation {
        FieldEntryLocation {
            order_index: self.active_order_index,
            row: self.active_row,
            channel: self.active_channel,
            field: self.active_field,
        }
    }

    fn should_preview_note_key(&self, key: Key) -> bool {
        !(self.edit_mode
            && self.active_field != ActiveField::Note
            && key_to_hex_digit(key).is_some())
    }

    fn adjust_selected_instrument(&mut self, delta: i16) {
        let max_instrument = self.max_selectable_instrument();
        let selected = i16::from(self.selected_instrument);
        self.selected_instrument = selected
            .saturating_add(delta)
            .clamp(1, i16::from(max_instrument)) as u8;
    }

    fn max_selectable_instrument(&self) -> u8 {
        self.editor
            .module()
            .instruments
            .len()
            .min(MAX_INSTRUMENTS)
            .min(usize::from(u8::MAX))
            .max(1) as u8
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

fn field_entry_state_id() -> egui::Id {
    egui::Id::new(FIELD_ENTRY_STATE_ID)
}

fn append_instrument_digit(instrument: u8, digit: u8) -> u8 {
    (instrument << 4) | (digit & 0x0f)
}

fn octave_for_function_key(input: &egui::InputState) -> Option<u8> {
    for (key, octave) in OCTAVE_KEYS {
        if input.key_pressed(key) {
            return Some(octave);
        }
    }
    None
}

fn get_hex_key_value(input: &egui::InputState) -> Option<u8> {
    HEX_KEYS
        .iter()
        .find_map(|&(key, value)| input.key_pressed(key).then_some(value))
}

fn key_to_hex_digit(key: Key) -> Option<u8> {
    HEX_KEYS
        .iter()
        .find_map(|&(hex_key, value)| (hex_key == key).then_some(value))
}

const OCTAVE_KEYS: [(Key, u8); 9] = [
    (Key::F1, 0),
    (Key::F2, 1),
    (Key::F3, 2),
    (Key::F4, 3),
    (Key::F5, 4),
    (Key::F6, 5),
    (Key::F7, 6),
    (Key::F8, 7),
    (Key::F9, 8),
];

const HEX_KEYS: [(Key, u8); 16] = [
    (Key::Num0, 0x00),
    (Key::Num1, 0x01),
    (Key::Num2, 0x02),
    (Key::Num3, 0x03),
    (Key::Num4, 0x04),
    (Key::Num5, 0x05),
    (Key::Num6, 0x06),
    (Key::Num7, 0x07),
    (Key::Num8, 0x08),
    (Key::Num9, 0x09),
    (Key::A, 0x0a),
    (Key::B, 0x0b),
    (Key::C, 0x0c),
    (Key::D, 0x0d),
    (Key::E, 0x0e),
    (Key::F, 0x0f),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> RustyTrackerApp {
        let ctx = egui::Context::default();
        RustyTrackerApp::new(&ctx)
    }

    fn active_cell(app: &RustyTrackerApp) -> PatternCell {
        app.editor.module().patterns[0]
            .cell(app.active_channel, app.active_row)
            .unwrap()
            .clone()
    }

    #[test]
    fn instrument_digit_entry_starts_fresh_and_advances_after_two_digits() {
        let mut app = app();
        let mut entry = FieldEntryState::default();
        app.active_field = ActiveField::Instrument;

        assert!(app.apply_active_hex_digit(0x01, &mut entry));
        assert_eq!(active_cell(&app).instrument, 0x01);
        assert_eq!(app.active_field, ActiveField::Instrument);
        assert_eq!(entry.digits_entered, 1);

        assert!(app.apply_active_hex_digit(0x02, &mut entry));
        assert_eq!(active_cell(&app).instrument, 0x12);
        assert_eq!(app.active_field, ActiveField::Effect0);
        assert_eq!(entry, FieldEntryState::default());
    }

    #[test]
    fn effect_digit_entry_starts_fresh_and_advances_after_three_digits() {
        let mut app = app();
        let mut entry = FieldEntryState::default();
        app.active_field = ActiveField::Effect0;

        for digit in [0x0f, 0x00, 0x06] {
            assert!(app.apply_active_hex_digit(digit, &mut entry));
        }

        assert_eq!(
            active_cell(&app).effects[0],
            EffectCommand {
                effect: 0x0f,
                operand: 0x06,
            }
        );
        assert_eq!(app.active_field, ActiveField::Effect1);
        assert_eq!(entry, FieldEntryState::default());
    }

    #[test]
    fn field_digit_entry_resets_when_cursor_location_changes() {
        let mut app = app();
        let mut entry = FieldEntryState::default();
        app.active_field = ActiveField::Instrument;

        assert!(app.apply_active_hex_digit(0x01, &mut entry));
        app.active_row = 1;
        assert!(app.apply_active_hex_digit(0x02, &mut entry));

        assert_eq!(active_cell(&app).instrument, 0x02);
        assert_eq!(entry.digits_entered, 1);
    }

    #[test]
    fn hex_note_keys_do_not_preview_while_editing_hex_fields() {
        let mut app = app();
        app.edit_mode = true;
        app.active_field = ActiveField::Effect0;

        assert!(!app.should_preview_note_key(Key::C));
        assert!(app.should_preview_note_key(Key::G));

        app.active_field = ActiveField::Note;
        assert!(app.should_preview_note_key(Key::C));
    }

    #[test]
    fn selected_instrument_shortcuts_clamp_to_available_pool() {
        let mut app = app();

        app.selected_instrument = 1;
        app.adjust_selected_instrument(-1);
        assert_eq!(app.selected_instrument, 1);

        app.selected_instrument = 127;
        app.adjust_selected_instrument(1);
        assert_eq!(app.selected_instrument, 128);
        app.adjust_selected_instrument(1);
        assert_eq!(app.selected_instrument, 128);
    }

    #[test]
    fn helper_key_maps_cover_hex_and_octave_shortcuts() {
        assert_eq!(key_to_hex_digit(Key::Num9), Some(0x09));
        assert_eq!(key_to_hex_digit(Key::F), Some(0x0f));
        assert_eq!(key_to_hex_digit(Key::G), None);

        assert_eq!(OCTAVE_KEYS.first(), Some(&(Key::F1, 0)));
        assert_eq!(OCTAVE_KEYS.last(), Some(&(Key::F9, MAX_OCTAVE)));
    }

    #[test]
    fn row_advance_wraps_inside_active_pattern() {
        let mut app = app();
        app.active_row = app.get_active_pattern_rows() - 1;

        app.advance_row_after_edit();

        assert_eq!(app.active_row, 0);
    }
}
