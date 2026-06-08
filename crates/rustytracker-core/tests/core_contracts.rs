use rustytracker_core::{
    CoreError, EffectCommand, FrequencyTable, InstrumentName, Module, ModuleTitle, Note, NoteName,
    OrderList, Pattern, PatternCell, SampleName, DEFAULT_BPM, DEFAULT_INSTRUMENTS,
    DEFAULT_MAIN_VOLUME, DEFAULT_PATTERN_ROWS, DEFAULT_SAMPLE_COUNT, DEFAULT_SONG_CHANNELS,
    DEFAULT_TICK_SPEED, EDITOR_PATTERN_CHANNELS, INSTRUMENT_NAME_LEN, MAX_ACTIVE_ORDERS,
    MAX_XM_NOTES, NOTE_OFF_VALUE, SAMPLE_DEFAULT_FLAGS, SAMPLE_DEFAULT_PANNING,
    SAMPLE_DEFAULT_VOLUME, SAMPLE_DEFAULT_VOLUME_FADEOUT, SAMPLE_NAME_LEN, TITLE_TEXT_LEN,
};

#[test]
fn empty_module_uses_milkytracker_defaults() {
    let module = Module::empty();

    assert_eq!(module.header.channel_count, DEFAULT_SONG_CHANNELS);
    assert_eq!(module.header.frequency_table, FrequencyTable::Linear);
    assert_eq!(module.header.bpm, DEFAULT_BPM);
    assert_eq!(module.header.tick_speed, DEFAULT_TICK_SPEED);
    assert_eq!(module.header.main_volume, DEFAULT_MAIN_VOLUME);
    assert_eq!(module.header.restart_position, 0);
    assert_eq!(module.orders, vec![0]);
    assert_eq!(module.patterns.len(), 1);
    assert_eq!(module.instruments.len(), DEFAULT_INSTRUMENTS);
    assert_eq!(module.samples.len(), DEFAULT_SAMPLE_COUNT);
}

#[test]
fn empty_module_rejects_channel_counts_the_editor_cannot_represent() {
    assert_eq!(
        Module::empty_with_channels(0).unwrap_err(),
        CoreError::InvalidChannelCount(0)
    );
    assert_eq!(
        Module::empty_with_channels(EDITOR_PATTERN_CHANNELS + 1).unwrap_err(),
        CoreError::InvalidChannelCount(EDITOR_PATTERN_CHANNELS + 1)
    );
}

#[test]
fn allocated_patterns_match_milkytracker_editor_shape() {
    let pattern = Pattern::empty_editor_pattern();

    assert_eq!(pattern.rows(), DEFAULT_PATTERN_ROWS);
    assert_eq!(pattern.channels(), EDITOR_PATTERN_CHANNELS);
    assert_eq!(pattern.effect_slots(), 2);

    let first = pattern.cell(0, 0).unwrap();
    assert_eq!(first.note, Note::Empty);
    assert_eq!(first.instrument, 0);
    assert_eq!(first.effects, vec![EffectCommand::default(); 2]);
}

#[test]
fn pattern_cells_are_bounds_checked() {
    let pattern = Pattern::empty_editor_pattern();

    assert!(matches!(
        pattern.cell(EDITOR_PATTERN_CHANNELS, 0),
        Err(CoreError::InvalidChannel { .. })
    ));
    assert!(matches!(
        pattern.cell(0, DEFAULT_PATTERN_ROWS),
        Err(CoreError::InvalidRow { .. })
    ));
}

#[test]
fn pattern_cell_write_requires_matching_effect_slot_count() {
    let mut pattern = Pattern::empty_editor_pattern();
    let bad_cell = PatternCell {
        effects: vec![EffectCommand::default(); 1],
        ..PatternCell::default()
    };

    assert!(matches!(
        pattern.set_cell(0, 0, bad_cell),
        Err(CoreError::InvalidEffectSlot { .. })
    ));
}

#[test]
fn notes_use_fasttracker_numeric_encoding() {
    assert_eq!(Note::Empty.raw(), 0);
    assert_eq!(Note::key(0, NoteName::C).unwrap().raw(), 1);
    assert_eq!(Note::key(7, NoteName::B).unwrap().raw(), MAX_XM_NOTES);
    assert_eq!(Note::Off.raw(), NOTE_OFF_VALUE);
    assert!(matches!(
        Note::key(8, NoteName::C),
        Err(CoreError::InvalidNote { .. })
    ));
}

#[test]
fn cell_write_roundtrips_exact_slot_values() {
    let mut pattern = Pattern::empty_editor_pattern();
    let cell = PatternCell {
        note: Note::key(4, NoteName::CSharp).unwrap(),
        instrument: 3,
        effects: vec![
            EffectCommand {
                effect: 0x0c,
                operand: 0x40,
            },
            EffectCommand {
                effect: 0x0f,
                operand: 0x7d,
            },
        ],
    };

    pattern.set_cell(7, 12, cell.clone()).unwrap();
    assert_eq!(pattern.cell(7, 12).unwrap(), &cell);
}

#[test]
fn fixed_text_fields_use_milkytracker_visible_lengths() {
    let title = ModuleTitle::new("ABCDEFGHIJKLMNOPQRSTUV");
    let instrument = InstrumentName::new("ABCDEFGHIJKLMNOPQRSTUVXYZ");
    let sample = SampleName::new("012345678901234567890123");

    assert_eq!(title.as_str(), "ABCDEFGHIJKLMNOPQRST");
    assert_eq!(title.capacity(), TITLE_TEXT_LEN);
    assert_eq!(instrument.as_str(), "ABCDEFGHIJKLMNOPQRSTUV");
    assert_eq!(instrument.capacity(), INSTRUMENT_NAME_LEN);
    assert_eq!(sample.as_str(), "0123456789012345678901");
    assert_eq!(sample.capacity(), SAMPLE_NAME_LEN);
}

#[test]
fn order_list_len_is_clamped_to_milkytracker_active_range() {
    let mut orders = OrderList::default();

    orders.set_len_clamped(0);
    assert_eq!(orders.as_slice(), &[0]);

    orders.set_len_clamped(MAX_ACTIVE_ORDERS + 10);
    assert_eq!(orders.len(), MAX_ACTIVE_ORDERS);
    assert!(orders.as_slice().iter().all(|&pattern| pattern == 0));
}

#[test]
fn inserting_an_order_duplicates_the_selected_pattern_number() {
    let mut orders = OrderList::from_orders(vec![3, 7, 9]).unwrap();

    orders.insert_duplicate_after(1).unwrap();

    assert_eq!(orders.as_slice(), &[3, 7, 7, 9]);
}

#[test]
fn deleting_orders_preserves_at_least_one_order() {
    let mut orders = OrderList::from_orders(vec![2]).unwrap();
    orders.delete(0).unwrap();
    assert_eq!(orders.as_slice(), &[2]);

    let mut orders = OrderList::from_orders(vec![2, 4]).unwrap();
    orders.delete(0).unwrap();
    assert_eq!(orders.as_slice(), &[4]);
}

#[test]
fn sequencing_an_order_uses_the_next_pattern_number_after_current_highest() {
    let mut orders = OrderList::from_orders(vec![0, 3, 1]).unwrap();

    let inserted = orders.sequence_after(1).unwrap();

    assert_eq!(inserted, 4);
    assert_eq!(orders.as_slice(), &[0, 3, 4, 1]);
}

#[test]
fn default_instruments_and_samples_match_empty_song_pool_defaults() {
    let module = Module::empty();
    let instrument = &module.instruments[0];
    let sample = &module.samples[0];

    assert_eq!(instrument.name.as_str(), "");
    assert_eq!(instrument.sample_slots.len(), 16);
    assert!(instrument.sample_slots.iter().all(|slot| slot.is_some()));
    assert_eq!(instrument.note_sample_map.len(), 96);
    assert!(instrument
        .note_sample_map
        .iter()
        .all(|&sample_index| sample_index == 0));

    assert_eq!(sample.name.as_str(), "");
    assert_eq!(sample.length, 0);
    assert_eq!(sample.loop_start, 0);
    assert_eq!(sample.loop_length, 0);
    assert_eq!(sample.volume, SAMPLE_DEFAULT_VOLUME);
    assert_eq!(sample.panning, SAMPLE_DEFAULT_PANNING);
    assert_eq!(sample.flags, SAMPLE_DEFAULT_FLAGS);
    assert_eq!(sample.volume_fadeout, SAMPLE_DEFAULT_VOLUME_FADEOUT);
}
