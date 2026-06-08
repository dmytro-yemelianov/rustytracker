use rustytracker_core::{EffectCommand, Module, Note, NoteName};
use rustytracker_edit::{ModuleEditor, Selection};

#[test]
fn test_undo_redo_snapshots() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    assert!(!editor.can_undo());
    assert!(!editor.can_redo());

    // 1. Make first edit (set note at pattern 0, channel 0, row 0 to C-4)
    let note_c4 = Note::key(4, NoteName::C).unwrap();
    editor.set_note(0, 0, 0, note_c4).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, note_c4);
    assert!(editor.can_undo());
    assert!(!editor.can_redo());

    // 2. Make second edit (set instrument to 5)
    editor.set_instrument(0, 0, 0, 5).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 5);

    // 3. Undo second edit (instrument goes back to 0)
    assert!(editor.undo());
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 0);
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, note_c4);
    assert!(editor.can_redo());

    // 4. Undo first edit (note goes back to Empty)
    assert!(editor.undo());
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, Note::Empty);

    // 5. Redo first edit
    assert!(editor.redo());
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, note_c4);
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 0);

    // 6. Redo second edit
    assert!(editor.redo());
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 5);
}

#[test]
fn test_note_and_cell_edits() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    // Set cell notes, instruments, effects
    let note_e5 = Note::key(5, NoteName::E).unwrap();
    editor.set_note(0, 1, 10, note_e5).unwrap();
    editor.set_instrument(0, 1, 10, 12).unwrap();
    editor.set_effect(0, 1, 10, 0, EffectCommand { effect: 0x0c, operand: 64 }).unwrap();

    let cell = editor.module().patterns[0].cell(1, 10).unwrap();
    assert_eq!(cell.note, note_e5);
    assert_eq!(cell.instrument, 12);
    assert_eq!(cell.effects[0].effect, 0x0c);
    assert_eq!(cell.effects[0].operand, 64);

    // Clear cell
    editor.clear_cell(0, 1, 10).unwrap();
    let cell_cleared = editor.module().patterns[0].cell(1, 10).unwrap();
    assert_eq!(cell_cleared.note, Note::Empty);
    assert_eq!(cell_cleared.instrument, 0);
    assert_eq!(cell_cleared.effects[0], EffectCommand::default());
}

#[test]
fn test_selection_transposition() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    let note_c4 = Note::key(4, NoteName::C).unwrap(); // raw 49
    let note_d4 = Note::key(4, NoteName::D).unwrap(); // raw 51
    editor.set_note(0, 0, 0, note_c4).unwrap();
    editor.set_note(0, 1, 1, note_d4).unwrap();

    // Define selection encompassing both notes
    let selection = Selection {
        start_channel: 0,
        end_channel: 2,
        start_row: 0,
        end_row: 2,
    };

    // Transpose up by 3 semitones (C-4 49 -> D#4 52, D-4 51 -> F-4 54)
    editor.transpose_selection(0, selection, 3).unwrap();

    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, Note::Key(52));
    assert_eq!(editor.module().patterns[0].cell(1, 1).unwrap().note, Note::Key(54));

    // Transpose down by 1 octave (-12 semitones)
    editor.transpose_selection(0, selection, -12).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, Note::Key(40));
    assert_eq!(editor.module().patterns[0].cell(1, 1).unwrap().note, Note::Key(42));
}

#[test]
fn test_selection_clear_and_remap() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    let note_c4 = Note::key(4, NoteName::C).unwrap();
    editor.set_note(0, 0, 0, note_c4).unwrap();
    editor.set_instrument(0, 0, 0, 3).unwrap();
    editor.set_effect(0, 0, 0, 0, EffectCommand { effect: 0x0c, operand: 64 }).unwrap();

    editor.set_note(0, 1, 1, note_c4).unwrap();
    editor.set_instrument(0, 1, 1, 3).unwrap();

    let selection = Selection {
        start_channel: 0,
        end_channel: 2,
        start_row: 0,
        end_row: 2,
    };

    // Remap instrument 3 to 9 in selection
    editor.remap_instrument_selection(0, selection, 3, 9).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 9);
    assert_eq!(editor.module().patterns[0].cell(1, 1).unwrap().instrument, 9);

    // Clear notes only in selection
    editor.clear_selection(0, selection, true, false, false).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().note, Note::Empty);
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 9);
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().effects[0].effect, 0x0c);

    // Clear instruments and effects in selection
    editor.clear_selection(0, selection, false, true, true).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().instrument, 0);
    assert_eq!(editor.module().patterns[0].cell(0, 0).unwrap().effects[0], EffectCommand::default());
}

#[test]
fn test_order_operations() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    // Initial order list is [0]
    assert_eq!(editor.module().orders, vec![0]);

    // Insert duplicate order
    editor.insert_duplicate_order(0).unwrap();
    assert_eq!(editor.module().orders, vec![0, 0]);

    // Set order pattern
    editor.set_order_pattern(1, 4).unwrap();
    assert_eq!(editor.module().orders, vec![0, 4]);

    // Move order
    editor.insert_duplicate_order(1).unwrap(); // [0, 4, 4]
    editor.set_order_pattern(2, 7).unwrap(); // [0, 4, 7]
    editor.move_order(2, 0).unwrap(); // move index 2 (7) to index 0 -> [7, 0, 4]
    assert_eq!(editor.module().orders, vec![7, 0, 4]);

    // Delete order
    editor.delete_order(1).unwrap(); // [7, 4]
    assert_eq!(editor.module().orders, vec![7, 4]);
}

#[test]
fn test_pattern_insert_delete_row() {
    let module = Module::empty();
    let mut editor = ModuleEditor::new(module);

    let note_c4 = Note::key(4, NoteName::C).unwrap();
    let note_d4 = Note::key(4, NoteName::D).unwrap();
    editor.set_note(0, 0, 2, note_c4).unwrap();
    editor.set_note(0, 0, 3, note_d4).unwrap();

    // Insert row at 2. Note C-4 at row 2 moves to row 3. Note D-4 moves to row 4. Row 2 becomes empty.
    editor.insert_row(0, 2).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 2).unwrap().note, Note::Empty);
    assert_eq!(editor.module().patterns[0].cell(0, 3).unwrap().note, note_c4);
    assert_eq!(editor.module().patterns[0].cell(0, 4).unwrap().note, note_d4);

    // Delete row at 2. Note C-4 at row 3 shifts back to row 2. Note D-4 shifts to row 3.
    editor.delete_row(0, 2).unwrap();
    assert_eq!(editor.module().patterns[0].cell(0, 2).unwrap().note, note_c4);
    assert_eq!(editor.module().patterns[0].cell(0, 3).unwrap().note, note_d4);
}
