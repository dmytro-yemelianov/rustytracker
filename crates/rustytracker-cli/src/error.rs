use std::fmt;

use crate::USAGE;

#[derive(Debug)]
pub enum DumpError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Xm(rustytracker_xm::XmParseError),
    Mod(rustytracker_mod::ModParseError),
    Playback(rustytracker_play::PlaybackError),
    InvalidArguments,
    InvalidRowCount(String),
    InvalidSampleRate(String),
    InvalidMixerMode(String),
    UnsupportedFormat(String),
}

impl fmt::Display for DumpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "I/O error: {error}"),
            Self::Json(error) => write!(formatter, "JSON error: {error}"),
            Self::Xm(error) => write!(formatter, "XM parse error: {error:?}"),
            Self::Mod(error) => write!(formatter, "MOD parse error: {error:?}"),
            Self::Playback(error) => write!(formatter, "playback error: {error:?}"),
            Self::InvalidArguments => formatter.write_str(USAGE),
            Self::InvalidRowCount(value) => {
                write!(formatter, "invalid play-state row count: {value}")
            }
            Self::InvalidSampleRate(value) => {
                write!(formatter, "invalid export sample rate: {value}")
            }
            Self::InvalidMixerMode(value) => {
                write!(formatter, "invalid export mixer mode: {value}")
            }
            Self::UnsupportedFormat(format) => {
                write!(formatter, "unsupported dump format: {format}")
            }
        }
    }
}

impl std::error::Error for DumpError {}

impl From<std::io::Error> for DumpError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for DumpError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<rustytracker_xm::XmParseError> for DumpError {
    fn from(error: rustytracker_xm::XmParseError) -> Self {
        Self::Xm(error)
    }
}

impl From<rustytracker_mod::ModParseError> for DumpError {
    fn from(error: rustytracker_mod::ModParseError) -> Self {
        Self::Mod(error)
    }
}

impl From<rustytracker_play::PlaybackError> for DumpError {
    fn from(error: rustytracker_play::PlaybackError) -> Self {
        Self::Playback(error)
    }
}
