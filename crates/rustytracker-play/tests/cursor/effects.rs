use crate::*;

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
fn position_jump_target_uses_order_pattern_index() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![0, 2, 1];
    module.patterns = vec![
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
    ];
    module.header.tick_speed = 1;

    let cell = PatternCell {
        effects: vec![
            EffectCommand {
                effect: EFFECT_POSITION_JUMP,
                operand: 1,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let playback = PlaybackState::start(&module).unwrap();
    let target = playback.clock().jump_target().unwrap();

    assert_eq!(target.order_index, 1);
    assert_eq!(target.pattern_index, 2);
    assert_eq!(target.row, 0);
}

#[test]
fn pattern_break_target_uses_next_order_pattern_index() {
    let mut module = Module::empty_with_channels(PLAY_TEST_CHANNELS).unwrap();
    module.orders = vec![0, 2, 1];
    module.patterns = vec![
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
        Pattern::new(2, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
        Pattern::new(4, PLAY_TEST_CHANNELS, DEFAULT_EFFECT_SLOTS),
    ];
    module.header.tick_speed = 1;

    let cell = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: EFFECT_PATTERN_BREAK,
                operand: 0x01,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let playback = PlaybackState::start(&module).unwrap();
    let target = playback.clock().jump_target().unwrap();

    assert_eq!(target.order_index, 1);
    assert_eq!(target.pattern_index, 2);
    assert_eq!(target.row, 1);
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

    // Row 1: No Note with explicit Arpeggio 0x00 -> uses memory (0x37)
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: 0x20, // Explicit arpeggio, displayed/written as 000
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
fn raw_zero_effect_nonzero_operand_applies_arpeggio() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = 3;

    let cell = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: EFFECT_ARPEGGIO_ZERO,
                operand: 0x37,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();
    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(playback.channels()[0].period, 4608);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608 - 3 * 64);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608 - 7 * 64);
}

#[test]
fn default_effect_slots_do_not_reuse_arpeggio_memory() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;

    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x20,
                operand: 0x37,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();
    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608 - 3 * 64);

    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].period, 4608);
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
fn test_effect_note_cut() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;
    module.samples[0].volume = 64;

    // Row 0: Note C-4 with note-cut at tick 2 (EC2 -> internal 0x3c / operand 0x02)
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x3c,
                operand: 0x02,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();
    map_instrument_to_sample(&mut module, 0, 0);

    let mut playback = PlaybackState::start(&module).unwrap();

    // Tick 0: sample volume applied, not yet cut.
    assert_eq!(playback.channels()[0].volume, 64);
    // Tick 1: still not cut.
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 64);
    // Tick 2: cut -> volume 0.
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].volume, 0);
}

#[test]
fn test_effect_glissando_control() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;
    module.samples[0].volume = 64;
    map_instrument_to_sample(&mut module, 0, 0);

    // Row 0: Note C-4 (49)
    // Row 1: Note D-4 (51) with tone portamento speed 4 (0x03, 4) and glissando control E31 (0x33, 1)
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![EffectCommand::default(), EffectCommand::default()],
    };
    let cell_1 = PatternCell {
        note: Note::Key(51),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x03, // Tone Portamento
                operand: 4,   // 4 * 4 = 16 period units per tick
            },
            EffectCommand {
                effect: 0x33, // Glissando Control (E31)
                operand: 1,
            },
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();

    // Row 0:
    // Tick 0
    let period_c4 = playback.channels()[0].base_period;
    assert_ne!(period_c4, 0);

    // Advance to Row 1 Tick 0
    playback.advance_tick(&module).unwrap(); // Tick 1
    playback.advance_tick(&module).unwrap(); // Tick 2
    playback.advance_tick(&module).unwrap(); // Row 1 Tick 0

    // Row 1 Tick 0 is a note trigger, base_period is still period_c4.
    let target_period = playback.channels()[0].target_period;
    assert_ne!(target_period, 0);
    assert_ne!(target_period, period_c4);

    // Row 1 Tick 1: slide is processed (16 units closer to target).
    playback.advance_tick(&module).unwrap();

    let base_period_tick1 = playback.channels()[0].base_period;
    let period_tick1 = playback.channels()[0].period;
    assert_eq!(base_period_tick1, period_c4 - 16);
    // Nearest semitone period to (period_c4 - 16) is period_c4 since (period_c4 - 16) is closer to period_c4 than C#4 (period_c4 - 64).
    assert_eq!(period_tick1, period_c4);

    // Row 1 Tick 2: slide by another 16 units (total 32 units).
    playback.advance_tick(&module).unwrap();
    let base_period_tick2 = playback.channels()[0].base_period;
    let period_tick2 = playback.channels()[0].period;
    assert_eq!(base_period_tick2, period_c4 - 32);
    // Since 32 is exactly halfway or just check that it rounded.
    assert!(period_tick2 == period_c4 || period_tick2 == period_c4 - 64);
}

#[test]
fn test_effect_vibrato_control() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;
    module.samples[0].volume = 64;
    map_instrument_to_sample(&mut module, 0, 0);

    // Row 0: Note C-4 with Vibrato (0x04, 0x44) and E42 (Square wave -> 0x34, 0x02)
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x04,  // Vibrato
                operand: 0x44, // Speed 4, depth 4
            },
            EffectCommand {
                effect: 0x34,  // Vibrato Control (E42)
                operand: 0x02, // Square wave
            },
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();

    let base_period = playback.channels()[0].base_period;

    // Tick 0: Square wave vibrato at pos 0 is positive (offset (255 * 4) >> 5 = 31)
    let period_tick0 = playback.channels()[0].period;
    assert_eq!(period_tick0, (base_period as i32 + 31) as u32);

    // Tick 1: phase advanced by speed 4 -> pos is 4 (still in positive half, < 32)
    playback.advance_tick(&module).unwrap();
    let period_tick1 = playback.channels()[0].period;
    assert_eq!(period_tick1, (base_period as i32 + 31) as u32);

    // Now test continuous (no-retrigger) waveform E46 (Square, no retrig -> 0x34, 0x06)
    let cell_1 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x34,  // Vibrato Control (E46)
                operand: 0x06, // Square, no retrigger
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();

    // Advance to Row 1 Tick 0 (triggering new note)
    playback.advance_tick(&module).unwrap(); // Tick 2

    playback.advance_tick(&module).unwrap(); // Row 1 Tick 0
    let pos_after = playback.channels()[0].vibrato_pos[0];
    assert_ne!(pos_after, 0);
}

#[test]
fn test_effect_tremolo_control() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_TWO_ROWS]);
    module.header.tick_speed = 3;
    module.samples[0].volume = 64;
    map_instrument_to_sample(&mut module, 0, 0);

    // Row 0: Note C-4 with Tremolo (0x07, 0x44) and E72 (Square wave -> 0x37, 0x02)
    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x07,  // Tremolo
                operand: 0x44, // Speed 4, depth 4
            },
            EffectCommand {
                effect: 0x37,  // Tremolo Control (E72)
                operand: 0x02, // Square wave
            },
        ],
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();

    // modulated volume on tick 0 should be 64 + 31 = 95
    let vol_tick0 = playback.channels()[0].volume;
    assert_eq!(vol_tick0, 64 + 31);

    // Tick 1: phase advanced by speed 4 -> pos is 4 (still in positive half, < 32)
    playback.advance_tick(&module).unwrap();
    let vol_tick1 = playback.channels()[0].volume;
    assert_eq!(vol_tick1, 64 + 31);
}

#[test]
fn test_effect_panning_slide_with_memory() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 3;
    map_instrument_to_sample(&mut module, 0, 0);

    let cell_0 = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: 0x08,
                operand: 100,
            },
            EffectCommand::default(),
        ],
    };
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: EFFECT_PANNING_SLIDE,
                operand: 0x30,
            },
        ],
        ..PatternCell::default()
    };
    let cell_2 = PatternCell {
        effects: vec![
            EffectCommand::default(),
            EffectCommand {
                effect: EFFECT_PANNING_SLIDE,
                operand: 0x00,
            },
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();
    module.patterns[0].set_cell(0, 2, cell_2).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 100);

    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 100);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 103);
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 106);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 106);
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].panning, 109);
}

#[test]
fn test_effect_tremor_mutes_output_volume_without_changing_base_volume() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.tick_speed = 5;
    module.samples[0].volume = 64;
    map_instrument_to_sample(&mut module, 0, 0);

    let cell = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: EFFECT_TREMOR,
                operand: 0x11,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.channels()[0].base_volume, 64);
    assert_eq!(playback.channels()[0].volume, 64);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].base_volume, 64);
    assert_eq!(playback.channels()[0].volume, 64);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].base_volume, 64);
    assert_eq!(playback.channels()[0].volume, 0);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].base_volume, 64);
    assert_eq!(playback.channels()[0].volume, 0);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.channels()[0].base_volume, 64);
    assert_eq!(playback.channels()[0].volume, 64);
}

#[test]
fn test_effect_global_volume_scales_rendered_output() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.samples[0].data = SampleData::pcm16(vec![1000; 8]);
    module.samples[0].volume = 255;
    map_instrument_to_sample(&mut module, 0, 0);

    let cell = PatternCell {
        note: Note::Key(49),
        instrument: 1,
        effects: vec![
            EffectCommand {
                effect: EFFECT_GLOBAL_VOLUME,
                operand: 128,
            },
            EffectCommand::default(),
        ],
    };
    module.patterns[0].set_cell(0, 0, cell).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.global_volume(), 128);
    assert_eq!(
        playback.render_raw_mono_pcm(&module, 8363, 1).unwrap(),
        vec![501]
    );
}

#[test]
fn test_effect_global_volume_slide_with_memory() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_THREE_ROWS]);
    module.header.tick_speed = 3;

    let cell_0 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: EFFECT_GLOBAL_VOLUME,
                operand: 128,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    let cell_1 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: EFFECT_GLOBAL_VOLUME_SLIDE,
                operand: 0x20,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    let cell_2 = PatternCell {
        effects: vec![
            EffectCommand {
                effect: EFFECT_GLOBAL_VOLUME_SLIDE,
                operand: 0x00,
            },
            EffectCommand::default(),
        ],
        ..PatternCell::default()
    };
    module.patterns[0].set_cell(0, 0, cell_0).unwrap();
    module.patterns[0].set_cell(0, 1, cell_1).unwrap();
    module.patterns[0].set_cell(0, 2, cell_2).unwrap();

    let mut playback = PlaybackState::start(&module).unwrap();
    assert_eq!(playback.global_volume(), 128);

    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.global_volume(), 128);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.global_volume(), 136);
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.global_volume(), 144);

    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.global_volume(), 144);
    playback.advance_tick(&module).unwrap();
    assert_eq!(playback.global_volume(), 152);
}
