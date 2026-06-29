use crate::channel::PlaybackChannelState;
use crate::error::PlaybackResult;
use crate::{
    Mixer, PlaybackSettings, RawStereoPcmFrame, SequencerCommand, PLAYBACK_STEREO_SILENCE,
};
use rustytracker_core::{Module, Note, PatternCell};

const PREVIEW_CHANNEL: u16 = 0;

/// A single, monophonic preview voice for auditioning an instrument/sample
/// outside of song playback. Reuses the shared mixer engine so preview honors
/// the selected mixer mode.
#[derive(Debug, Clone)]
pub struct PreviewVoice {
    mixer: Mixer,
    channels: Vec<PlaybackChannelState>,
    settings: PlaybackSettings,
}

impl Default for PreviewVoice {
    fn default() -> Self {
        Self::new()
    }
}

impl PreviewVoice {
    pub fn new() -> Self {
        Self {
            mixer: Mixer::new(1),
            channels: vec![PlaybackChannelState::empty(PREVIEW_CHANNEL)],
            settings: PlaybackSettings::default(),
        }
    }

    /// Trigger a note for the given instrument. Monophonic: any currently
    /// sounding preview note is cut first. On a missing instrument/sample the
    /// voice stays silent and the error is returned.
    pub fn note_on(
        &mut self,
        module: &Module,
        instrument: u8,
        note: u8,
        settings: PlaybackSettings,
    ) -> PlaybackResult<()> {
        self.settings = settings;

        // Mono: stop whatever was playing before resolving the new note.
        self.mixer.handle_commands(&[SequencerCommand::Stop {
            channel: PREVIEW_CHANNEL,
        }]);

        let cell = PatternCell {
            note: Note::Key(note),
            instrument,
            ..PatternCell::default()
        };
        self.channels[0].apply_cell(module, &cell)?;

        if self.channels[0].active {
            if let (Some(sample_index), Some(instrument_index)) = (
                self.channels[0].sample_index,
                self.channels[0].instrument_index,
            ) {
                self.mixer.handle_commands(&[SequencerCommand::Trigger {
                    channel: PREVIEW_CHANNEL,
                    sample_index,
                    instrument_index,
                    note: self.channels[0].note,
                    instrument: self.channels[0].instrument,
                    volume: self.channels[0].volume,
                    panning: self.channels[0].panning,
                    period: self.channels[0].period,
                    offset: None,
                }]);
            }
        }

        Ok(())
    }

    pub fn note_off(&mut self) {
        self.mixer.handle_commands(&[SequencerCommand::Stop {
            channel: PREVIEW_CHANNEL,
        }]);
    }

    pub fn is_active(&self) -> bool {
        self.mixer
            .voices
            .first()
            .map(|voice| voice.active)
            .unwrap_or(false)
    }

    pub fn render_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawStereoPcmFrame> {
        if !self.is_active() {
            return Ok(PLAYBACK_STEREO_SILENCE);
        }
        self.mixer.render_stereo_frame(
            module,
            sample_rate,
            &mut self.channels,
            self.settings.mixer_mode,
        )
    }
}
