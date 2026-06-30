use crate::*;

#[test]
fn writes_internal_arpeggio_back_to_xm_effect_zero() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND
        )
    );
}

#[test]
fn writes_internal_extended_effects_back_to_xm_e_commands() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_SOURCE_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_EXTENDED_SOURCE_OPERAND
        )
    );
}

#[test]
fn does_not_relocate_mixed_direction_volume_slide_to_volume_column() {
    let bytes = write_single_cell_pattern(vec![
        effect(
            XM_WRITER_VOLUME_SLIDE_EFFECT,
            XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
        ),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_VOLUME_SLIDE_EFFECT,
            XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects,
        vec![
            EffectCommand::default(),
            effect(
                XM_WRITER_VOLUME_SLIDE_EFFECT,
                XM_WRITER_MIXED_VOLUME_SLIDE_OPERAND,
            ),
        ]
    );
}

#[test]
fn writes_internal_fine_volume_slides_to_xm_volume_column_when_effect_column_is_needed() {
    for (fine_effect, fine_operand, volume_column) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_COLUMN,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_COLUMN,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(fine_effect, fine_operand),
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                volume_column,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "fine slide {:#04x}/{:#04x}",
            fine_effect,
            fine_operand
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                effect(fine_effect, fine_operand),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "fine slide {:#04x}/{:#04x}",
            fine_effect,
            fine_operand
        );
    }
}

#[test]
fn does_not_backfill_internal_fine_volume_slides_from_later_effect_slots() {
    for (fine_effect, fine_operand) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
            effect(fine_effect, fine_operand),
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "fine slide {:#04x}/{:#04x}",
            fine_effect,
            fine_operand
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "fine slide {:#04x}/{:#04x}",
            fine_effect,
            fine_operand
        );
    }
}

#[test]
fn writes_zero_operand_internal_fine_volume_slides_to_effect_column_for_roundtrip_symmetry() {
    for (fine_effect, extended_operand) in [
        (
            XM_WRITER_INTERNAL_EXTENDED_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_UP_EXTENDED_OPERAND,
        ),
        (
            XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
            XM_WRITER_FINE_VOLUME_SLIDE_DOWN_EXTENDED_OPERAND,
        ),
    ] {
        let bytes = write_single_cell_pattern(vec![effect(
            fine_effect,
            XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND,
        )]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_EXTENDED_EFFECT,
                extended_operand,
            ],
            "fine slide {:#04x}",
            fine_effect
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects[1],
            effect(fine_effect, XM_WRITER_FINE_VOLUME_SLIDE_EMPTY_OPERAND),
            "fine slide {:#04x}",
            fine_effect
        );
    }
}

#[test]
fn writes_internal_extra_fine_portamento_back_to_xm_21() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(
            XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_XM_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(
            XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
            XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
        )
    );
}

#[test]
fn writes_full_scale_core_volume_back_to_xm_volume_operand() {
    let bytes = write_single_cell_pattern(vec![
        EffectCommand::default(),
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
            XM_WRITER_VOLUME_EFFECT,
            XM_WRITER_FULL_VOLUME_64,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[1],
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255)
    );
}

#[test]
fn writes_relocatable_first_effect_to_volume_column_when_effect_column_is_needed() {
    let bytes = write_single_cell_pattern(vec![
        effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
        effect(
            XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_FULL_VOLUME_COLUMN,
            XM_WRITER_XM_ARPEGGIO_EFFECT,
            XM_WRITER_ARPEGGIO_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects,
        vec![
            effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND
            ),
        ]
    );
}

#[test]
fn writes_first_panning_effect_to_volume_column() {
    let bytes = write_single_cell_pattern(vec![
        effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255),
        EffectCommand::default(),
    ]);

    assert_eq!(
        first_raw_pattern_cell(&bytes),
        &[
            XM_WRITER_TEST_NOTE,
            XM_WRITER_TEST_INSTRUMENT,
            XM_WRITER_CENTER_PANNING_COLUMN,
            XM_WRITER_EMPTY_EFFECT,
            XM_WRITER_EMPTY_OPERAND,
        ]
    );
    assert_eq!(
        first_decoded_cell(&bytes).effects[0],
        effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255)
    );
}

#[test]
fn writes_zero_operand_slides_to_effect_column_for_roundtrip_symmetry() {
    for slide_effect in [
        XM_WRITER_VOLUME_SLIDE_EFFECT,
        XM_WRITER_PANNING_SLIDE_EFFECT,
    ] {
        let bytes = write_single_cell_pattern(vec![effect(slide_effect, XM_WRITER_EMPTY_OPERAND)]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                slide_effect,
                XM_WRITER_EMPTY_OPERAND,
            ],
            "slide effect {:#04x}",
            slide_effect
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects[1],
            effect(slide_effect, XM_WRITER_EMPTY_OPERAND),
            "slide effect {:#04x}",
            slide_effect
        );
    }
}

#[test]
fn does_not_relocate_lossy_effects_to_volume_column_when_effect_column_is_occupied() {
    for lossy_effect in [
        effect(
            XM_WRITER_TONE_PORTAMENTO_EFFECT,
            XM_WRITER_LOW_NIBBLE_TONE_PORTAMENTO_OPERAND,
        ),
        effect(XM_WRITER_VOLUME_SLIDE_EFFECT, XM_WRITER_EMPTY_OPERAND),
        effect(XM_WRITER_PANNING_SLIDE_EFFECT, XM_WRITER_EMPTY_OPERAND),
    ] {
        let bytes = write_single_cell_pattern(vec![
            effect(
                XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ),
            lossy_effect,
        ]);

        assert_eq!(
            first_raw_pattern_cell(&bytes),
            &[
                XM_WRITER_TEST_NOTE,
                XM_WRITER_TEST_INSTRUMENT,
                XM_WRITER_TEST_EMPTY_VOLUME_COLUMN,
                XM_WRITER_XM_ARPEGGIO_EFFECT,
                XM_WRITER_ARPEGGIO_OPERAND,
            ],
            "lossy effect {:#04x}/{:#04x}",
            lossy_effect.effect,
            lossy_effect.operand
        );
        assert_eq!(
            first_decoded_cell(&bytes).effects,
            vec![
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
            "lossy effect {:#04x}/{:#04x}",
            lossy_effect.effect,
            lossy_effect.operand
        );
    }
}
