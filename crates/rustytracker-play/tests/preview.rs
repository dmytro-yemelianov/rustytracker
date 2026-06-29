use rustytracker_core::{FrequencyTable, Module, SampleData};
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PreviewVoice};

const PREVIEW_TEST_INSTRUMENT: u8 = 1;
const PREVIEW_TEST_NOTE: u8 = 49; // C-4
const PREVIEW_TEST_SAMPLE_RATE: u32 = 44_100;

fn module_with_preview_sample(data: SampleData) -> Module {
    let mut module = Module::empty_with_channels(2).unwrap();
    module.header.frequency_table = FrequencyTable::Linear;
    let map_len = module.instruments[0].note_sample_map.len().max(96);
    module.instruments[0].note_sample_map = vec![Some(0); map_len];
    module.samples[0].volume = 255;
    module.samples[0].data = data;
    module
}

#[test]
fn preview_voice_is_silent_before_note_on() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![1000; 64]));
    let mut voice = PreviewVoice::new();
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_note_on_produces_output() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    assert!(voice.is_active());
    let (l, r) = voice
        .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
        .unwrap();
    assert!(l != 0 || r != 0, "expected audible preview output");
}

#[test]
fn preview_voice_note_off_stops_voice() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    voice.note_off();
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_missing_instrument_stays_inactive() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000; 64]));
    let mut voice = PreviewVoice::new();
    let missing = (module.instruments.len() as u8) + 1;
    let result = voice.note_on(
        &module,
        missing,
        PREVIEW_TEST_NOTE,
        PlaybackSettings::default(),
    );
    assert!(result.is_err());
    assert!(!voice.is_active());
    assert_eq!(
        voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap(),
        (0, 0)
    );
}

#[test]
fn preview_voice_non_looping_sample_stops_after_end() {
    let module = module_with_preview_sample(SampleData::pcm16(vec![10_000, 9_000, 8_000, 7_000]));
    let mut voice = PreviewVoice::new();
    voice
        .note_on(
            &module,
            PREVIEW_TEST_INSTRUMENT,
            PREVIEW_TEST_NOTE,
            PlaybackSettings::default(),
        )
        .unwrap();
    assert!(voice.is_active());
    for _ in 0..200 {
        let _ = voice
            .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
            .unwrap();
    }
    assert!(
        !voice.is_active(),
        "non-looping preview should stop after the sample ends"
    );
}

#[test]
fn preview_voice_honors_mixer_mode() {
    let data: Vec<i16> = (0..256).map(|i| (i * 100) as i16).collect();
    let module = module_with_preview_sample(SampleData::pcm16(data));

    let render_n = |mode: PlaybackMixerMode| -> Vec<(i32, i32)> {
        let mut voice = PreviewVoice::new();
        voice
            .note_on(
                &module,
                PREVIEW_TEST_INSTRUMENT,
                PREVIEW_TEST_NOTE,
                PlaybackSettings::with_mixer_mode(mode),
            )
            .unwrap();
        (0..8)
            .map(|_| {
                voice
                    .render_stereo_frame(&module, PREVIEW_TEST_SAMPLE_RATE)
                    .unwrap()
            })
            .collect()
    };

    let hifi = render_n(PlaybackMixerMode::HiFi);
    let amiga = render_n(PlaybackMixerMode::Amiga);
    assert_ne!(
        hifi, amiga,
        "HiFi (interpolated) and Amiga (stepped) fetch should differ on a ramp sample"
    );
}
