use rustytracker_core::{
    EffectCommand, Envelope, EnvelopePoint, Module, Note, Pattern, PatternCell, SampleData,
    SampleLoopKind, DEFAULT_EFFECT_SLOTS,
};
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
const PLAY_TEST_SECOND_INSTRUMENT_INDEX: usize = 1;
const PLAY_TEST_FIRST_SAMPLE_INDEX: usize = 0;
const PLAY_TEST_SECOND_SAMPLE_INDEX: usize = 1;
const PLAY_TEST_SAMPLE_START_FRAME: usize = 0;
const PLAY_TEST_SECOND_SAMPLE_FRAME: usize = 1;
const PLAY_TEST_SAMPLE_VOLUME: u8 = 48;
const PLAY_TEST_SAMPLE_PANNING: u8 = 96;
const PLAY_TEST_MISSING_INSTRUMENT: u8 = 200;
const PLAY_TEST_PCM8_FIRST_VALUE: i8 = -2;
const PLAY_TEST_PCM8_SECOND_VALUE: i8 = 3;
const PLAY_TEST_PCM16_FIRST_VALUE: i16 = -512;
const PLAY_TEST_PCM16_SECOND_VALUE: i16 = 1024;
const PLAY_TEST_RENDER_FRAMES: usize = 3;
const PLAY_TEST_PCM8_FIRST_MONO: i32 = -512;
const PLAY_TEST_PCM16_HIGH_VALUE: i16 = 1024;
const PLAY_TEST_FIRST_MIXED_MONO: i32 = 512;
const PLAY_TEST_SILENCE_MONO: i32 = 0;

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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm8(vec![
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm16(vec![
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

#[test]
fn raw_mono_render_sums_pcm8_and_pcm16_steps_by_channel() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
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
                PLAY_TEST_CHANNEL_ONE,
                PLAYBACK_FIRST_ROW,
                test_cell(PLAY_TEST_CHANNEL_ONE_NOTE, PLAY_TEST_CHANNEL_ONE_INSTRUMENT),
            ),
        ],
    );
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm8(vec![
        PLAY_TEST_PCM8_FIRST_VALUE,
        PLAY_TEST_PCM8_SECOND_VALUE,
    ]);
    module.samples[PLAY_TEST_SECOND_SAMPLE_INDEX].data = SampleData::pcm16(vec![
        PLAY_TEST_PCM16_HIGH_VALUE,
        PLAY_TEST_PCM16_FIRST_VALUE,
    ]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback
            .render_raw_mono_pcm(&module, 8363, PLAY_TEST_RENDER_FRAMES)
            .unwrap(),
        vec![PLAY_TEST_FIRST_MIXED_MONO, 287, PLAY_TEST_SILENCE_MONO,]
    );
    assert!(!playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
    assert!(!playback.channels()[PLAY_TEST_CHANNEL_ONE as usize].active);
}

#[test]
fn raw_mono_render_returns_requested_silence_after_sample_end() {
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data =
        SampleData::pcm8(vec![PLAY_TEST_PCM8_FIRST_VALUE]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback
            .render_raw_mono_pcm(&module, 8363, PLAY_TEST_RENDER_FRAMES)
            .unwrap(),
        vec![
            PLAY_TEST_PCM8_FIRST_MONO,
            PLAY_TEST_SILENCE_MONO,
            PLAY_TEST_SILENCE_MONO,
        ]
    );
    assert!(!playback.channels()[PLAY_TEST_CHANNEL_ZERO as usize].active);
}

#[test]
fn raw_mono_render_advances_ticks_and_rows_based_on_sample_rate() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = PLAY_TEST_THREE_TICKS_PER_ROW; // 3 ticks per row
    module.header.bpm = PLAY_TEST_DEFAULT_BPM; // 125 BPM -> 20 ms per tick

    // We want to render at a sample rate of 50 Hz.
    // At 50 Hz, 1 sample is 20 ms, which matches 1 tick!
    // So 1 frame = 1 tick.
    // 3 ticks per row, 2 rows. Total song = 6 ticks.
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(playback.clock().tick(), 0);
    assert_eq!(playback.clock().position(&module).unwrap().row, 0);

    // Render 1 frame. This will be tick 0 of row 0.
    let _ = playback.render_raw_mono_pcm(&module, 50, 1).unwrap();
    // After rendering 1 frame, the fractional remainder becomes 0.
    assert_eq!(playback.clock().tick(), 0);
    assert_eq!(playback.clock().position(&module).unwrap().row, 0);

    // Render 1 more frame. This advances the clock to tick 1, and renders that frame.
    let _ = playback.render_raw_mono_pcm(&module, 50, 1).unwrap();
    assert_eq!(playback.clock().tick(), 1);
    assert_eq!(playback.clock().position(&module).unwrap().row, 0);

    // Render 1 more frame (total 3). This advances the clock to tick 2, and renders it.
    let _ = playback.render_raw_mono_pcm(&module, 50, 1).unwrap();
    assert_eq!(playback.clock().tick(), 2);
    assert_eq!(playback.clock().position(&module).unwrap().row, 0);

    // Render 1 more frame (total 4). This advances the clock to tick 0 of row 1!
    let _ = playback.render_raw_mono_pcm(&module, 50, 1).unwrap();
    assert_eq!(playback.clock().tick(), 0);
    assert_eq!(playback.clock().position(&module).unwrap().row, 1);
}

#[test]
fn test_effect_set_speed() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 6;

    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0f,
                operand: 3,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.clock().timing().ticks_per_row(), 3);

    // Tick 0 -> Tick 1
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::SameRow
    );
    assert_eq!(playback.clock().tick(), 1);

    // Tick 1 -> Tick 2
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::SameRow
    );
    assert_eq!(playback.clock().tick(), 2);

    // Tick 2 -> Row 1 (since speed is 3)
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    assert_eq!(playback.clock().tick(), 0);
    assert_eq!(playback.clock().position(&module).unwrap().row, 1);
}

#[test]
fn test_effect_set_bpm() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.bpm = 125;

    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0f,
                operand: 150,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.clock().timing().bpm(), 150);
    assert_eq!(
        playback.clock().timing().tick_duration_nanos(),
        2_500_000_000 / 150
    );
}

#[test]
fn test_effect_speed_zero_halts_playback() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);

    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0f,
                operand: 0,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::SongEnd
    );
}

#[test]
fn test_effect_set_volume() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let cell = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0c, // Set Volume
                operand: 100,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);
}

#[test]
fn test_effect_set_panning() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x08, // Set Panning
                operand: 200,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 200);
}

#[test]
fn vibrato_effect_memory_tracks_effect_slot_count() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![PLAY_TEST_PATTERN_ZERO];
    module.patterns = vec![Pattern::new(PLAY_TEST_ONE_ROW, PLAY_TEST_CHANNELS, 3)];
    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand::default(),
            EffectCommand {
                effect: 0x04,
                operand: 0x44,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    let channel = &playback.channels()[0];
    assert_eq!(channel.vibrato_speed.len(), 3);
    assert_eq!(channel.vibrato_depth.len(), 3);
    assert_eq!(channel.vibrato_pos.len(), 3);
    assert_eq!(channel.vibrato_speed[2], 4);
    assert_eq!(channel.vibrato_depth[2], 4);

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::SameRow
    );
}

#[test]
fn test_effect_volume_slide_up() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Set Volume to 100
    let cell_0 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0c,
                operand: 100,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Volume Slide Up by 3
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0a,  // Volume Slide
                operand: 0x30, // x=3, y=0 (slide up)
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);

    // Tick 0 -> Tick 1 of Row 0
    playback.advance_tick(&module).unwrap();
    // Tick 1 -> Tick 2 of Row 0
    playback.advance_tick(&module).unwrap();

    // Tick 2 -> Tick 0 of Row 1
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextRow
    );
    assert_eq!(playback.channels()[0].volume, 100); // No slide on tick 0

    // Tick 0 -> Tick 1 of Row 1
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 112); // 100 + 3*4 = 112

    // Tick 1 -> Tick 2 of Row 1
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 124); // 112 + 3*4 = 124
}

#[test]
fn test_effect_volume_slide_down() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Set Volume to 100
    let cell_0 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0c,
                operand: 100,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Volume Slide Down by 2
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0a,
                operand: 0x02, // x=0, y=2 (slide down)
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);

    // Tick 0 -> Tick 1 of Row 0
    playback.advance_tick(&module).unwrap();
    // Tick 1 -> Tick 2 of Row 0
    playback.advance_tick(&module).unwrap();

    // Tick 2 -> Tick 0 of Row 1
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);

    // Tick 0 -> Tick 1 of Row 1
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 92); // 100 - 2*4 = 92

    // Tick 1 -> Tick 2 of Row 1
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 84); // 92 - 2*4 = 84
}

#[test]
fn test_effect_fine_volume_slide() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Set Volume to 100
    let cell_0 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0c,
                operand: 100,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Fine Volume Slide Up by 5
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 58, // Fine Volume Slide Up (0x3a)
                operand: 5,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    // Row 2: Fine Volume Slide Down by 3
    let cell_2 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 59, // Fine Volume Slide Down (0x3b)
                operand: 3,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 2, cell_2).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);

    // Row 0 Tick 0 -> Tick 1 -> Tick 2
    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();

    // Row 0 Tick 2 -> Row 1 Tick 0: Fine slide up applied immediately!
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 120); // 100 + 5*4 = 120

    // Row 1 Tick 0 -> Tick 1 -> Tick 2: Volume does not change on ticks > 0
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 120);
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 120);

    // Row 1 Tick 2 -> Row 2 Tick 0: Fine slide down applied immediately!
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 108); // 120 - 3*4 = 108
}

#[test]
fn test_effect_position_jump() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![0, 1, 2];
    module.patterns = vec![
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 0 (2 rows)
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 1 (2 rows)
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 2 (2 rows)
    ];
    module.header.tick_speed = 1;

    let cell = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0b, // Position Jump
                operand: 2,   // to order 2
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 0);

    // Row 0 Tick 0 -> Row 0 Tick 0 of order 2 (since speed is 1, it advances to next row/order next tick)
    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextOrder
    );
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 2);
    assert_eq!(playback.clock().position(&module).unwrap().row, 0);
}

#[test]
fn test_effect_pattern_break() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![0, 1];
    module.patterns = vec![
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 0
        Pattern::new(15, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 1
    ];
    module.header.tick_speed = 1;

    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x0d,  // Pattern Break
                operand: 0x12, // BCD for 12 -> row 12
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 0);

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextOrder
    );
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 1);
    assert_eq!(playback.clock().position(&module).unwrap().row, 12);
}

#[test]
fn test_effect_position_jump_and_pattern_break() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![0, 1, 2];
    module.patterns = vec![
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 0
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 1
        Pattern::new(10, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS), // Pattern 2
    ];
    module.header.tick_speed = 1;

    // Both on Row 0
    let cell = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x0b, // Position Jump to order 2
                operand: 2,
            },
            EffectCommand {
                effect: 0x0d, // Pattern Break to row 8
                operand: 0x08,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 0);

    assert_eq!(
        playback.advance_tick(&module).unwrap(),
        TickAdvance::NextOrder
    );
    assert_eq!(playback.clock().position(&module).unwrap().order_index, 2);
    assert_eq!(playback.clock().position(&module).unwrap().row, 8);
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

fn map_instrument_to_sample(module: &mut Module, instrument_index: usize, sample_index: usize) {
    module.instruments[instrument_index].note_sample_map =
        vec![Some(sample_index); module.instruments[instrument_index].note_sample_map.len()];
}

#[test]
fn test_effect_arpeggio() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Note C-4 with Arpeggio 0x37 (offset 3 and 7)
    let cell_0 = PatternCell {
        note: Note::Key(49), // C-4
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x20, // Arpeggio (nonzero)
                operand: 0x37,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: No Note with Arpeggio 0x00 (operand 0) -> uses memory (0x37)
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x00, // Arpeggio (zero)
                operand: 0x00,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0 Tick 0
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608);
    assert_eq!(ch.period, 4608); // Tick 0 -> offset 0

    // Tick 0 -> Tick 1
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.period, 4608 - 3 * 64); // Tick 1 -> offset 3

    // Tick 1 -> Tick 2
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.period, 4608 - 7 * 64); // Tick 2 -> offset 7

    // Tick 2 -> Row 1 Tick 0
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.period, 4608); // Tick 0 -> offset 0

    // Tick 0 -> Tick 1 of Row 1
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.period, 4608 - 3 * 64); // Tick 1 -> offset 3 (from memory)

    // Tick 1 -> Tick 2 of Row 1
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.period, 4608 - 7 * 64); // Tick 2 -> offset 7 (from memory)
}

#[test]
fn test_effect_portamento_up_down() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Note C-4 with Portamento Up 0x01 operand 8
    let cell_0 = PatternCell {
        note: Note::Key(49), // C-4
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x01, // Portamento Up
                operand: 8,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Portamento Down 0x02 operand 6
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x02, // Portamento Down
                operand: 6,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    // Row 2: Portamento Up 0x01 operand 0 (uses memory, so speed 8)
    let cell_2 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x01,
                operand: 0,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 2, cell_2).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0 Tick 0
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608);
    assert_eq!(ch.period, 4608);

    // Row 0 Tick 1 (speed 8 * 4 = 32 units down)
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 32);
    assert_eq!(ch.period, 4608 - 32);

    // Row 0 Tick 2
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64);
    assert_eq!(ch.period, 4608 - 64);

    // Row 0 Tick 2 -> Row 1 Tick 0 (no slide on tick 0)
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64);

    // Row 1 Tick 1 (slide down, so period increases by 6 * 4 = 24)
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64 + 24);

    // Row 1 Tick 2
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64 + 48);

    // Row 1 Tick 2 -> Row 2 Tick 0
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64 + 48);

    // Row 2 Tick 1 (slide up using memory: speed 8 * 4 = 32)
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608 - 64 + 48 - 32);
}

#[test]
fn test_effect_tone_portamento() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Note C-4
    let cell_0 = PatternCell {
        note: Note::Key(49), // C-4 -> period 4608
        instrument: 1,
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Note C-5 with Tone Portamento 0x03 operand 10
    // C-5 is note 61 -> period 4608 - 12 * 64 = 3840
    let cell_1 = PatternCell {
        note: Note::Key(61),
        effects: vec![
            EffectCommand {
                effect: 0x03, // Tone Portamento
                operand: 10,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0 Tick 0
    assert_eq!(playback.channels()[0].base_period, 4608);

    // Row 0 Tick 1, Tick 2
    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();

    // Row 0 Tick 2 -> Row 1 Tick 0: Target period should be 3840, but base_period is still 4608 (no slide on tick 0)
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4608);
    assert_eq!(ch.target_period, 3840);
    assert!(ch.active); // Note was not stopped, sample frame not reset (we don't check sample_frame directly but it remains active)

    // Row 1 Tick 1: slide towards target by 10 * 4 = 40.
    // 4608 - 40 = 4568
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4568);

    // Row 1 Tick 2: slide towards target by 40.
    // 4568 - 40 = 4528
    playback.advance_tick(&module).unwrap();
    let ch = &playback.channels()[0];
    assert_eq!(ch.base_period, 4528);
}

#[test]
fn test_effect_vibrato() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Note C-4 with Vibrato 0x04 speed 4, depth 2
    let cell_0 = PatternCell {
        note: Note::Key(49), // C-4 -> period 4608
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x04,  // Vibrato
                operand: 0x42, // speed 4, depth 2
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Tick 0: vibpos = 0, VIB_TAB[0] = 0 -> period = 4608
    assert_eq!(playback.channels()[0].period, 4608);

    // Tick 1: vibpos = 0 (incremented to 4 after calculation), VIB_TAB[0] = 0 -> period = 4608
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608);

    // Tick 2: vibpos = 4 (incremented to 8 after calculation), VIB_TAB[4] = 97 -> vm = (97 * 2) >> 5 = 6 -> period = 4608 + 6 = 4614
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4614);
}

#[test]
fn test_effect_vibrato_volume_slide() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 3;

    // Row 0: Note C-4 with Vibrato 0x04 speed 4, depth 2, Volume 100
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x0c, // Set Volume
                operand: 100,
            },
            EffectCommand {
                effect: 0x04, // Vibrato speed 4, depth 2
                operand: 0x42,
            },
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Vibrato + Volume Slide 0x06 (slide up by 3)
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: 0x06,  // Vibrato + Volume Slide
                operand: 0x30, // slide up by 3 (operand 0x30 -> x=3, y=0)
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0 Tick 0
    assert_eq!(playback.channels()[0].volume, 100);
    assert_eq!(playback.channels()[0].period, 4608);

    // Row 0 Tick 1
    playback.advance_tick(&module).unwrap();
    // Row 0 Tick 2 (vibpos is 4 here, so period 4614)
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4614);

    // Row 0 Tick 2 -> Row 1 Tick 0: vibpos is 8. Volume should not change on tick 0.
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 100);
    // On tick 0, vibpos is 8, VIB_TAB[8] = 180 -> vm = (180 * 2) >> 5 = 11.
    assert_eq!(playback.channels()[0].period, 4608 + 11);

    // Row 1 Tick 1: volume slides up by 3 * 4 = 12 -> 112.
    // vibpos is 8. vm is 11. After calculation, vibpos increments to 12.
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 112);
    assert_eq!(playback.channels()[0].period, 4608 + 11);
}

#[test]
fn test_effect_sample_offset() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 1;

    // Row 0: Note C-4 with Sample Offset 0x09 operand 2 -> start at 512
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x09, // Sample Offset
                operand: 2,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Note C-4 with Sample Offset 0x09 operand 0 -> uses memory (start at 512)
    let cell_1 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x09,
                operand: 0,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    // Row 2: Note C-4 with Sample Offset 0x09 operand 5 -> start at 1280 (exceeds sample length, so stops)
    let cell_2 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x09,
                operand: 5,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 2, cell_2).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    // Give sample 1000 frames
    module.samples[0].data = SampleData::pcm8(vec![0; 1000]);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0 Tick 0 (since speed is 1, starting row 0 immediately initializes it to 512)
    assert!(playback.channels()[0].active);
    assert_eq!(playback.channels()[0].sample_frame, 512);

    // Row 0 -> Row 1 Tick 0
    playback.advance_tick(&module).unwrap();
    assert!(playback.channels()[0].active);
    assert_eq!(playback.channels()[0].sample_frame, 512);

    // Row 1 -> Row 2 Tick 0
    playback.advance_tick(&module).unwrap();
    // Exceeded length, should be inactive/stopped
    assert!(!playback.channels()[0].active);
}

#[test]
fn test_forward_loop() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = 1;

    let cell = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();
    map_instrument_to_sample(&mut module, 0, 0);

    // Give sample 6 frames, loop_start = 2, loop_length = 3 (loop_end = 5)
    module.samples[0].data = SampleData::pcm8(vec![10, 11, 12, 13, 14, 15]);
    module.samples[0].loop_start = 2;
    module.samples[0].loop_length = 3;
    module.samples[0].loop_kind = SampleLoopKind::Forward;

    let mut playback = PlaybackState::start(&module).unwrap();

    // Frame sequence should be: 0, 1, 2, 3, 4, 2, 3, 4, 2...
    let expected_frames = vec![0, 1, 2, 3, 4, 2, 3, 4, 2, 3, 4];
    for &expected in &expected_frames {
        assert!(playback.channels()[0].active);
        let frames = playback.step_samples(&module).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].sample_frame, expected);
    }
}

#[test]
fn test_ping_pong_loop() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = 1;

    let cell = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();
    map_instrument_to_sample(&mut module, 0, 0);

    // Give sample 6 frames, loop_start = 2, loop_length = 3 (loop_end = 5)
    module.samples[0].data = SampleData::pcm8(vec![10, 11, 12, 13, 14, 15]);
    module.samples[0].loop_start = 2;
    module.samples[0].loop_length = 3;
    module.samples[0].loop_kind = SampleLoopKind::PingPong;

    let mut playback = PlaybackState::start(&module).unwrap();

    // Frame sequence should be: 0, 1, 2, 3, 4, 3, 2, 3, 4, 3, 2...
    let expected_frames = vec![0, 1, 2, 3, 4, 3, 2, 3, 4, 3, 2];
    for &expected in &expected_frames {
        let frames = playback.step_samples(&module).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].sample_frame, expected);
    }
}

#[test]
fn test_volume_envelope_and_fadeout() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 5; // 5 ticks per row

    // Row 0: Note C-4 Instrument 1
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    // Row 1: Note Off
    let cell_1 = PatternCell {
        note: Note::Off,
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    map_instrument_to_sample(&mut module, 0, 0);

    // Set sample data
    module.samples[0].data = SampleData::pcm8(vec![0; 100]);

    // Setup volume envelope:
    // Point 0: frame 0, value 256
    // Point 1: frame 2, value 128 (Sustain Point)
    // Point 2: frame 5, value 0
    module.instruments[0].volume_envelope = Envelope {
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
        flags: 0x01 | 0x02, // On | Sustain
    };

    // Setup panning envelope:
    // Point 0: frame 0, value 128
    // Point 1: frame 4, value 256
    module.instruments[0].panning_envelope = Envelope {
        points: vec![
            EnvelopePoint {
                frame: 0,
                value: 128,
            },
            EnvelopePoint {
                frame: 4,
                value: 256,
            },
        ],
        point_count: 2,
        sustain_point: 0,
        loop_start_point: 0,
        loop_end_point: 0,
        flags: 0x01, // On
    };

    // Fadeout = 16384 (1/4 of 65536)
    module.instruments[0].volume_fadeout = 16384;

    let mut playback = PlaybackState::start(&module).unwrap();

    // --- Row 0 Tick 0 ---
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        assert!(ch.keyon);
        assert_eq!(ch.volume_envelope_val, 256);
        assert_eq!(ch.panning_envelope_val, 128);
        assert_eq!(ch.fadeout_volume, 65536);
    }

    // --- Row 0 Tick 1 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        // Interpolated between 256 (frame 0) and 128 (frame 2) at frame 1 -> 192
        assert_eq!(ch.volume_envelope_val, 192);
        // Interpolated between 128 (frame 0) and 256 (frame 4) at frame 1 -> 128 + 32 = 160
        assert_eq!(ch.panning_envelope_val, 160);
        assert_eq!(ch.fadeout_volume, 65536);
    }

    // --- Row 0 Tick 2 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        assert_eq!(ch.volume_envelope_val, 128); // Sustain point reached
        assert_eq!(ch.panning_envelope_val, 192); // 128 + 64 = 192
        assert_eq!(ch.fadeout_volume, 65536);
    }

    // --- Row 0 Tick 3 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        assert_eq!(ch.volume_envelope_val, 128); // Sustain point holds
        assert_eq!(ch.panning_envelope_val, 224); // 128 + 96 = 224
        assert_eq!(ch.fadeout_volume, 65536);
    }

    // --- Row 0 Tick 4 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        assert_eq!(ch.volume_envelope_val, 128); // Sustain point holds
        assert_eq!(ch.panning_envelope_val, 256); // end point reached
        assert_eq!(ch.fadeout_volume, 65536);
    }

    // --- Row 1 Tick 0 (Note Off triggers) ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active); // Envelope keeps channel active
        assert!(!ch.keyon); // keyon is false now
                            // Read before advance: still at step 2 -> 128
        assert_eq!(ch.volume_envelope_val, 128);
        assert_eq!(ch.panning_envelope_val, 256); // remains at last point
                                                  // fadeout volume starts decreasing: 65536 - 16384 = 49152
        assert_eq!(ch.fadeout_volume, 49152);
    }

    // --- Row 1 Tick 1 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        // step advanced to 3 at end of previous tick. Interpolated value -> 128 * (5-3)/3 = 85
        assert_eq!(ch.volume_envelope_val, 85);
        assert_eq!(ch.fadeout_volume, 32768);
    }

    // --- Row 1 Tick 2 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        assert!(ch.active);
        // step advanced to 4. Interpolated value -> 128 * (5-4)/3 = 42
        assert_eq!(ch.volume_envelope_val, 42);
        assert_eq!(ch.fadeout_volume, 16384);
    }

    // --- Row 1 Tick 3 ---
    playback.advance_tick(&module).unwrap();
    {
        let ch = &playback.channels()[0];
        // step advanced to 5. volume envelope is 0 -> deactivates channel!
        assert!(!ch.active);
    }
}

#[test]
fn test_raw_stereo_render_with_panning() {
    let mut module = module_with_two_channel_cells(
        PLAY_TEST_ONE_ROW,
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
                PLAY_TEST_CHANNEL_ONE,
                PLAYBACK_FIRST_ROW,
                test_cell(PLAY_TEST_CHANNEL_ONE_NOTE, PLAY_TEST_CHANNEL_ONE_INSTRUMENT),
            ),
        ],
    );
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

    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm16(vec![1000]);
    module.samples[PLAY_TEST_SECOND_SAMPLE_INDEX].data = SampleData::pcm16(vec![1000]);

    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].panning = 64;
    module.samples[PLAY_TEST_SECOND_SAMPLE_INDEX].panning = 192;

    let mut playback = PlaybackState::start(&module).unwrap();

    let frames = playback.render_raw_stereo_pcm(&module, 8363, 1).unwrap();
    assert_eq!(frames.len(), 1);

    let (left, right) = frames[0];
    assert_eq!(left, 996);
    assert_eq!(right, 1003);
}

#[test]
fn test_render_to_wav() {
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
    map_instrument_to_sample(
        &mut module,
        PLAY_TEST_FIRST_INSTRUMENT_INDEX,
        PLAY_TEST_FIRST_SAMPLE_INDEX,
    );
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm16(vec![100; 100]);

    let mut playback = PlaybackState::start(&module).unwrap();
    let wav_bytes = playback.render_to_wav(&module, 44100).unwrap();

    assert!(wav_bytes.len() >= 44);
    assert_eq!(&wav_bytes[0..4], b"RIFF");
    assert_eq!(&wav_bytes[8..12], b"WAVE");
    assert_eq!(&wav_bytes[12..16], b"fmt ");
    assert_eq!(&wav_bytes[36..40], b"data");

    // Audio format = 1
    assert_eq!(u16::from_le_bytes([wav_bytes[20], wav_bytes[21]]), 1);
    // Num channels = 2
    assert_eq!(u16::from_le_bytes([wav_bytes[22], wav_bytes[23]]), 2);
    // Sample rate = 44100
    assert_eq!(
        u32::from_le_bytes([wav_bytes[24], wav_bytes[25], wav_bytes[26], wav_bytes[27]]),
        44100
    );
    // Bits per sample = 16
    assert_eq!(u16::from_le_bytes([wav_bytes[34], wav_bytes[35]]), 16);

    let data_size = u32::from_le_bytes(wav_bytes[40..44].try_into().unwrap());
    let file_size = u32::from_le_bytes(wav_bytes[4..8].try_into().unwrap());
    assert_eq!(file_size, data_size + 36);
    assert_eq!(wav_bytes.len(), data_size as usize + 44);
}
