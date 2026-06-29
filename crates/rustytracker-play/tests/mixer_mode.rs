use rustytracker_play::PlaybackMixerMode;

#[test]
fn rustysynth_replaces_milkytracker_mode() {
    let mode = PlaybackMixerMode::from_name("rustysynth").unwrap();
    assert_eq!(mode, PlaybackMixerMode::RustySynth);
    assert_eq!(mode.cli_name(), "rustysynth");
    assert_eq!(mode.label(), "RustySynth");

    // Short aliases still resolve.
    assert_eq!(
        PlaybackMixerMode::from_name("rusty"),
        Some(PlaybackMixerMode::RustySynth)
    );
    assert_eq!(
        PlaybackMixerMode::from_name("rs"),
        Some(PlaybackMixerMode::RustySynth)
    );

    // The old program name no longer maps to the project synth mode.
    assert_eq!(PlaybackMixerMode::from_name("milkytracker"), None);

    // RustySynth is part of the selectable set; HiFi stays the default.
    assert!(PlaybackMixerMode::ALL.contains(&PlaybackMixerMode::RustySynth));
    assert_eq!(PlaybackMixerMode::default(), PlaybackMixerMode::HiFi);
}
