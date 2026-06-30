mod channel;
mod cursor;
mod effects;
mod envelope;
mod error;
mod flow;
mod mixer;
mod preview;
mod sample;
mod sequencer;
mod state;
mod timing;
mod warmth;

pub use channel::{
    ChannelSampleFrame, PlaybackChannelState, PlaybackSampleValue, PLAYBACK_EMPTY_VOLUME,
    PLAYBACK_INSTRUMENT_NUMBER_BASE, PLAYBACK_PCM8_TO_I16_SHIFT, PLAYBACK_SAMPLE_FRAME_STEP,
    PLAYBACK_SAMPLE_START_FRAME,
};
pub use cursor::{
    ChannelRowState, PlaybackClock, PlaybackCursor, PlaybackPosition, PlaybackRowState, RowAdvance,
    TickAdvance, PLAYBACK_EMPTY_PATTERN_ROWS, PLAYBACK_FIRST_CHANNEL, PLAYBACK_FIRST_ORDER_INDEX,
    PLAYBACK_FIRST_ROW, PLAYBACK_FIRST_TICK, PLAYBACK_ORDER_STEP, PLAYBACK_ROW_STEP,
    PLAYBACK_TICK_STEP,
};
pub use effects::{
    EFFECT_ARPEGGIO_NONZERO, EFFECT_ARPEGGIO_ZERO, EFFECT_FINE_VOLUME_SLIDE_DOWN,
    EFFECT_FINE_VOLUME_SLIDE_UP, EFFECT_PANNING, EFFECT_PORTAMENTO_DOWN, EFFECT_PORTAMENTO_UP,
    EFFECT_SAMPLE_OFFSET, EFFECT_TONE_PORTAMENTO, EFFECT_VIBRATO, EFFECT_VIBRATO_VOLSLIDE,
    EFFECT_VOLUME, EFFECT_VOLUME_SLIDE, VIB_TAB,
};
pub use envelope::PlaybackEnvelopeState;
pub use error::{PlaybackError, PlaybackResult, PLAYBACK_MIN_SAMPLE_RATE};
pub use flow::{
    EFFECT_PATTERN_BREAK, EFFECT_POSITION_JUMP, EFFECT_SET_SPEED_BPM, SPEED_BPM_THRESHOLD,
};
pub use mixer::{Mixer, MixerVoice};
pub use preview::PreviewVoice;
pub use sequencer::{Sequencer, SequencerCommand};
pub use state::PlaybackState;
pub use timing::{
    PlaybackTiming, PLAYBACK_MIN_BPM, PLAYBACK_MIN_TICK_SPEED, PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM,
};

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const PLAYBACK_STEREO_SILENCE: RawStereoPcmFrame = (0, 0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackMixerMode {
    #[default]
    HiFi,
    RustySynth,
    Amiga,
    ProTracker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Stepped,
    Linear,
    Cubic,
}

impl PlaybackMixerMode {
    pub const ALL: [Self; 4] = [Self::HiFi, Self::RustySynth, Self::Amiga, Self::ProTracker];

    pub fn label(self) -> &'static str {
        match self {
            Self::HiFi => "HiFi",
            Self::RustySynth => "RustySynth",
            Self::Amiga => "Amiga",
            Self::ProTracker => "ProTracker",
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Self::HiFi => "hifi",
            Self::RustySynth => "rustysynth",
            Self::Amiga => "amiga",
            Self::ProTracker => "protracker",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "hifi" | "hi-fi" => Some(Self::HiFi),
            "rustysynth" | "rusty" | "rs" => Some(Self::RustySynth),
            "amiga" => Some(Self::Amiga),
            "protracker" | "pro-tracker" | "pt" => Some(Self::ProTracker),
            _ => None,
        }
    }

    pub fn uses_pal_clock(self) -> bool {
        matches!(self, Self::Amiga | Self::ProTracker)
    }

    pub fn interpolation(self) -> Interpolation {
        match self {
            Self::HiFi => Interpolation::Linear,
            Self::RustySynth => Interpolation::Cubic,
            Self::Amiga | Self::ProTracker => Interpolation::Stepped,
        }
    }

    pub fn uses_warmth(self) -> bool {
        matches!(self, Self::RustySynth)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaybackSettings {
    pub mixer_mode: PlaybackMixerMode,
}

impl PlaybackSettings {
    pub fn with_mixer_mode(mixer_mode: PlaybackMixerMode) -> Self {
        Self { mixer_mode }
    }
}
