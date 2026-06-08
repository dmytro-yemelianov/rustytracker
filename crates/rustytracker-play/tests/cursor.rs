use rustytracker_core::{Module, Pattern, DEFAULT_EFFECT_SLOTS};
use rustytracker_play::{
    PlaybackClock, PlaybackCursor, PlaybackError, PlaybackTiming, RowAdvance, TickAdvance,
    PLAYBACK_FIRST_ORDER_INDEX, PLAYBACK_FIRST_ROW, PLAYBACK_FIRST_TICK, PLAYBACK_ORDER_STEP,
    PLAYBACK_ROW_STEP, PLAYBACK_TICK_STEP,
};

const PLAY_TEST_CHANNELS: u16 = 1;
const PLAY_TEST_PATTERN_ZERO: u8 = 0;
const PLAY_TEST_PATTERN_ONE: u8 = 1;
const PLAY_TEST_FIRST_PATTERN_INDEX: usize = 0;
const PLAY_TEST_SECOND_PATTERN_INDEX: usize = 1;
const PLAY_TEST_ZERO_ROWS: u16 = 0;
const PLAY_TEST_ONE_ROW: u16 = 1;
const PLAY_TEST_TWO_ROWS: u16 = 2;
const PLAY_TEST_THREE_ROWS: u16 = 3;
const PLAY_TEST_DEFAULT_TICK_SPEED: u16 = 6;
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

fn module_with_orders_and_pattern_rows(orders: Vec<u8>, rows: &[u16]) -> Module {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = orders;
    module.patterns = rows
        .iter()
        .map(|rows| Pattern::new(*rows, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS))
        .collect();

    module
}
