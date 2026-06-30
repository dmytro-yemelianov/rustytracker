use crate::*;

#[test]
fn crate_root_re_exports_channel_api() {
    let _ = core::mem::size_of::<PlaybackChannelState>();
    let _ = core::mem::size_of::<PlaybackEnvelopeState>();
    let _ = core::mem::size_of::<PlaybackSampleValue>();
    let _ = core::mem::size_of::<RawMonoPcmFrame>();
    let _ = core::mem::size_of::<RawStereoPcmFrame>();

    assert_eq!(EFFECT_ARPEGGIO_ZERO, 0x00);
    assert_eq!(EFFECT_TONE_PORTAMENTO, 0x03);
    assert_eq!(EFFECT_VOLUME_SLIDE, 0x0a);
    assert_eq!(EFFECT_POSITION_JUMP, 0x0b);
    assert_eq!(EFFECT_PATTERN_BREAK, 0x0d);
    assert_eq!(EFFECT_SET_SPEED_BPM, 0x0f);
    assert_eq!(SPEED_BPM_THRESHOLD, 32);
    assert_eq!(PLAYBACK_MONO_SILENCE, 0);
    assert_eq!(PLAYBACK_STEREO_SILENCE, (0, 0));
    assert_eq!(VIB_TAB.len(), 32);
}

#[test]
fn envelope_value_clamps_before_first_point() {
    let envelope = Envelope {
        points: vec![
            EnvelopePoint {
                frame: 4,
                value: 200,
            },
            EnvelopePoint {
                frame: 8,
                value: 100,
            },
        ],
        point_count: 2,
        sustain_point: 0,
        loop_start_point: 0,
        loop_end_point: 0,
        flags: PLAY_TEST_ENVELOPE_ENABLED_FLAG,
    };

    let state = PlaybackEnvelopeState::new();

    assert_eq!(state.get_value(&envelope, 256), 200);
}
