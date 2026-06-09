use rustytracker_core::Module;

use crate::error::{PlaybackError, PlaybackResult};

pub const PLAYBACK_MIN_TICK_SPEED: u16 = 1;
pub const PLAYBACK_MIN_BPM: u16 = 1;
pub const PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM: u64 = 2_500_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaybackTiming {
    pub tick_speed: u16,
    pub bpm: u16,
    pub tick_duration_nanos: u64,
}

impl PlaybackTiming {
    pub fn from_module(module: &Module) -> PlaybackResult<Self> {
        let tick_speed = module.header.tick_speed;
        if tick_speed < PLAYBACK_MIN_TICK_SPEED {
            return Err(PlaybackError::InvalidTickSpeed { tick_speed });
        }

        let bpm = module.header.bpm;
        if bpm < PLAYBACK_MIN_BPM {
            return Err(PlaybackError::InvalidBpm { bpm });
        }

        Ok(Self {
            tick_speed,
            bpm,
            tick_duration_nanos: PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM / u64::from(bpm),
        })
    }

    pub fn ticks_per_row(&self) -> u16 {
        self.tick_speed
    }

    pub fn bpm(&self) -> u16 {
        self.bpm
    }

    pub fn tick_duration_nanos(&self) -> u64 {
        self.tick_duration_nanos
    }

    pub fn row_duration_nanos(&self) -> u64 {
        self.tick_duration_nanos
            .saturating_mul(u64::from(self.tick_speed))
    }

    pub fn set_bpm(&mut self, bpm: u16) -> PlaybackResult<()> {
        if bpm < PLAYBACK_MIN_BPM {
            return Err(PlaybackError::InvalidBpm { bpm });
        }
        self.bpm = bpm;
        self.tick_duration_nanos = PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM / u64::from(bpm);
        Ok(())
    }

    pub fn set_tick_speed(&mut self, tick_speed: u16) -> PlaybackResult<()> {
        if tick_speed < PLAYBACK_MIN_TICK_SPEED {
            return Err(PlaybackError::InvalidTickSpeed { tick_speed });
        }
        self.tick_speed = tick_speed;
        Ok(())
    }
}
