use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModParseError {
    Truncated {
        expected: usize,
        actual: usize,
    },
    InvalidSignature,
    InvalidOrderCount {
        orders: usize,
        maximum: usize,
    },
    InvalidChannelCount {
        channel_count: u16,
        minimum: u16,
        maximum: u16,
    },
}

impl fmt::Display for ModParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { expected, actual } => write!(
                f,
                "MOD file truncated: expected at least {expected} bytes, got {actual}"
            ),
            Self::InvalidSignature => write!(f, "Invalid MOD signature"),
            Self::InvalidOrderCount { orders, maximum } => {
                write!(f, "Invalid MOD order count: {orders}, maximum {maximum}")
            }
            Self::InvalidChannelCount {
                channel_count,
                minimum,
                maximum,
            } => write!(
                f,
                "Invalid MOD channel count: {channel_count}, expected {minimum}..={maximum}"
            ),
        }
    }
}

impl std::error::Error for ModParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModWriteError {
    TooManyChannels {
        channels: u16,
    },
    TooManyOrders {
        orders: usize,
    },
    TooManyPatterns {
        patterns: usize,
    },
    SampleTooLong {
        sample_index: usize,
        byte_len: usize,
        maximum: usize,
    },
    MissingSample {
        instrument_index: usize,
        sample_index: usize,
    },
    MissingPattern {
        pattern_index: usize,
    },
    InvalidPatternShape {
        pattern_index: usize,
        rows: u16,
        channels: u16,
        required_rows: u16,
        required_channels: u16,
    },
    UnsupportedExtraEffect {
        pattern_index: usize,
        row: u16,
        channel: u16,
        effect_slot: usize,
    },
    UnsupportedInstrument {
        pattern_index: usize,
        row: u16,
        channel: u16,
        instrument: u8,
        maximum: u8,
    },
}

impl fmt::Display for ModWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyChannels { channels } => {
                write!(f, "Too many channels for MOD format: {channels}")
            }
            Self::TooManyOrders { orders } => {
                write!(f, "Too many orders for MOD format: {orders}")
            }
            Self::TooManyPatterns { patterns } => {
                write!(f, "Too many patterns for MOD format: {patterns}")
            }
            Self::SampleTooLong {
                sample_index,
                byte_len,
                maximum,
            } => write!(
                f,
                "Sample {sample_index} is too long for MOD format: {byte_len} bytes, maximum {maximum}"
            ),
            Self::MissingSample {
                instrument_index,
                sample_index,
            } => write!(
                f,
                "Instrument {instrument_index} references missing sample {sample_index}"
            ),
            Self::MissingPattern { pattern_index } => {
                write!(f, "Order list references missing pattern {pattern_index}")
            }
            Self::InvalidPatternShape {
                pattern_index,
                rows,
                channels,
                required_rows,
                required_channels,
            } => write!(
                f,
                "Pattern {pattern_index} has shape {rows}x{channels}, but MOD export requires {required_rows}x{required_channels}"
            ),
            Self::UnsupportedExtraEffect {
                pattern_index,
                row,
                channel,
                effect_slot,
            } => write!(
                f,
                "Pattern {pattern_index} row {row} channel {channel} has non-empty extra effect slot {effect_slot}, which MOD cannot represent"
            ),
            Self::UnsupportedInstrument {
                pattern_index,
                row,
                channel,
                instrument,
                maximum,
            } => write!(
                f,
                "Pattern {pattern_index} row {row} channel {channel} references instrument {instrument}, but MOD export supports instruments up to {maximum}"
            ),
        }
    }
}

impl std::error::Error for ModWriteError {}
