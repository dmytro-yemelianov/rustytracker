use crate::*;

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
