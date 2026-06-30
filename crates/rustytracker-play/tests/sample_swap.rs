use rustytracker_core::{
    FrequencyTable, Instrument, Module, Note, NoteName, Pattern, PatternCell, Sample, SampleData,
    DEFAULT_EFFECT_SLOTS,
};
use rustytracker_play::{PlaybackMixerMode, PlaybackState};

#[test]
fn test_protracker_mod_sample_swap() {
    let mut module = Module::empty_with_channels(1).unwrap();
    module.header.is_mod = true;
    module.header.frequency_table = FrequencyTable::Amiga;
    module.header.tick_speed = 1;

    // Set up instrument 1 (index 0) with sample 0
    let mut inst1 = Instrument::empty(0);
    inst1.note_sample_map = vec![Some(0); 120];
    module.instruments[0] = inst1;

    let mut sample0 = Sample::default();
    sample0.volume = 64;
    // Provide some sample data so rendering doesn't stop immediately
    sample0.data = SampleData::pcm8(vec![10; 1000]);
    module.samples[0] = sample0;

    // Set up instrument 2 (index 1) with sample 1
    let mut inst2 = Instrument::empty(1);
    inst2.note_sample_map = vec![Some(1); 120];
    module.instruments[1] = inst2;

    let mut sample1 = Sample::default();
    sample1.volume = 32;
    sample1.data = SampleData::pcm8(vec![20; 1000]);
    module.samples[1] = sample1;

    // Set up pattern with 2 rows
    let mut pattern = Pattern::new(2, 1, DEFAULT_EFFECT_SLOTS);
    
    // Row 0: trigger Note C-4 with Instrument 1
    let cell0 = PatternCell {
        note: Note::key(4, NoteName::C).unwrap(),
        instrument: 1,
        ..PatternCell::default()
    };
    pattern.set_cell(0, 0, cell0).unwrap();

    // Row 1: no note, Instrument 2
    let cell1 = PatternCell {
        note: Note::Empty,
        instrument: 2,
        ..PatternCell::default()
    };
    pattern.set_cell(0, 1, cell1).unwrap();

    module.patterns = vec![pattern];
    module.orders = vec![0];

    // Start playback (triggers Row 0 tick 0)
    let mut state = PlaybackState::start_with_settings(
        &module,
        rustytracker_play::PlaybackSettings::with_mixer_mode(PlaybackMixerMode::ProTracker),
    ).unwrap();

    // Verify initial Row 0 state
    {
        let channels = state.channels();
        assert!(channels[0].active);
        assert_eq!(channels[0].sample_index, Some(0));
        assert_eq!(channels[0].volume, 64);
    }

    // Render some samples to advance sample_frame
    let _frames = state.step_samples(&module).unwrap();

    let frame_before_swap = state.channels()[0].sample_frame;
    assert!(frame_before_swap > 0, "sample_frame should have advanced");

    // Advance to Row 1 (tick speed is 1, so this triggers Row 1 tick 0)
    state.advance_tick(&module).unwrap();

    {
        let channels = state.channels();
        assert!(channels[0].active);
        assert_eq!(channels[0].sample_index, Some(1), "sample should have swapped to 1");
        assert_eq!(channels[0].volume, 32, "volume should have updated to 32");
        assert_eq!(
            channels[0].sample_frame, frame_before_swap,
            "sample_frame should NOT have reset to 0"
        );
    }
}
