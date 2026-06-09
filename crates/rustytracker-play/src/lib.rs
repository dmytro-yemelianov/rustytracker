//! Playback sequencing, channel state, and raw PCM rendering for RustyTracker.
//!
//! The crate root is the compatibility facade for playback modules:
//!
//! ```
//! use rustytracker_play::{
//!     PlaybackChannelState, PlaybackEnvelopeState, PlaybackSampleValue, EFFECT_ARPEGGIO_ZERO,
//!     VIB_TAB,
//! };
//!
//! let _ = core::mem::size_of::<PlaybackChannelState>();
//! let _ = core::mem::size_of::<PlaybackEnvelopeState>();
//! let _ = core::mem::size_of::<PlaybackSampleValue>();
//! let _ = EFFECT_ARPEGGIO_ZERO;
//! let _ = VIB_TAB.len();
//! ```

mod channel;
mod cursor;
mod effects;
mod envelope;
mod error;
mod flow;
mod render;
mod timing;

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
pub use render::{
    RawMonoPcmFrame, RawStereoPcmFrame, PLAYBACK_MONO_SILENCE, PLAYBACK_STEREO_SILENCE,
};
use rustytracker_core::Module;
pub use timing::{
    PlaybackTiming, PLAYBACK_MIN_BPM, PLAYBACK_MIN_TICK_SPEED, PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackState {
    clock: PlaybackClock,
    channels: Vec<PlaybackChannelState>,
    tick_samples_fractional_rem: i64,
    song_ended: bool,
    initialized: bool,
    use_pal_clock: bool,
}

impl PlaybackState {
    pub fn start(module: &Module) -> PlaybackResult<Self> {
        Self::start_with_config(module, false)
    }

    pub fn start_with_config(module: &Module, use_pal_clock: bool) -> PlaybackResult<Self> {
        let clock = PlaybackClock::start(module)?;
        let row_state = clock.row_state(module)?;
        let channels = row_state
            .channels
            .iter()
            .map(|channel| PlaybackChannelState::empty(channel.channel))
            .collect();
        let mut state = Self {
            clock,
            channels,
            tick_samples_fractional_rem: 0,
            song_ended: false,
            initialized: false,
            use_pal_clock,
        };
        state.apply_row_state(module, &row_state)?;
        Ok(state)
    }

    pub fn clock(&self) -> PlaybackClock {
        self.clock
    }

    pub fn channels(&self) -> &[PlaybackChannelState] {
        &self.channels
    }

    pub fn song_ended(&self) -> bool {
        self.song_ended
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        self.clock.row_state(module)
    }

    pub fn advance_tick(&mut self, module: &Module) -> PlaybackResult<TickAdvance> {
        if self.song_ended {
            return Ok(TickAdvance::SongEnd);
        }
        let advance = self.clock.advance_tick(module)?;
        match advance {
            TickAdvance::NextRow | TickAdvance::NextOrder => self.trigger_current_row(module)?,
            TickAdvance::SameRow => {
                let current_tick = self.clock.tick();
                for channel in &mut self.channels {
                    channel.process_tick_effects(module, current_tick);
                }
            }
            TickAdvance::SongEnd => {
                self.song_ended = true;
            }
        }
        Ok(advance)
    }

    pub fn step_samples(&mut self, module: &Module) -> PlaybackResult<Vec<ChannelSampleFrame>> {
        let mut frames = Vec::new();
        for channel in &mut self.channels {
            if let Some(frame) = channel.step_sample(module)? {
                frames.push(frame);
            }
        }

        Ok(frames)
    }

    fn trigger_current_row(&mut self, module: &Module) -> PlaybackResult<()> {
        let row_state = self.clock.row_state(module)?;
        self.apply_row_state(module, &row_state)
    }

    fn apply_row_state(
        &mut self,
        module: &Module,
        row_state: &PlaybackRowState,
    ) -> PlaybackResult<()> {
        if flow::apply_row_flow(&mut self.clock, module, row_state)? {
            self.song_ended = true;
        }

        for channel in &row_state.channels {
            let ch_state = &mut self.channels[usize::from(channel.channel)];
            ch_state.apply_cell(module, &channel.cell)?;
            ch_state.process_tick_effects(module, 0);
        }

        Ok(())
    }
}
