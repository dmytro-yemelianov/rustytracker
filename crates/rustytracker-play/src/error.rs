pub const PLAYBACK_MIN_SAMPLE_RATE: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackError {
    InvalidTickSpeed {
        tick_speed: u16,
    },
    InvalidBpm {
        bpm: u16,
    },
    InvalidSampleRate {
        sample_rate: u32,
    },
    EmptyOrderList,
    OrderIndexOutOfRange {
        order_index: usize,
        order_count: usize,
    },
    MissingPattern {
        order_index: usize,
        pattern_index: usize,
    },
    EmptyPattern {
        pattern_index: usize,
    },
    RowOutOfRange {
        pattern_index: usize,
        row: u16,
        rows: u16,
    },
    PatternChannelOutOfRange {
        pattern_index: usize,
        module_channels: u16,
        pattern_channels: u16,
    },
    MissingInstrument {
        channel: u16,
        instrument: u8,
    },
    MissingSample {
        channel: u16,
        instrument_index: usize,
        sample_index: usize,
    },
}

pub type PlaybackResult<T> = Result<T, PlaybackError>;

pub(crate) fn validate_sample_rate(sample_rate: u32) -> PlaybackResult<()> {
    if sample_rate < PLAYBACK_MIN_SAMPLE_RATE {
        return Err(PlaybackError::InvalidSampleRate { sample_rate });
    }

    Ok(())
}
