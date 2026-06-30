use crate::*;

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
fn playback_state_defaults_to_hifi_mixer() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let playback = PlaybackState::start(&module).unwrap();

    assert_eq!(playback.settings().mixer_mode, PlaybackMixerMode::HiFi);
}

#[test]
fn raw_mono_render_mixer_mode_controls_sample_interpolation() {
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
    module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].data = SampleData::pcm16(vec![0, 1000]);

    let mut hifi_playback =
        PlaybackState::start_with_mixer_mode(&module, PlaybackMixerMode::HiFi).unwrap();
    let mut protracker_playback =
        PlaybackState::start_with_mixer_mode(&module, PlaybackMixerMode::ProTracker).unwrap();

    assert_eq!(
        hifi_playback
            .render_raw_mono_pcm(&module, 16_726, 2)
            .unwrap(),
        vec![0, 499]
    );
    assert_eq!(
        protracker_playback
            .render_raw_mono_pcm(&module, 16_726, 2)
            .unwrap(),
        vec![0, 0]
    );
}

#[test]
fn raw_mono_render_plays_pre_loop_frames_before_looping() {
    for loop_kind in [SampleLoopKind::Forward, SampleLoopKind::PingPong] {
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
            SampleData::pcm16(vec![1000, 2000, 3000, 4000, 5000]);
        module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].loop_start = 3;
        module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].loop_length = 2;
        module.samples[PLAY_TEST_FIRST_SAMPLE_INDEX].loop_kind = loop_kind;
        let mut playback = PlaybackState::start(&module).unwrap();

        assert_eq!(
            playback.render_raw_mono_pcm(&module, 8363, 5).unwrap(),
            vec![1000, 2000, 3000, 4000, 5000]
        );
    }
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
    assert_eq!(
        data_size / 4,
        (44100 * 5 * PLAY_TEST_DEFAULT_TICK_SPEED as u32) / (2 * PLAY_TEST_DEFAULT_BPM as u32) + 1
    );
}

#[test]
fn render_to_wav_uses_milkytracker_amiga_tick_clock() {
    let mut module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    module.header.frequency_table = FrequencyTable::Amiga;
    let mut playback = PlaybackState::start(&module).unwrap();
    let wav_bytes = playback.render_to_wav(&module, 44100).unwrap();

    let data_size = u32::from_le_bytes(wav_bytes[40..44].try_into().unwrap());
    let expected_tick_frames = (44100 / 250) * 5;
    assert_eq!(
        data_size / 4,
        expected_tick_frames * PLAY_TEST_DEFAULT_TICK_SPEED as u32 + 1
    );
}

#[test]
fn render_to_wav_rejects_zero_sample_rate() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback
            .render_to_wav(&module, PLAY_TEST_ZERO_SAMPLE_RATE)
            .unwrap_err(),
        PlaybackError::InvalidSampleRate {
            sample_rate: PLAY_TEST_ZERO_SAMPLE_RATE,
        }
    );
}

#[test]
fn raw_render_rejects_zero_sample_rate() {
    let module =
        module_with_orders_and_pattern_rows(vec![PLAY_TEST_PATTERN_ZERO], &[PLAY_TEST_ONE_ROW]);
    let mut playback = PlaybackState::start(&module).unwrap();

    assert_eq!(
        playback
            .render_raw_stereo_pcm(&module, PLAY_TEST_ZERO_SAMPLE_RATE, PLAY_TEST_RENDER_FRAMES)
            .unwrap_err(),
        PlaybackError::InvalidSampleRate {
            sample_rate: PLAY_TEST_ZERO_SAMPLE_RATE,
        }
    );
}
