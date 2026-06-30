use crate::*;

#[test]
fn writes_empty_module_header_and_order_table() {
    let mut module = Module::empty();
    module.header.title = ModuleTitle::new("empty writer test");
    module.orders = XM_WRITER_TEST_ORDERS.to_vec();

    let bytes = write_xm_header(&module).unwrap();
    let header = parse_xm_header(&bytes).unwrap();

    assert_eq!(header.title, "empty writer test");
    assert_eq!(header.version, XM_WRITER_TEST_VERSION);
    assert_eq!(header.header_size, XM_WRITER_TEST_HEADER_SIZE);
    assert_eq!(header.song_length, module.orders.len() as u16);
    assert_eq!(header.restart_position, module.header.restart_position);
    assert_eq!(header.channel_count, module.header.channel_count);
    assert_eq!(header.pattern_count, module.patterns.len() as u16);
    assert_eq!(header.instrument_count, module.instruments.len() as u16);
    assert_eq!(header.frequency_table, FrequencyTable::Linear);
    assert_eq!(header.default_tick_speed, module.header.tick_speed);
    assert_eq!(header.default_bpm, module.header.bpm);
    assert_eq!(header.orders, module.orders);
}

#[test]
fn roundtrips_bundled_fixture_headers_and_orders() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();

        let written = write_xm_header(&module).unwrap();
        let header = parse_xm_header(&written).unwrap();

        assert_eq!(header.title, module.header.title.as_str(), "{}", fixture);
        assert_eq!(
            header.channel_count, module.header.channel_count,
            "{}",
            fixture
        );
        assert_eq!(
            header.frequency_table, module.header.frequency_table,
            "{}",
            fixture
        );
        assert_eq!(
            header.default_tick_speed, module.header.tick_speed,
            "{}",
            fixture
        );
        assert_eq!(header.default_bpm, module.header.bpm, "{}", fixture);
        assert_eq!(
            header.restart_position, module.header.restart_position,
            "{}",
            fixture
        );
        assert_eq!(header.orders, module.orders, "{}", fixture);
        assert_eq!(
            header.pattern_count,
            module.patterns.len() as u16,
            "{}",
            fixture
        );
        assert_eq!(
            header.instrument_count,
            module.instruments.len() as u16,
            "{}",
            fixture
        );
    }
}

#[test]
fn roundtrips_bundled_fixtures_to_equivalent_core_modules() {
    if !fixtures_available() {
        return;
    }

    for fixture in FIXTURES {
        let bytes = fs::read(fixture_path(fixture)).unwrap();
        let module = parse_xm_module(&bytes).unwrap();
        let written = write_xm_module(&module).unwrap();
        let reparsed = parse_xm_module(&written).unwrap();

        assert_module_roundtrip_summary_eq(
            &module_roundtrip_summary(&reparsed),
            &module_roundtrip_summary(&module),
            fixture,
        );
    }
}

#[test]
fn roundtrips_synthetic_effect_cells_through_full_module_writer() {
    for (name, effects) in [
        (
            "effect-column arpeggio",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "effect-column extended command",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_EXTENDED_EFFECT,
                    XM_WRITER_EXTENDED_SOURCE_OPERAND,
                ),
            ],
        ),
        (
            "effect-column extra-fine portamento",
            [
                EffectCommand::default(),
                effect(
                    XM_WRITER_INTERNAL_EXTRA_FINE_PORTA_EFFECT,
                    XM_WRITER_EXTRA_FINE_PORTA_SOURCE_OPERAND,
                ),
            ],
        ),
        (
            "volume column set volume",
            [
                effect(XM_WRITER_VOLUME_EFFECT, XM_WRITER_FULL_VOLUME_255),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column volume slide up",
            [
                effect(
                    XM_WRITER_VOLUME_SLIDE_EFFECT,
                    XM_WRITER_VOLUME_SLIDE_UP_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column volume slide down",
            [
                effect(
                    XM_WRITER_VOLUME_SLIDE_EFFECT,
                    XM_WRITER_VOLUME_SLIDE_DOWN_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column fine volume slide up",
            [
                effect(
                    XM_WRITER_INTERNAL_EXTENDED_EFFECT,
                    XM_WRITER_FINE_VOLUME_SLIDE_UP_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column fine volume slide down",
            [
                effect(
                    XM_WRITER_INTERNAL_FINE_VOLUME_SLIDE_DOWN_EFFECT,
                    XM_WRITER_FINE_VOLUME_SLIDE_DOWN_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column vibrato speed",
            [
                effect(XM_WRITER_VIBRATO_EFFECT, XM_WRITER_VIBRATO_SPEED_OPERAND),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column vibrato depth",
            [
                effect(XM_WRITER_VIBRATO_EFFECT, XM_WRITER_VIBRATO_DEPTH_OPERAND),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning set",
            [
                effect(XM_WRITER_PANNING_EFFECT, XM_WRITER_CENTER_PANNING_255),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning slide left",
            [
                effect(
                    XM_WRITER_PANNING_SLIDE_EFFECT,
                    XM_WRITER_PANNING_SLIDE_LEFT_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column panning slide right",
            [
                effect(
                    XM_WRITER_PANNING_SLIDE_EFFECT,
                    XM_WRITER_PANNING_SLIDE_RIGHT_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
        (
            "volume column tone portamento high nibble",
            [
                effect(
                    XM_WRITER_TONE_PORTAMENTO_EFFECT,
                    XM_WRITER_HIGH_NIBBLE_TONE_PORTAMENTO_OPERAND,
                ),
                effect(
                    XM_WRITER_INTERNAL_ARPEGGIO_EFFECT,
                    XM_WRITER_ARPEGGIO_OPERAND,
                ),
            ],
        ),
    ] {
        let cell = roundtrip_single_cell_module(effects.to_vec());

        assert_eq!(cell.note, Note::Key(XM_WRITER_TEST_NOTE), "{}", name);
        assert_eq!(cell.instrument, XM_WRITER_TEST_INSTRUMENT, "{}", name);
        assert_eq!(cell.effects, effects.to_vec(), "{}", name);
    }
}

#[test]
fn rejects_order_tables_that_do_not_fit_in_xm_header() {
    let mut module = Module::empty();
    module.orders = vec![0; XM_WRITER_OVERLONG_ORDER_LEN];

    assert_eq!(
        write_xm_header(&module).unwrap_err(),
        XmWriteError::TooManyOrders {
            requested: XM_WRITER_OVERLONG_ORDER_LEN,
            maximum: XM_WRITER_MAX_ACTIVE_ORDERS,
        }
    );
}

#[test]
fn rejects_empty_order_tables() {
    let mut module = Module::empty();
    module.orders.clear();

    assert_eq!(
        write_xm_header(&module).unwrap_err(),
        XmWriteError::EmptyOrderList
    );
}
