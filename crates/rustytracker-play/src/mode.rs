//! Mixer mode selection: pitch clock, interpolation, and warmth per profile.

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
