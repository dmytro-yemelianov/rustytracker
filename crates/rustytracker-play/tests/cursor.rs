use rustytracker_core::{Module, Note, Pattern, PatternCell, SampleData, DEFAULT_EFFECT_SLOTS};
use rustytracker_play::{
    ChannelSampleFrame, PlaybackClock, PlaybackCursor, PlaybackError, PlaybackSampleValue,
    PlaybackState, PlaybackTiming, RowAdvance, TickAdvance, PLAYBACK_FIRST_ORDER_INDEX,
    PLAYBACK_FIRST_ROW, PLAYBACK_FIRST_TICK, PLAYBACK_ORDER_STEP, PLAYBACK_ROW_STEP,
    PLAYBACK_TICK_STEP,
};

const PLAY_TEST_CHANNELS: u16 = 1;
const PLAY_TEST_TWO_CHANNELS: u16 = 2;
const PLAY_TEST_CHANNEL_ZERO: u16 = 0;
const PLAY_TEST_CHANNEL_ONE: u16 = 1;
const PLAY_TEST_PATTERN_ZERO: u8 = 0;
const PLAY_TEST_PATTERN_ONE: u8 = 1;
const PLAY_TEST_FIRST_PATTERN_INDEX: usize = 0;
const PLAY_TEST_SECOND_PATTERN_INDEX: usize = 1;
const PLAY_TEST_ZERO_ROWS: u16 = 0;
const PLAY_TEST_ONE_ROW: u16 = 1;
const PLAY_TEST_TWO_ROWS: u16 = 2;
const PLAY_TEST_THREE_ROWS: u16 = 3;
const PLAY_TEST_DEFAULT_TICK_SPEED: u16 = 6;
const PLAY_TEST_ONE_TICK_PER_ROW: u16 = 1;
const PLAY_TEST_THREE_TICKS_PER_ROW: u16 = 3;
const PLAY_TEST_DEFAULT_BPM: u16 = 125;
const PLAY_TEST_FAST_BPM: u16 = 250;
const PLAY_TEST_ZERO_TICK_SPEED: u16 = 0;
const PLAY_TEST_ZERO_BPM: u16 = 0;
const PLAY_TEST_DEFAULT_TICK_NANOS: u64 = 20_000_000;
const PLAY_TEST_DEFAULT_ROW_NANOS: u64 =
    PLAY_TEST_DEFAULT_TICK_NANOS * PLAY_TEST_DEFAULT_TICK_SPEED as u64;
const PLAY_TEST_FAST_TICK_NANOS: u64 = 10_000_000;
const PLAY_TEST_FAST_ROW_NANOS: u64 =
    PLAY_TEST_FAST_TICK_NANOS * PLAY_TEST_THREE_TICKS_PER_ROW as u64;
const PLAY_TEST_CHANNEL_ZERO_NOTE: u8 = 49;
const PLAY_TEST_CHANNEL_ONE_NOTE: u8 = 50;
const PLAY_TEST_ROW_ONE_NOTE: u8 = 51;
const PLAY_TEST_CHANNEL_ZERO_INSTRUMENT: u8 = 1;
const PLAY_TEST_CHANNEL_ONE_INSTRUMENT: u8 = 2;
const PLAY_TEST_ROW_ONE_INSTRUMENT: u8 = 3;
const PLAY_TEST_FIRST_INSTRUMENT_INDEX: usize = 0;
const PLAY_TEST_FIRST_SAMPLE_INDEX: usize = 0;
const PLAY_TEST_SAMPLE_START_FRAME: usize = 0;
const PLAY_TEST_SECOND_SAMPLE_FRAME: usize = 1;
const PLAY_TEST_SAMPLE_VOLUME: u8 = 48;
const PLAY_TEST_SAMPLE_PANNING: u8 = 96;
const PLAY_TEST_MISSING_INSTRUMENT: u8 = 200;
const PLAY_TEST_PCM8_FIRST_VALUE: i8 = -2;
const PLAY_TEST_PCM8_SECOND_VALUE: i8 = 3;
const PLAY_TEST_PCM16_FIRST_VALUE: i16 = -512;
const PLAY_TEST_PCM16_SECOND_VALUE: i16 = 1024;

#[test]
fn starts_at_first_order_first_row() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);

    let cursor = PlaybackCursor::start(&module).unwrap();
    let position = cursor.position(&module).unwrap();

    assert_eq!(position.order_index, PLAYBACK_FIRST_ORDER_INDEX);
    assert_eq!(position.pattern_index, PLAY_TEST_FIRST_PATTERN_INDEX);
    assert_eq!(position.row, PLAYBACK_FIRST_ROW);
}

#[test]
fn advances_rows_inside_current_pattern() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    let mut cursor = PlaybackCursor::start(&module).unwrap();

    assert_eq!(cursor.advance_row(&module).unwrap(), RowAdvance::SameOrder);

    let position = cursor.position(&module).unwrap();
    assert_eq!(position.order_index, PLAYBACK_FIRST_ORDER_INDEX);
    assert_eq!(position.row, PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP);
}

#[test]
fn advances_to_next_order_after_last_pattern_row() {
    let module = module_with_orders_and_pattern_rows(
        vec![PLAY_TEST_PATTERN_ZERO, PLAY_TEST_PATTERN_ONE],
        &[PLAY_TEST_TWO_ROWS, PLAY_TEST_THREE_ROWS],
    );
    let mut cursor = PlaybackCursor::start(&module).unwrap();

    assert_eq!(cursor.advance_row(&module).unwrap(), RowAdvance::SameOrder);
    assert_eq!(cursor.advance_row(&module).unwrap(), RowAdvance::NextOrder);

    let position = cursor.position(&module).unwrap();
    assert_eq!(
        position.order_index,
        PLAYBACK_FIRST_ORDER_INDEX + PLAYBACK_ORDER_STEP
    );
    assert_eq!(position.pattern_index, PLAY_TEST_SECOND_PATTERN_INDEX);
    assert_eq!(position.row, PLAYBACK_FIRST_ROW);
}

#[test]
fn reports_song_end_after_last_pattern_row() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let mut cursor = PlaybackCursor::start(&module).unwrap();

    assert_eq!(cursor.advance_row(&module).unwrap(), RowAdvance::SongEnd);

    let position = cursor.position(&module).unwrap();
    assert_eq!(position.order_index, PLAYBACK_FIRST_ORDER_INDEX);
    assert_eq!(position.row, PLAYBACK_FIRST_ROW);
}

#[test]
fn rejects_empty_order_lists() {
    let module = module_with_orders_and_pattern_rows(Vec::new(), &[PLAY_TEST_ONE_ROW]);

    assert_eq!(
        PlaybackCursor::start(&module).unwrap_err(),
        PlaybackError::EmptyOrderList
    );
}

#[test]
fn rejects_orders_that_reference_missing_patterns() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ONE], &[PLAY_TEST_ONE_ROW]);

    assert_eq!(
        PlaybackCursor::start(&module).unwrap_err(),
        PlaybackError::MissingPattern {
            order_index: PLAYBACK_FIRST_ORDER_INDEX,
            pattern_index: PLAY_TEST_SECOND_PATTERN_INDEX,
        }
    );
}

#[test]
fn rejects_empty_patterns() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ZERO_ROWS]);

    assert_eq!(
        PlaybackCursor::start(&module).unwrap_err(),
        PlaybackError::EmptyPattern {
            pattern_index: PLAY_TEST_FIRST_PATTERN_INDEX,
        }
    );
}

#[test]
fn derives_tick_and_row_duration_from_module_speed_and_bpm() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);

    let timing = PlaybackTiming::from_module(&module).unwrap();

    assert_eq!(timing.ticks_per_row(), PLAY_TEST_DEFAULT_TICK_SPEED);
    assert_eq!(timing.bpm(), PLAY_TEST_DEFAULT_BPM);
    assert_eq!(timing.tick_duration_nanos(), PLAY_TEST_DEFAULT_TICK_NANOS);
    assert_eq!(timing.row_duration_nanos(), PLAY_TEST_DEFAULT_ROW_NANOS);
}

#[test]
fn derives_shorter_tick_duration_for_higher_bpm() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = PLAY_TEST_THREE_TICKS_PER_ROW;
    module.header.bpm = PLAY_TEST_FAST_BPM;

    let timing = PlaybackTiming::from_module(&module).unwrap();

    assert_eq!(timing.ticks_per_row(), PLAY_TEST_THREE_TICKS_PER_ROW);
    assert_eq!(timing.bpm(), PLAY_TEST_FAST_BPM);
    assert_eq!(timing.tick_duration_nanos(), PLAY_TEST_FAST_TICK_NANOS);
    assert_eq!(timing.row_duration_nanos(), PLAY_TEST_FAST_ROW_NANOS);
}

#[test]
fn rejects_zero_tick_speed_for_timing() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = PLAY_TEST_ZERO_TICK_SPEED;

    assert_eq!(
        PlaybackTiming::from_module(&module).unwrap_err(),
        PlaybackError::InvalidTickSpeed {
            tick_speed: PLAY_TEST_ZERO_TICK_SPEED,
        }
    );
}

#[test]
fn rejects_zero_bpm_for_timing() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.bpm = PLAY_TEST_ZERO_BPM;

    assert_eq!(
        PlaybackTiming::from_module(&module).unwrap_err(),
        PlaybackError::InvalidBpm {
            bpm: PLAY_TEST_ZERO_BPM,
        }
    );
}

#[test]
fn playback_clock_advances_ticks_before_rows() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = PLAY_TEST_THREE_TICKS_PER_ROW;
    module.header.bpm = PLAY_TEST_FAST_BPM;
    let mut state = PlaybackClock::start(&module).unwrap();

    assert_eq!(state.tick(), PLAYBACK_FIRST_TICK);
    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::SameRow);
    assert_eq!(state.tick(), PLAYBACK_FIRST_TICK + PLAYBACK_TICK_STEP);

    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::SameRow);
    assert_eq!(
        state.tick(),
        PLAYBACK_FIRST_TICK + PLAYBACK_TICK_STEP + PLAYBACK_TICK_STEP
    );

    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::NextRow);

    let position = state.position(&module).unwrap();
    assert_eq!(position.row, PLAYBACK_FIRST_ROW + PLAYBACK_ROW_STEP);
    assert_eq!(state.tick(), PLAYBACK_FIRST_TICK);
}

#[test]
fn playback_clock_reports_song_end_without_moving_past_final_tick() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = PLAY_TEST_THREE_TICKS_PER_ROW;
    let mut state = PlaybackClock::start(&module).unwrap();

    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::SameRow);
    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::SameRow);
    assert_eq!(state.advance_tick(&module).unwrap(), TickAdvance::SongEnd);

    let position = state.position(&module).unwrap();
    assert_eq!(position.order_index, PLAYBACK_FIRST_ORDER_INDEX);
    assert_eq!(position.row, PLAYBACK_FIRST_ROW);
    assert_eq!(
        state.tick(),
        PLAYBACK_FIRST_TICK + PLAYBACK_TICK_STEP + PLAYBACK_TICK_STEP
    );
}

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

#[test]
fn sample_step_reads_pcm8_frames_and_advances_position() {
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::Pcm8(vec![
        PLAY_TEST_PCM8_FIRST_VALUE,
        PLAY_TEST_PCM8_SECOND_VALUE,
    ]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.step_samples(&module).unwrap(),
        vec![ChannelSampleFrame {
            channel: PLAY_TEST_CHANNEL_ZERO,
            sample_index: PLAY_TEST_FIRST_SAMPLE_INDEX,
            sample_frame: PLAY_TEST_SAMPLE_START_FRAME,
            value: PlaybackSampleValue::Pcm8(PLAY_TEST_PCM8_FIRST_VALUE),
        }]
    );
    assert!(playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].sample_frame,
        PLAY_TEST_SECOND_SAMPLE_FRAME
    );

    assert_eq!(
        playback.step_samples(&module).unwrap(),
        vec![ChannelSampleFrame {
            channel: PLAY_TEST_CHANNEL_ZERO,
            sample_index: PLAY_TEST_FIRST_SAMPLE_INDEX,
            sample_frame: PLAY_TEST_SECOND_SAMPLE_FRAME,
            value: PlaybackSampleValue::Pcm8(PLAY_TEST_PCM8_SECOND_VALUE),
        }]
    );
    assert!(!playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].note,
        Note::Key(PLAY_TEST_CHANNEL_ZERO_NOTE)
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].sample_index,
        None
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].sample_frame,
        PLAY_TEST_SAMPLE_START_FRAME
    );
    assert!(playback.step_samples(&module).unwrap().is_empty());
}

#[test]
fn sample_step_reads_pcm16_frames_without_interpolation() {
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::Pcm16(vec![
        PLAY_TEST_PCM16_FIRST_VALUE,
        PLAY_TEST_PCM16_SECOND_VALUE,
    ]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback.step_samples(&module).unwrap(),
        vec![ChannelSampleFrame {
            channel: PLAY_TEST_CHANNEL_ZERO,
            sample_index: PLAY_TEST_FIRST_SAMPLE_INDEX,
            sample_frame: PLAY_TEST_SAMPLE_START_FRAME,
            value: PlaybackSampleValue::Pcm16(PLAY_TEST_PCM16_FIRST_VALUE),
        }]
    );
    assert_eq!(
        playback.step_samples(&module).unwrap(),
        vec![ChannelSampleFrame {
            channel: PLAY_TEST_CHANNEL_ZERO,
            sample_index: PLAY_TEST_FIRST_SAMPLE_INDEX,
            sample_frame: PLAY_TEST_SECOND_SAMPLE_FRAME,
            value: PlaybackSampleValue::Pcm16(PLAY_TEST_PCM16_SECOND_VALUE),
        }]
    );
}

#[test]
fn sample_step_releases_empty_sample_data_without_frame() {
    let module = module_with_two_channel_cells(
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
    let mut playback = PlaybackState::start(&module).unwrap();

    assert!(playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
    assert!(playback.step_samples(&module).unwrap().is_empty());
    assert!(!playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].note,
        Note::Key(PLAY_TEST_CHANNEL_ZERO_NOTE)
    );
    assert_eq!(
        playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].sample_index,
        None
    );
}

fn module_with_orders_and_pattern_rows(orders: Vec<u8>, rows: &[u16]) -> Module {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = orders;
    module.patterns = rows
        .iter()
        .map(|rows| Pattern::new(*rows, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS))
        .collect();

    module
}

fn module_with_two_channel_cells(rows: u16, cells: &[(u16, u16, PatternCell)]) -> Module {
    let mut module = Module::empty_with_channels(PLAY_TEST_TWO_CHANNELS).unwrap();
    module.orders = vec![PLAY_TEST_PATTERN_ZERO];
    let mut pattern = Pattern::new(rows, PLAY_TEST_TWO_CHANNELS, DEFAULT_EFFECT_SLOTS);

    for (channel, row, cell) in cells {
        pattern.set_cell(*channel, *row, cell.clone()).unwrap();
    }

    module.patterns = vec![pattern];
    module
}

fn test_cell(note: u8, instrument: u8) -> PatternCell {
    PatternCell {
        note: Note::Key(note),
        instrument,
        ..PatternCell::default()
    }
}

fn note_off_cell() -> PatternCell {
    PatternCell {
        note: Note::Off,
        ..PatternCell::default()
    }
}

fn note_only_cell(note: u8) -> PatternCell {
    PatternCell {
        note: Note::Key(note),
        ..PatternCell::default()
    }
}
