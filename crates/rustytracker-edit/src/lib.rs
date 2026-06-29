//! Core tracker editing operations and undo/redo command history.
//!
//! Exposes a command-driven and snapshot-based transaction model for pattern edits,
//! order modifications, transpositions, and selections.

use std::collections::VecDeque;

use rustytracker_core::{
    CoreError, CoreResult, EffectCommand, Instrument, Module, Note, PatternCell, Sample,
    MAX_ACTIVE_ORDERS, MAX_XM_NOTES,
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

/// Reversible editor commands for pattern and module modifications.
#[derive(Debug, Clone, PartialEq)]
pub enum EditCommand {
    SetNote {
        pattern_idx: usize,
        channel: u16,
        row: u16,
        old_note: Note,
        new_note: Note,
    },
    SetInstrument {
        pattern_idx: usize,
        channel: u16,
        row: u16,
        old_instrument: u8,
        new_instrument: u8,
    },
    SetEffect {
        pattern_idx: usize,
        channel: u16,
        row: u16,
        slot: u8,
        old_effect: EffectCommand,
        new_effect: EffectCommand,
    },
    ClearCell {
        pattern_idx: usize,
        channel: u16,
        row: u16,
        old_cell: PatternCell,
        new_cell: PatternCell,
    },
    InsertOrder {
        index: usize,
        pattern_idx: u8,
    },
    DeleteOrder {
        index: usize,
        old_pattern_idx: u8,
        was_only_one: bool,
    },
    SetOrder {
        index: usize,
        old_pattern_idx: u8,
        new_pattern_idx: u8,
    },
    MoveOrder {
        from_idx: usize,
        to_idx: usize,
    },
    TransposeSelection {
        pattern_idx: usize,
        selection: Selection,
        semitones: i8,
        old_cells: Vec<(u16, u16, PatternCell)>,
    },
    RemapInstrumentSelection {
        pattern_idx: usize,
        selection: Selection,
        from_ins: u8,
        to_ins: u8,
        old_cells: Vec<(u16, u16, PatternCell)>,
    },
    ClearSelection {
        pattern_idx: usize,
        selection: Selection,
        clear_notes: bool,
        clear_instruments: bool,
        clear_effects: bool,
        old_cells: Vec<(u16, u16, PatternCell)>,
    },
    InsertRow {
        pattern_idx: usize,
        row_idx: u16,
        discarded_row_cells: Vec<PatternCell>,
    },
    DeleteRow {
        pattern_idx: usize,
        row_idx: u16,
        deleted_row_cells: Vec<PatternCell>,
    },
    ReplaceModule {
        old_module: Box<Module>,
        new_module: Box<Module>,
    },
    EditInstrumentAndSample {
        instrument_index: usize,
        sample_index: Option<usize>,
        old_instrument: Box<Instrument>,
        old_sample: Option<Box<Sample>>,
        new_instrument: Box<Instrument>,
        new_sample: Option<Box<Sample>>,
    },
}

impl EditCommand {
    pub fn undo(&self, module: &mut Module) -> CoreResult<()> {
        match self {
            EditCommand::SetNote {
                pattern_idx,
                channel,
                row,
                old_note,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                cell.note = *old_note;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::SetInstrument {
                pattern_idx,
                channel,
                row,
                old_instrument,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                cell.instrument = *old_instrument;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::SetEffect {
                pattern_idx,
                channel,
                row,
                slot,
                old_effect,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                if usize::from(*slot) >= cell.effects.len() {
                    return Err(CoreError::InvalidEffectSlot {
                        slot: *slot,
                        slots: cell.effects.len() as u8,
                    });
                }
                cell.effects[usize::from(*slot)] = *old_effect;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::ClearCell {
                pattern_idx,
                channel,
                row,
                old_cell,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                pattern.set_cell(*channel, *row, old_cell.clone())?;
            }
            EditCommand::InsertOrder { index, .. } => {
                let len = module.orders.len();
                if *index < len {
                    if len > 1 {
                        module.orders.remove(*index);
                    } else {
                        module.orders[0] = 0;
                    }
                }
            }
            EditCommand::DeleteOrder {
                index,
                old_pattern_idx,
                was_only_one,
            } => {
                if *was_only_one {
                    if !module.orders.is_empty() {
                        module.orders[0] = *old_pattern_idx;
                    }
                } else {
                    module.orders.insert(*index, *old_pattern_idx);
                }
            }
            EditCommand::SetOrder {
                index,
                old_pattern_idx,
                ..
            } => {
                let len = module.orders.len();
                if *index < len {
                    module.orders[*index] = *old_pattern_idx;
                }
            }
            EditCommand::MoveOrder { from_idx, to_idx } => {
                let len = module.orders.len();
                if *from_idx < len && *to_idx < len {
                    let item = module.orders.remove(*to_idx);
                    module.orders.insert(*from_idx, item);
                }
            }
            EditCommand::TransposeSelection {
                pattern_idx,
                old_cells,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for &(channel, row, ref cell) in old_cells {
                    pattern.set_cell(channel, row, cell.clone())?;
                }
            }
            EditCommand::RemapInstrumentSelection {
                pattern_idx,
                old_cells,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for &(channel, row, ref cell) in old_cells {
                    pattern.set_cell(channel, row, cell.clone())?;
                }
            }
            EditCommand::ClearSelection {
                pattern_idx,
                old_cells,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for &(channel, row, ref cell) in old_cells {
                    pattern.set_cell(channel, row, cell.clone())?;
                }
            }
            EditCommand::InsertRow {
                pattern_idx,
                row_idx,
                discarded_row_cells,
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let rows = pattern.rows();
                let chs = pattern.channels();
                for row in *row_idx..rows - 1 {
                    for ch in 0..chs {
                        let cell_below = pattern.cell(ch, row + 1)?.clone();
                        pattern.set_cell(ch, row, cell_below)?;
                    }
                }
                for ch in 0..chs {
                    pattern.set_cell(ch, rows - 1, discarded_row_cells[ch as usize].clone())?;
                }
            }
            EditCommand::DeleteRow {
                pattern_idx,
                row_idx,
                deleted_row_cells,
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let rows = pattern.rows();
                let chs = pattern.channels();
                for row in (*row_idx + 1..rows).rev() {
                    for ch in 0..chs {
                        let cell_above = pattern.cell(ch, row - 1)?.clone();
                        pattern.set_cell(ch, row, cell_above)?;
                    }
                }
                for ch in 0..chs {
                    pattern.set_cell(ch, *row_idx, deleted_row_cells[ch as usize].clone())?;
                }
            }
            EditCommand::ReplaceModule { old_module, .. } => {
                *module = *old_module.clone();
            }
            EditCommand::EditInstrumentAndSample {
                instrument_index,
                sample_index,
                old_instrument,
                old_sample,
                ..
            } => {
                module.instruments[*instrument_index] = *old_instrument.clone();
                if let (Some(idx), Some(sample)) = (sample_index, old_sample) {
                    module.samples[*idx] = *sample.clone();
                }
            }
        }
        Ok(())
    }

    pub fn redo(&self, module: &mut Module) -> CoreResult<()> {
        match self {
            EditCommand::SetNote {
                pattern_idx,
                channel,
                row,
                new_note,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                cell.note = *new_note;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::SetInstrument {
                pattern_idx,
                channel,
                row,
                new_instrument,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                cell.instrument = *new_instrument;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::SetEffect {
                pattern_idx,
                channel,
                row,
                slot,
                new_effect,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let mut cell = pattern.cell(*channel, *row)?.clone();
                if usize::from(*slot) >= cell.effects.len() {
                    return Err(CoreError::InvalidEffectSlot {
                        slot: *slot,
                        slots: cell.effects.len() as u8,
                    });
                }
                cell.effects[usize::from(*slot)] = *new_effect;
                pattern.set_cell(*channel, *row, cell)?;
            }
            EditCommand::ClearCell {
                pattern_idx,
                channel,
                row,
                new_cell,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                pattern.set_cell(*channel, *row, new_cell.clone())?;
            }
            EditCommand::InsertOrder { index, pattern_idx } => {
                module.orders.insert(*index, *pattern_idx);
            }
            EditCommand::DeleteOrder {
                index,
                was_only_one,
                ..
            } => {
                if *was_only_one {
                    if !module.orders.is_empty() {
                        module.orders[0] = 0;
                    }
                } else {
                    let len = module.orders.len();
                    if *index < len {
                        module.orders.remove(*index);
                    }
                }
            }
            EditCommand::SetOrder {
                index,
                new_pattern_idx,
                ..
            } => {
                let len = module.orders.len();
                if *index < len {
                    module.orders[*index] = *new_pattern_idx;
                }
            }
            EditCommand::MoveOrder { from_idx, to_idx } => {
                let len = module.orders.len();
                if *from_idx < len && *to_idx < len {
                    let item = module.orders.remove(*from_idx);
                    module.orders.insert(*to_idx, item);
                }
            }
            EditCommand::TransposeSelection {
                pattern_idx,
                selection,
                semitones,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for channel in selection.start_channel..=selection.end_channel {
                    for row in selection.start_row..=selection.end_row {
                        if let Ok(cell) = pattern.cell(channel, row) {
                            if let Note::Key(raw_val) = cell.note {
                                let new_note = transpose_raw_note(raw_val, *semitones);
                                let mut updated_cell = cell.clone();
                                updated_cell.note = Note::Key(new_note);
                                pattern.set_cell(channel, row, updated_cell)?;
                            }
                        }
                    }
                }
            }
            EditCommand::RemapInstrumentSelection {
                pattern_idx,
                selection,
                from_ins,
                to_ins,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for channel in selection.start_channel..=selection.end_channel {
                    for row in selection.start_row..=selection.end_row {
                        if let Ok(cell) = pattern.cell(channel, row) {
                            if cell.instrument == *from_ins {
                                let mut updated_cell = cell.clone();
                                updated_cell.instrument = *to_ins;
                                pattern.set_cell(channel, row, updated_cell)?;
                            }
                        }
                    }
                }
            }
            EditCommand::ClearSelection {
                pattern_idx,
                selection,
                clear_notes,
                clear_instruments,
                clear_effects,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                for channel in selection.start_channel..=selection.end_channel {
                    for row in selection.start_row..=selection.end_row {
                        if let Ok(cell) = pattern.cell(channel, row) {
                            let mut updated_cell = cell.clone();
                            if *clear_notes {
                                updated_cell.note = Note::Empty;
                            }
                            if *clear_instruments {
                                updated_cell.instrument = 0;
                            }
                            if *clear_effects {
                                for eff in &mut updated_cell.effects {
                                    *eff = EffectCommand::default();
                                }
                            }
                            pattern.set_cell(channel, row, updated_cell)?;
                        }
                    }
                }
            }
            EditCommand::InsertRow {
                pattern_idx,
                row_idx,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let rows = pattern.rows();
                let chs = pattern.channels();
                let slots = pattern.effect_slots();
                for row in (*row_idx + 1..rows).rev() {
                    for ch in 0..chs {
                        let cell_above = pattern.cell(ch, row - 1)?.clone();
                        pattern.set_cell(ch, row, cell_above)?;
                    }
                }
                for ch in 0..chs {
                    let empty_cell = PatternCell {
                        note: Note::Empty,
                        instrument: 0,
                        effects: vec![EffectCommand::default(); usize::from(slots)],
                    };
                    pattern.set_cell(ch, *row_idx, empty_cell)?;
                }
            }
            EditCommand::DeleteRow {
                pattern_idx,
                row_idx,
                ..
            } => {
                let pattern = module
                    .patterns
                    .get_mut(*pattern_idx)
                    .ok_or(CoreError::PatternNumberOverflow)?;
                let rows = pattern.rows();
                let chs = pattern.channels();
                let slots = pattern.effect_slots();
                for row in *row_idx..rows - 1 {
                    for ch in 0..chs {
                        let cell_below = pattern.cell(ch, row + 1)?.clone();
                        pattern.set_cell(ch, row, cell_below)?;
                    }
                }
                for ch in 0..chs {
                    let empty_cell = PatternCell {
                        note: Note::Empty,
                        instrument: 0,
                        effects: vec![EffectCommand::default(); usize::from(slots)],
                    };
                    pattern.set_cell(ch, rows - 1, empty_cell)?;
                }
            }
            EditCommand::ReplaceModule { new_module, .. } => {
                *module = *new_module.clone();
            }
            EditCommand::EditInstrumentAndSample {
                instrument_index,
                sample_index,
                new_instrument,
                new_sample,
                ..
            } => {
                module.instruments[*instrument_index] = *new_instrument.clone();
                if let (Some(idx), Some(sample)) = (sample_index, new_sample) {
                    module.samples[*idx] = *sample.clone();
                }
            }
        }
        Ok(())
    }
}

/// Undo/Redo stack for command-pattern (delta edits) state restoration.
#[derive(Debug, Clone)]
pub struct UndoHistory {
    undo_stack: VecDeque<EditCommand>,
    redo_stack: VecDeque<EditCommand>,
    limit: usize,
}

impl UndoHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            limit,
        }
    }

    pub fn push_command(&mut self, command: EditCommand) {
        self.undo_stack.push_back(command);
        if self.undo_stack.len() > self.limit {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut Module) -> bool {
        if let Some(cmd) = self.undo_stack.pop_back() {
            if cmd.undo(current).is_ok() {
                self.redo_stack.push_back(cmd);
                if self.redo_stack.len() > self.limit {
                    self.redo_stack.pop_front();
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut Module) -> bool {
        if let Some(cmd) = self.redo_stack.pop_back() {
            if cmd.redo(current).is_ok() {
                self.undo_stack.push_back(cmd);
                if self.undo_stack.len() > self.limit {
                    self.undo_stack.pop_front();
                }
                true
            } else {
                false
            }
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
            let cmd = EditCommand::ReplaceModule {
                old_module: Box::new(self.module.clone()),
                new_module: Box::new(module.clone()),
            };
            self.module = module;
            self.history.push_command(cmd);
        }
    }

    pub fn edit_instrument_and_sample_with_undo<F>(
        &mut self,
        instrument_index: usize,
        sample_index: Option<usize>,
        edit: F,
    ) -> CoreResult<()>
    where
        F: FnOnce(&mut Instrument, Option<&mut Sample>),
    {
        let instrument_len = self.module.instruments.len();
        if instrument_index >= instrument_len {
            return Err(CoreError::InvalidInstrumentIndex {
                index: instrument_index,
                len: instrument_len,
            });
        }

        if let Some(index) = sample_index {
            let sample_len = self.module.samples.len();
            if index >= sample_len {
                return Err(CoreError::InvalidSampleIndex {
                    index,
                    len: sample_len,
                });
            }
        }

        let old_instrument = Box::new(self.module.instruments[instrument_index].clone());
        let old_sample = sample_index.map(|index| Box::new(self.module.samples[index].clone()));

        let Module {
            instruments,
            samples,
            ..
        } = &mut self.module;
        let instrument = &mut instruments[instrument_index];
        let sample = sample_index.map(|index| &mut samples[index]);
        edit(instrument, sample);

        let new_instrument = Box::new(self.module.instruments[instrument_index].clone());
        let new_sample = sample_index.map(|index| Box::new(self.module.samples[index].clone()));

        let cmd = EditCommand::EditInstrumentAndSample {
            instrument_index,
            sample_index,
            old_instrument,
            old_sample,
            new_instrument,
            new_sample,
        };
        self.history.push_command(cmd);

        Ok(())
    }

    pub fn into_module(self) -> Module {
        self.module
    }

    /// Stub to satisfy legacy callers. Commands are now auto-recorded on every mutation.
    #[deprecated(
        since = "0.2.0",
        note = "Commands are now auto-recorded, begin_transaction is no longer needed."
    )]
    pub fn begin_transaction(&mut self) {}

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
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        let old_note = cell.note;
        cell.note = note;

        let cmd = EditCommand::SetNote {
            pattern_idx,
            channel,
            row,
            old_note,
            new_note: note,
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        pattern.set_cell(channel, row, cell)?;

        self.history.push_command(cmd);
        Ok(())
    }

    pub fn set_instrument(
        &mut self,
        pattern_idx: usize,
        channel: u16,
        row: u16,
        instrument: u8,
    ) -> CoreResult<()> {
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        let old_instrument = cell.instrument;
        cell.instrument = instrument;

        let cmd = EditCommand::SetInstrument {
            pattern_idx,
            channel,
            row,
            old_instrument,
            new_instrument: instrument,
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        pattern.set_cell(channel, row, cell)?;

        self.history.push_command(cmd);
        Ok(())
    }

    pub fn set_effect(
        &mut self,
        pattern_idx: usize,
        channel: u16,
        row: u16,
        slot: u8,
        command: EffectCommand,
    ) -> CoreResult<()> {
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let mut cell = pattern.cell(channel, row)?.clone();
        if usize::from(slot) >= cell.effects.len() {
            return Err(CoreError::InvalidEffectSlot {
                slot,
                slots: cell.effects.len() as u8,
            });
        }
        let old_effect = cell.effects[usize::from(slot)];
        cell.effects[usize::from(slot)] = command;

        let cmd = EditCommand::SetEffect {
            pattern_idx,
            channel,
            row,
            slot,
            old_effect,
            new_effect: command,
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        pattern.set_cell(channel, row, cell)?;

        self.history.push_command(cmd);
        Ok(())
    }

    pub fn clear_cell(&mut self, pattern_idx: usize, channel: u16, row: u16) -> CoreResult<()> {
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let old_cell = pattern.cell(channel, row)?.clone();
        let effect_slots = pattern.effect_slots();
        let clean_cell = PatternCell {
            note: Note::Empty,
            instrument: 0,
            effects: vec![EffectCommand::default(); usize::from(effect_slots)],
        };

        let cmd = EditCommand::ClearCell {
            pattern_idx,
            channel,
            row,
            old_cell,
            new_cell: clean_cell.clone(),
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        pattern.set_cell(channel, row, clean_cell)?;

        self.history.push_command(cmd);
        Ok(())
    }

    // --- Order List Editing ---

    pub fn insert_duplicate_order(&mut self, index: usize) -> CoreResult<()> {
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

        let cmd = EditCommand::InsertOrder {
            index: index + 1,
            pattern_idx: pattern,
        };

        self.module.orders.insert(index + 1, pattern);
        self.history.push_command(cmd);
        Ok(())
    }

    pub fn delete_order(&mut self, index: usize) -> CoreResult<()> {
        if index >= self.module.orders.len() {
            return Err(CoreError::InvalidOrderIndex {
                index,
                len: self.module.orders.len(),
            });
        }

        let old_pattern_idx = self.module.orders[index];
        let was_only_one = self.module.orders.len() == 1;

        let cmd = EditCommand::DeleteOrder {
            index,
            old_pattern_idx,
            was_only_one,
        };

        if !was_only_one {
            self.module.orders.remove(index);
        } else {
            self.module.orders[0] = 0;
        }

        self.history.push_command(cmd);
        Ok(())
    }

    pub fn set_order_pattern(&mut self, index: usize, pattern_idx: u8) -> CoreResult<()> {
        let len = self.module.orders.len();
        if index >= len {
            return Err(CoreError::InvalidOrderIndex { index, len });
        }

        let old_pattern_idx = self.module.orders[index];
        let cmd = EditCommand::SetOrder {
            index,
            old_pattern_idx,
            new_pattern_idx: pattern_idx,
        };

        self.module.orders[index] = pattern_idx;
        self.history.push_command(cmd);
        Ok(())
    }

    pub fn move_order(&mut self, from_idx: usize, to_idx: usize) -> CoreResult<()> {
        let len = self.module.orders.len();
        if from_idx >= len || to_idx >= len {
            return Err(CoreError::InvalidOrderIndex {
                index: from_idx.max(to_idx),
                len,
            });
        }

        let cmd = EditCommand::MoveOrder { from_idx, to_idx };

        let item = self.module.orders.remove(from_idx);
        self.module.orders.insert(to_idx, item);
        self.history.push_command(cmd);
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
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        let mut old_cells = Vec::new();
        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    old_cells.push((channel, row, cell.clone()));
                }
            }
        }

        let cmd = EditCommand::TransposeSelection {
            pattern_idx,
            selection,
            semitones,
            old_cells,
        };

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

        self.history.push_command(cmd);
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
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        let mut old_cells = Vec::new();
        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    old_cells.push((channel, row, cell.clone()));
                }
            }
        }

        let cmd = EditCommand::RemapInstrumentSelection {
            pattern_idx,
            selection,
            from_ins,
            to_ins,
            old_cells,
        };

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

        self.history.push_command(cmd);
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
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        let mut old_cells = Vec::new();
        for channel in selection.start_channel..=selection.end_channel {
            for row in selection.start_row..=selection.end_row {
                if let Ok(cell) = pattern.cell(channel, row) {
                    old_cells.push((channel, row, cell.clone()));
                }
            }
        }

        let cmd = EditCommand::ClearSelection {
            pattern_idx,
            selection,
            clear_notes,
            clear_instruments,
            clear_effects,
            old_cells,
        };

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

        self.history.push_command(cmd);
        Ok(())
    }

    // --- Pattern Manipulation Tools ---

    /// Inserts a blank row at the target index in the pattern, shifting rows down and discarding the last row.
    pub fn insert_row(&mut self, pattern_idx: usize, row_idx: u16) -> CoreResult<()> {
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let rows = pattern.rows();
        let chs = pattern.channels();
        let slots = pattern.effect_slots();

        if row_idx >= rows {
            return Err(CoreError::InvalidRow { row: row_idx, rows });
        }

        let mut discarded_row_cells = Vec::new();
        for ch in 0..chs {
            discarded_row_cells.push(pattern.cell(ch, rows - 1)?.clone());
        }

        let cmd = EditCommand::InsertRow {
            pattern_idx,
            row_idx,
            discarded_row_cells,
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

        // Shift cells down
        for row in (row_idx + 1..rows).rev() {
            for ch in 0..chs {
                let cell_above = pattern.cell(ch, row - 1)?.clone();
                pattern.set_cell(ch, row, cell_above)?;
            }
        }

        // Insert clean empty cell
        for ch in 0..chs {
            let empty_cell = PatternCell {
                note: Note::Empty,
                instrument: 0,
                effects: vec![EffectCommand::default(); usize::from(slots)],
            };
            pattern.set_cell(ch, row_idx, empty_cell)?;
        }

        self.history.push_command(cmd);
        Ok(())
    }

    /// Deletes the row at the target index, shifting subsequent rows up and filling the last row with empty cells.
    pub fn delete_row(&mut self, pattern_idx: usize, row_idx: u16) -> CoreResult<()> {
        let pattern = self
            .module
            .patterns
            .get(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;
        let rows = pattern.rows();
        let chs = pattern.channels();
        let slots = pattern.effect_slots();

        if row_idx >= rows {
            return Err(CoreError::InvalidRow { row: row_idx, rows });
        }

        let mut deleted_row_cells = Vec::new();
        for ch in 0..chs {
            deleted_row_cells.push(pattern.cell(ch, row_idx)?.clone());
        }

        let cmd = EditCommand::DeleteRow {
            pattern_idx,
            row_idx,
            deleted_row_cells,
        };

        let pattern = self
            .module
            .patterns
            .get_mut(pattern_idx)
            .ok_or(CoreError::PatternNumberOverflow)?;

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

        self.history.push_command(cmd);
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
