//! Core tracker editing operations and undo/redo command history.
//!
//! Exposes a command-driven and snapshot-based transaction model for pattern edits,
//! order modifications, transpositions, and selections.

use rustytracker_core::{
    CoreError, CoreResult, EffectCommand, Module, Note, PatternCell, MAX_ACTIVE_ORDERS,
    MAX_XM_NOTES,
};

pub const DEFAULT_UNDO_LIMIT: usize = 64;

/// Selection boundary defining a rectangular area in a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start_channel: u16,
    pub end_channel: u16,
    pub start_row: u16,
    pub end_row: u16,
}

impl Selection {
    /// Checks if a channel and row fall inside this selection (inclusive bounds).
    pub fn contains(&self, channel: u16, row: u16) -> bool {
        channel >= self.start_channel
            && channel <= self.end_channel
            && row >= self.start_row
            && row <= self.end_row
    }
}

/// Undo/Redo stack for snapshot-based state restoration.
#[derive(Debug, Clone)]
pub struct UndoHistory {
    undo_stack: Vec<Module>,
    redo_stack: Vec<Module>,
    limit: usize,
}

impl UndoHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            limit,
        }
    }

    pub fn save_state(&mut self, state: &Module) {
        self.undo_stack.push(state.clone());
        if self.undo_stack.len() > self.limit {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut Module) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current.clone());
            if self.redo_stack.len() > self.limit {
                self.redo_stack.remove(0);
            }
            *current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut Module) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current.clone());
            if self.undo_stack.len() > self.limit {
                self.undo_stack.remove(0);
            }
            *current = next;
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

/// Facade for safe, transaction-backed mutations of a tracker Module.
#[derive(Debug, Clone)]
pub struct ModuleEditor {
    module: Module,
    history: UndoHistory,
}

impl ModuleEditor {
    pub fn new(module: Module) -> Self {
        Self {
            module,
            history: UndoHistory::new(DEFAULT_UNDO_LIMIT),
        }
    }

    pub fn module(&self) -> &Module {
        &self.module
    }

    pub fn module_mut(&mut self) -> &mut Module {
        &mut self.module
    }

    /// Replace the module as a single undoable transaction when a caller edits a full snapshot.
    pub fn replace_module_with_undo(&mut self, module: Module) {
        if self.module != module {
            self.begin_transaction();
            self.module = module;
        }
    }

    pub fn into_module(self) -> Module {
        self.module
    }

    /// Commit the current module state to the undo history stack before making a change.
    pub fn begin_transaction(&mut self) {
        self.history.save_state(&self.module);
    }

    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.module)
    }

    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.module)
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // --- Note & Cell Editing ---

    pub fn set_note(
        &mut self,
        pattern_idx: usize,
        channel: u16,
        row: u16,
        note: Note,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        cell.note = note;
        pattern.set_cell(channel, row, cell)
    }

    pub fn set_instrument(
        &mut self,
        pattern_idx: usize,
        channel: u16,
        row: u16,
        instrument: u8,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        cell.instrument = instrument;
        pattern.set_cell(channel, row, cell)
    }

    pub fn set_effect(
        &mut self,
        pattern_idx: usize,
        channel: u16,
        row: u16,
        slot: u8,
        command: EffectCommand,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        if usize::from(slot) >= cell.effects.len() {
            return Err(CoreError::InvalidEffectSlot {
                slot,
                slots: cell.effects.len() as u8,
            });
        }
        cell.effects[usize::from(slot)] = command;
        pattern.set_cell(channel, row, cell)
    }

    pub fn clear_cell(&mut self, pattern_idx: usize, channel: u16, row: u16) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let clean_cell = PatternCell {
            note: Note::Empty,
            instrument: 0,
            effects: vec![EffectCommand::default(); usize::from(pattern.effect_slots())],
        };
        pattern.set_cell(channel, row, clean_cell)
    }

    // --- Order List Editing ---

    pub fn insert_duplicate_order(&mut self, index: usize) -> CoreResult<()> {
        self.begin_transaction();
        if self.module.orders.len() >= MAX_ACTIVE_ORDERS {
            return Err(CoreError::TooManyOrders {
                requested: self.module.orders.len() + 1,
                maximum: MAX_ACTIVE_ORDERS,
            });
        }
        let pattern = *self
            .module
            .orders
            .get(index)
            .ok_or(CoreError::InvalidOrderIndex {
                index,
                len: self.module.orders.len(),
            })?;
        self.module.orders.insert(index + 1, pattern);
        Ok(())
    }

    pub fn delete_order(&mut self, index: usize) -> CoreResult<()> {
        self.begin_transaction();
        if index >= self.module.orders.len() {
            return Err(CoreError::InvalidOrderIndex {
                index,
                len: self.module.orders.len(),
            });
        }
        if self.module.orders.len() > 1 {
            self.module.orders.remove(index);
        } else {
            self.module.orders[0] = 0;
        }
        Ok(())
    }

    pub fn set_order_pattern(&mut self, index: usize, pattern_idx: u8) -> CoreResult<()> {
        self.begin_transaction();
        let len = self.module.orders.len();
        let val = self
            .module
            .orders
            .get_mut(index)
            .ok_or(CoreError::InvalidOrderIndex { index, len })?;
        *val = pattern_idx;
        Ok(())
    }

    pub fn move_order(&mut self, from_idx: usize, to_idx: usize) -> CoreResult<()> {
        self.begin_transaction();
        let len = self.module.orders.len();
        if from_idx >= len || to_idx >= len {
            return Err(CoreError::InvalidOrderIndex {
                index: from_idx.max(to_idx),
                len,
            });
        }
        let item = self.module.orders.remove(from_idx);
        self.module.orders.insert(to_idx, item);
        Ok(())
    }

    // --- Selection Transformations ---

    /// Transpose all notes in the selected area of a pattern by a number of semitones.
    pub fn transpose_selection(
        &mut self,
        pattern_idx: usize,
        selection: Selection,
        semitones: i8,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    if let Note::Key(raw_val) = cell.note {
                        let new_note = transpose_raw_note(raw_val, semitones);
                        let mut updated_cell = cell.clone();
                        updated_cell.note = Note::Key(new_note);
                        pattern.set_cell(channel, row, updated_cell)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Remap all notes referencing one instrument to another instrument within the selection.
    pub fn remap_instrument_selection(
        &mut self,
        pattern_idx: usize,
        selection: Selection,
        from_ins: u8,
        to_ins: u8,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    if cell.instrument == from_ins {
                        let mut updated_cell = cell.clone();
                        updated_cell.instrument = to_ins;
                        pattern.set_cell(channel, row, updated_cell)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Clear notes, instruments, or effects inside the selection area.
    pub fn clear_selection(
        &mut self,
        pattern_idx: usize,
        selection: Selection,
        clear_notes: bool,
        clear_instruments: bool,
        clear_effects: bool,
    ) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    let mut updated_cell = cell.clone();
                    if clear_notes {
                        updated_cell.note = Note::Empty;
                    }
                    if clear_instruments {
                        updated_cell.instrument = 0;
                    }
                    if clear_effects {
                        for eff in &mut updated_cell.effects {
                            *eff = EffectCommand::default();
                        }
                    }
                    pattern.set_cell(channel, row, updated_cell)?;
                }
            }
        }
        Ok(())
    }

    // --- Pattern Manipulation Tools ---

    /// Inserts a blank row at the target index in the pattern, shifting rows down and discarding the last row.
    pub fn insert_row(&mut self, pattern_idx: usize, row_idx: u16) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let rows = pattern.rows();
        let chs = pattern.channels();
        let slots = pattern.effect_slots();

        if row_idx >= rows {
            return Err(CoreError::InvalidRow { row: row_idx, rows });
        }

        // We shift cells down starting from the end
        for row in (row_idx + 1..rows).rev() {
            for ch in 0..chs {
                let cell_above = pattern.cell(ch, row - 1)?.clone();
                pattern.set_cell(ch, row, cell_above)?;
            }
        }

        // Insert clean empty cell at target row_idx
        for ch in 0..chs {
            let empty_cell = PatternCell {
                note: Note::Empty,
                instrument: 0,
                effects: vec![EffectCommand::default(); usize::from(slots)],
            };
            pattern.set_cell(ch, row_idx, empty_cell)?;
        }

        Ok(())
    }

    /// Deletes the row at the target index, shifting subsequent rows up and filling the last row with empty cells.
    pub fn delete_row(&mut self, pattern_idx: usize, row_idx: u16) -> CoreResult<()> {
        self.begin_transaction();
        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let rows = pattern.rows();
        let chs = pattern.channels();
        let slots = pattern.effect_slots();

        if row_idx >= rows {
            return Err(CoreError::InvalidRow { row: row_idx, rows });
        }

        // Shift up
        for row in row_idx..rows - 1 {
            for ch in 0..chs {
                let cell_below = pattern.cell(ch, row + 1)?.clone();
                pattern.set_cell(ch, row, cell_below)?;
            }
        }

        // Clear the last row
        for ch in 0..chs {
            let empty_cell = PatternCell {
                note: Note::Empty,
                instrument: 0,
                effects: vec![EffectCommand::default(); usize::from(slots)],
            };
            pattern.set_cell(ch, rows - 1, empty_cell)?;
        }

        Ok(())
    }
}

/// Helper function to safely transpose note values, clamping to standard 1..96 XM note range.
fn transpose_raw_note(raw_val: u8, semitones: i8) -> u8 {
    let mut val = raw_val as i16 + semitones as i16;
    if val < 1 {
        val = 1;
    }
    if val > MAX_XM_NOTES as i16 {
        val = MAX_XM_NOTES as i16;
    }
    val as u8
}
