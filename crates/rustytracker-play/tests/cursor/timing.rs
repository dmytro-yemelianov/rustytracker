use crate::*;

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
