use crate::*;

#[test]
fn row_state_returns_current_row_cells_for_active_channels() {
    let channel_zero_cell = test_cell(
        PLAY_TEST_CHANNEL_ZERO_NOTE,
        PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
    );
    let channel_one_cell = test_cell(PLAY_TEST_CHANNEL_ONE_NOTE, PLAY_TEST_CHANNEL_ONE_INSTRUMENT);
    let module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
        &[
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW,
                channel_zero_cell.clone(),
            ),
            (
                PLAY_TEST_CHANNEL_ONE,
                PLAYBACK_FIRST_ROW,
                channel_one_cell.clone(),
            ),
        ],
    );
    let clock = PlaybackClock::start(&module).unwrap();

    let row_state = clock.row_state(&module).unwrap();

    assert_eq!(row_state.position.order_index, PLAYBACK_FIRST_ORDER_INDEX);
    assert_eq!(row_state.position.row, PLAYBACK_FIRST_ROW);
    assert_eq!(row_state.channels.len(), PLAY_TEST_TWO_CHANNELS as usize);
    assert_eq!(
        row_state.channels[PLAY_TEST_CHANNEL_ZERO as usize].channel,
        PLAY_TEST_CHANNEL_ZERO
    );
    assert_eq!(
        row_state.channels[PLAY_TEST_CHANNEL_ZERO as usize].cell,
        channel_zero_cell
    );
    assert_eq!(
        row_state.channels[PLAY_TEST_CHANNEL_ONE as usize].channel,
        PLAY_TEST_CHANNEL_ONE
    );
    assert_eq!(
        row_state.channels[PLAY_TEST_CHANNEL_ONE as usize].cell,
        channel_one_cell
    );
}

#[test]
fn row_state_follows_tick_driven_row_advance() {
    let row_one_cell = test_cell(PLAY_TEST_ROW_ONE_NOTE, PLAY_TEST_ROW_ONE_INSTRUMENT);
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_TWO_ROWS,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP,
            row_one_cell.clone(),
        )],
    );
    module.header.tick_speed = PLAY_TEST_ONE_TICK_PER_ROW;
    let mut clock = PlaybackClock::start(&module).unwrap();

    assert_eq!(clock.advance_tick(&module).unwrap(), TickAdvance::NextRow);

    let row_state = clock.row_state(&module).unwrap();
    assert_eq!(
        row_state.position.row,
        PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP
    );
    assert_eq!(
        row_state.channels[PLAY_TEST_CHANNEL_ZERO as usize].cell,
        row_one_cell
    );
}

#[test]
fn row_state_rejects_patterns_with_too_few_channels() {
    let mut module = Module::empty_with_channels(PLAY_TEST_TWO_CHANNELS).unwrap();
    module.orders = vec![PLAY_TEST_PATTERN_ZERO];
    module.patterns = vec![Pattern::new(
        PLAY_TEST_ONE_ROW,
        PLAY_TEST_CHANNELS,
        DEFAULT_EFFECT_SLOTS,
    )];
    let clock = PlaybackClock::start(&module).unwrap();

    assert_eq!(
        clock.row_state(&module).unwrap_err(),
        PlaybackError::PatternChannelOutOfRange {
            pattern_index: PLAY_TEST_FIRST_PATTERN_INDEX,
            module_channels: PLAY_TEST_TWO_CHANNELS,
            pattern_channels: PLAY_TEST_CHANNELS,
        }
    );
}

#[test]
fn playback_state_triggers_initial_note_instrument_sample() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW,
            test_cell(
                PLAY_TEST_CHANNEL_ZERO_NOTE,
                PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
            ),
        )],
    );
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].volume = PLAY_TEST_SAMPLE_VOLUME;
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].panning = PLAY_TEST_SAMPLE_PANNING;

    let playback = PlaybackState::start(&module).unwrap();
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert!(channel.active);
    assert_eq!(channel.channel, PLAY_TEST_CHANNEL_ZERO);
    assert_eq!(channel.note, Note::Key(PLAY_TEST_CHANNEL_ZERO_NOTE));
    assert_eq!(channel.instrument, PLAY_TEST_CHANNEL_ZERO_INSTRUMENT);
    assert_eq!(
        channel.instrument_index,
        Some(PLAY_TEST_FIRST_INSTRUMENT_INDEX)
    );
    assert_eq!(channel.sample_index, Some(PLAY_TEST_FIRST_SAMPLE_INDEX));
    assert_eq!(channel.sample_frame, PLAY_TEST_SAMPLE_START_FRAME);
    assert_eq!(channel.volume, PLAY_TEST_SAMPLE_VOLUME);
    assert_eq!(channel.panning, PLAY_TEST_SAMPLE_PANNING);
}

#[test]
fn playback_state_uses_amiga_period_table_for_mod_notes() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW,
            test_cell(
                PLAY_TEST_CHANNEL_ZERO_NOTE,
                PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
            ),
        )],
    );
    module.header.frequency_table = FrequencyTable::Amiga;

    let playback = PlaybackState::start(&module).unwrap();
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert_eq!(channel.base_period, PLAY_TEST_CHANNEL_ZERO_AMIGA_PERIOD);
    assert_eq!(channel.period, PLAY_TEST_CHANNEL_ZERO_AMIGA_PERIOD);
}

#[test]
fn playback_state_uses_amiga_channel_panning_for_mod_notes() {
    let mut module = Module::empty_with_channels(4).unwrap();
    module.header.frequency_table = FrequencyTable::Amiga;
    module.orders = vec![PLAY_TEST_PATTERN_ZERO];
    let mut pattern = Pattern::new(PLAY_TEST_ONE_ROW, 4, DEFAULT_EFFECT_SLOTS);
    for channel in [
        PLAY_TEST_CHANNEL_ZERO,
        PLAY_TEST_CHANNEL_ONE,
        PLAY_TEST_CHANNEL_TWO,
        PLAY_TEST_CHANNEL_THREE,
    ] {
        pattern
            .set_cell(
                channel,
                PLAYBACK_FIRST_ROW,
                test_cell(
                    PLAY_TEST_CHANNEL_ZERO_NOTE,
                    PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
                ),
            )
            .unwrap();
    }
    module.patterns = vec![pattern];

    let playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].panning,
        0
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ONE as usize].panning,
        255
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_TWO as usize].panning,
        255
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_THREE as usize].panning,
        0
    );
}

#[test]
fn playback_state_preserves_active_channel_on_empty_rows() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_TWO_ROWS,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW,
            test_cell(
                PLAY_TEST_CHANNEL_ZERO_NOTE,
                PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
            ),
        )],
    );
    module.header.tick_speed = PLAY_TEST_ONE_TICK_PER_ROW;
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert!(channel.active);
    assert_eq!(channel.note, Note::Key(PLAY_TEST_CHANNEL_ZERO_NOTE));
    assert_eq!(channel.instrument, PLAY_TEST_CHANNEL_ZERO_INSTRUMENT);
    assert_eq!(channel.sample_index, Some(PLAY_TEST_FIRST_SAMPLE_INDEX));
}

#[test]
fn playback_state_releases_channel_on_note_off() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_TWO_ROWS,
        &[
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW,
                test_cell(
                    PLAY_TEST_CHANNEL_ZERO_NOTE,
                    PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
                ),
            ),
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP,
                note_off_cell(),
            ),
        ],
    );
    module.header.tick_speed = PLAY_TEST_ONE_TICK_PER_ROW;
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert!(!channel.active);
    assert_eq!(channel.note, Note::Off);
    assert_eq!(channel.sample_index, None);
    assert_eq!(channel.sample_frame, PLAY_TEST_SAMPLE_START_FRAME);
}

#[test]
fn playback_state_note_off_with_instrument_releases_current_instrument() {
    let mut note_off = note_off_cell();
    note_off.instrument = PLAY_TEST_CHANNEL_ONE_INSTRUMENT;

    let mut module = module_with_two_channel_cells(
        PLAY_TEST_TWO_ROWS,
        &[
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW,
                test_cell(
                    PLAY_TEST_CHANNEL_ZERO_NOTE,
                    PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
                ),
            ),
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP,
                note_off,
            ),
        ],
    );
    module.header.tick_speed = PLAY_TEST_ONE_TICK_PER_ROW;
    map_instrument_to_sample(
        &mut module,
        PLAY_TEST_FIRST_INSTRUMENT_INDEX,
        PLAY_TEST_FIRST_SAMPLE_INDEX,
    );
    map_instrument_to_sample(
        &mut module,
        PLAY_TEST_SECOND_INSTRUMENT_INDEX,
        PLAY_TEST_SECOND_SAMPLE_INDEX,
    );
    module.instruments[PLAY_TEST_FIRST_INSTRUMENT_INDEX].volume_envelope = Envelope {
        points: vec![
            EnvelopePoint {
                frame: 0,
                value: 256,
            },
            EnvelopePoint {
                frame: 2,
                value: 128,
            },
            EnvelopePoint { frame: 5, value: 0 },
        ],
        point_count: 3,
        sustain_point: 1,
        loop_start_point: 0,
        loop_end_point: 0,
        flags: 0x01 | 0x02,
    };

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert!(channel.active);
    assert!(!channel.keyon);
    assert_eq!(channel.note, Note::Off);
    assert_eq!(channel.instrument, PLAY_TEST_CHANNEL_ZERO_INSTRUMENT);
    assert_eq!(
        channel.instrument_index,
        Some(PLAY_TEST_FIRST_INSTRUMENT_INDEX)
    );
    assert_eq!(channel.sample_index, Some(PLAY_TEST_FIRST_SAMPLE_INDEX));
}

#[test]
fn playback_state_reuses_previous_instrument_for_note_only_rows() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_TWO_ROWS,
        &[
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW,
                test_cell(
                    PLAY_TEST_CHANNEL_ZERO_NOTE,
                    PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
                ),
            ),
            (
                PLAY_TEST_CHANNEL_ZERO,
                PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP,
                note_only_cell(PLAY_TEST_ROW_ONE_NOTE),
            ),
        ],
    );
    module.header.tick_speed = PLAY_TEST_ONE_TICK_PER_ROW;
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    let channel = &playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize];

    assert!(channel.active);
    assert_eq!(channel.note, Note::Key(PLAY_TEST_ROW_ONE_NOTE));
    assert_eq!(channel.instrument, PLAY_TEST_CHANNEL_ZERO_INSTRUMENT);
    assert_eq!(
        channel.instrument_index,
        Some(PLAY_TEST_FIRST_INSTRUMENT_INDEX)
    );
    assert_eq!(channel.sample_index, Some(PLAY_TEST_FIRST_SAMPLE_INDEX));
}

#[test]
fn playback_state_rejects_missing_instruments() {
    let module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW,
            test_cell(PLAY_TEST_CHANNEL_ZERO_NOTE, PLAY_TEST_MISSING_INSTRUMENT),
        )],
    );

    assert_eq!(
        PlaybackState::start(&module).unwrap_err(),
        PlaybackError::MissingInstrument {
            channel: PLAY_TEST_CHANNEL_ZERO,
            instrument: PLAY_TEST_MISSING_INSTRUMENT,
        }
    );
}

#[test]
fn playback_state_rejects_missing_samples() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
        &[(
            PLAY_TEST_CHANNEL_ZERO,
            PLAYBACK_FIRST_ROW,
            test_cell(
                PLAY_TEST_CHANNEL_ZERO_NOTE,
                PLAY_TEST_CHANNEL_ZERO_INSTRUMENT,
            ),
        )],
    );
    let missing_sample_index = module.samples.len();
    module.instruments[PLAY_TEST_FIRST_INSTRUMENT_INDEX].note_sample_map = vec![
            Some(missing_sample_index);
            module.instruments[PLAY_TEST_FIRST_INSTRUMENT_INDEX]
                .note_sample_map
                .len()
        ];

    assert_eq!(
        PlaybackState::start(&module).unwrap_err(),
        PlaybackError::MissingSample {
            channel: PLAY_TEST_CHANNEL_ZERO,
            instrument_index: PLAY_TEST_FIRST_INSTRUMENT_INDEX,
            sample_index: missing_sample_index,
        }
    );
}
