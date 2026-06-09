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
use error::validate_sample_rate;
pub use error::{PlaybackError, PlaybackResult, PLAYBACK_MIN_SAMPLE_RATE};
pub use flow::{
    EFFECT_PATTERN_BREAK, EFFECT_POSITION_JUMP, EFFECT_SET_SPEED_BPM, SPEED_BPM_THRESHOLD,
};
use rustytracker_core::{FrequencyTable, Module, Sample, SampleData, SampleLoopKind};
pub use timing::{
    PlaybackTiming, PLAYBACK_MIN_BPM, PLAYBACK_MIN_TICK_SPEED, PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM,
};

pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const PLAYBACK_STEREO_SILENCE: RawStereoPcmFrame = (0, 0);

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

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

    pub fn render_raw_mono_pcm(
        &mut self,
        module: &Module,
        sample_rate: u32,
        frame_count: usize,
    ) -> PlaybackResult<Vec<RawMonoPcmFrame>> {
        validate_sample_rate(sample_rate)?;
        let mut rendered = vec![PLAYBACK_MONO_SILENCE; frame_count];
        self.render_raw_mono_into(module, sample_rate, &mut rendered)?;
        Ok(rendered)
    }

    pub fn render_raw_mono_into(
        &mut self,
        module: &Module,
        sample_rate: u32,
        output: &mut [RawMonoPcmFrame],
    ) -> PlaybackResult<()> {
        validate_sample_rate(sample_rate)?;
        for frame in output {
            *frame = self.render_raw_mono_frame(module, sample_rate)?;
        }

        Ok(())
    }

    pub fn render_raw_stereo_pcm(
        &mut self,
        module: &Module,
        sample_rate: u32,
        frame_count: usize,
    ) -> PlaybackResult<Vec<RawStereoPcmFrame>> {
        validate_sample_rate(sample_rate)?;
        let mut rendered = vec![PLAYBACK_STEREO_SILENCE; frame_count];
        self.render_raw_stereo_into(module, sample_rate, &mut rendered)?;
        Ok(rendered)
    }

    pub fn render_raw_stereo_into(
        &mut self,
        module: &Module,
        sample_rate: u32,
        output: &mut [RawStereoPcmFrame],
    ) -> PlaybackResult<()> {
        validate_sample_rate(sample_rate)?;
        for frame in output {
            *frame = self.render_raw_stereo_frame(module, sample_rate)?;
        }

        Ok(())
    }

    pub fn render_to_wav(&mut self, module: &Module, sample_rate: u32) -> PlaybackResult<Vec<u8>> {
        validate_sample_rate(sample_rate)?;

        use std::io::{Cursor, Seek, SeekFrom, Write};

        let mut buffer = Cursor::new(Vec::new());

        // Write a dummy header first
        let header_bytes = [0u8; 44];
        buffer
            .write_all(&header_bytes)
            .expect("writing to memory buffer should not fail");

        let mut total_samples_written: u32 = 0;
        let buffer_frames = 1024;
        let mut frames = vec![PLAYBACK_STEREO_SILENCE; buffer_frames];
        let mut bytes = Vec::with_capacity(buffer_frames * 4);

        while !self.song_ended() {
            self.render_raw_stereo_into(module, sample_rate, &mut frames)?;

            bytes.clear();
            for &(left_i32, right_i32) in &frames {
                let left_i16 = left_i32.clamp(-32768, 32767) as i16;
                let right_i16 = right_i32.clamp(-32768, 32767) as i16;
                bytes.extend_from_slice(&left_i16.to_le_bytes());
                bytes.extend_from_slice(&right_i16.to_le_bytes());
                total_samples_written += 1;
            }
            buffer
                .write_all(&bytes)
                .expect("writing to memory buffer should not fail");

            if self.song_ended() {
                break;
            }

            // Safety limit (1 hour of audio max)
            if total_samples_written > sample_rate * 3600 {
                break;
            }
        }

        buffer
            .seek(SeekFrom::Start(0))
            .expect("seeking in memory buffer should not fail");

        let num_channels = 2u16;
        let bits_per_sample = 16u16;
        let block_align = num_channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;
        let data_size = total_samples_written * block_align as u32;
        let file_size = 36 + data_size;

        let mut header = [0u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        header[16..20].copy_from_slice(&16u32.to_le_bytes()); // Chunk size
        header[20..22].copy_from_slice(&1u16.to_le_bytes()); // Audio format (1 = PCM)
        header[22..24].copy_from_slice(&num_channels.to_le_bytes());
        header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
        header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
        header[32..34].copy_from_slice(&block_align.to_le_bytes());
        header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
        header[36..40].copy_from_slice(b"data");
        header[40..44].copy_from_slice(&data_size.to_le_bytes());

        buffer
            .write_all(&header)
            .expect("writing to memory buffer should not fail");

        Ok(buffer.into_inner())
    }

    pub fn render_raw_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawStereoPcmFrame> {
        validate_sample_rate(sample_rate)?;
        let mut current_bpm = self.clock.timing().bpm() as i64;

        if !self.initialized {
            self.tick_samples_fractional_rem = 5 * sample_rate as i64;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.song_ended {
                return Ok(PLAYBACK_STEREO_SILENCE);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    self.song_ended = true;
                    return Ok(PLAYBACK_STEREO_SILENCE);
                }
                _ => {
                    let new_bpm = self.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = 2 * current_bpm;
                        let new_denom = 2 * new_bpm;
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                    }
                    self.tick_samples_fractional_rem += 5 * sample_rate as i64;
                }
            }
        }

        let mut mixed_l = 0.0;
        let mut mixed_r = 0.0;
        for channel in &mut self.channels {
            if channel.active {
                if let Some(sample_index) = channel.sample_index {
                    if let Some(sample) = module.samples.get(sample_index) {
                        let frame_count = sample.data.frame_count();
                        if frame_count > 0 {
                            let frequency = period_to_frequency(
                                channel.period,
                                module.header.frequency_table,
                                self.use_pal_clock,
                            );
                            let step = frequency / sample_rate as f64;

                            let interpolated_val = get_sample_value_linear(
                                &sample.data,
                                channel.sample_frame,
                                channel.sample_frame_fraction,
                                sample,
                            );

                            let vol_factor = (channel.volume as f64 / 255.0)
                                * (channel.volume_envelope_val as f64 / 256.0)
                                * (channel.fadeout_volume as f64 / 65536.0);

                            let channel_mono_pcm = interpolated_val * vol_factor;

                            let mut pan =
                                channel.panning as i32 + channel.panning_envelope_val as i32 - 128;
                            pan = pan.clamp(0, 255);

                            let right_gain = pan as f64 / 255.0;
                            let left_gain = 1.0 - right_gain;

                            mixed_l += channel_mono_pcm * left_gain;
                            mixed_r += channel_mono_pcm * right_gain;

                            channel.advance_sample_position(sample, step);
                        }
                    }
                }
            }
        }

        self.tick_samples_fractional_rem -= 2 * current_bpm;
        Ok((mixed_l as i32, mixed_r as i32))
    }

    pub fn render_raw_mono_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawMonoPcmFrame> {
        validate_sample_rate(sample_rate)?;
        let mut current_bpm = self.clock.timing().bpm() as i64;

        if !self.initialized {
            self.tick_samples_fractional_rem = 5 * sample_rate as i64;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.song_ended {
                return Ok(PLAYBACK_MONO_SILENCE);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    self.song_ended = true;
                    return Ok(PLAYBACK_MONO_SILENCE);
                }
                _ => {
                    let new_bpm = self.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = 2 * current_bpm;
                        let new_denom = 2 * new_bpm;
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                    }
                    self.tick_samples_fractional_rem += 5 * sample_rate as i64;
                }
            }
        }

        let mut mixed = PLAYBACK_MONO_SILENCE;
        for channel in &mut self.channels {
            if channel.active {
                if let Some(sample_index) = channel.sample_index {
                    if let Some(sample) = module.samples.get(sample_index) {
                        let frame_count = sample.data.frame_count();
                        if frame_count > 0 {
                            let frequency = period_to_frequency(
                                channel.period,
                                module.header.frequency_table,
                                self.use_pal_clock,
                            );
                            let step = frequency / sample_rate as f64;

                            let interpolated_val = get_sample_value_linear(
                                &sample.data,
                                channel.sample_frame,
                                channel.sample_frame_fraction,
                                sample,
                            );

                            let vol_factor = (channel.volume as f64 / 255.0)
                                * (channel.volume_envelope_val as f64 / 256.0)
                                * (channel.fadeout_volume as f64 / 65536.0);

                            let channel_mono_pcm = interpolated_val * vol_factor;
                            mixed += channel_mono_pcm as i32;

                            channel.advance_sample_position(sample, step);
                        }
                    }
                }
            }
        }

        self.tick_samples_fractional_rem -= 2 * current_bpm;
        Ok(mixed)
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

fn sample_value_as_f64(data: &SampleData, index: usize) -> f64 {
    match data {
        SampleData::Empty => 0.0,
        SampleData::Pcm8(values) => {
            if let Some(&val) = values.get(index) {
                ((val as i32) << PLAYBACK_PCM8_TO_I16_SHIFT) as f64
            } else {
                0.0
            }
        }
        SampleData::Pcm16(values) => {
            if let Some(&val) = values.get(index) {
                val as f64
            } else {
                0.0
            }
        }
    }
}

fn next_frame_index(frame: usize, sample: &Sample) -> Option<usize> {
    let frame_count = sample.data.frame_count();
    if frame_count == 0 {
        return None;
    }
    let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
    if has_loop {
        let loop_start = sample.loop_start as usize;
        let loop_length = sample.loop_length as usize;
        let loop_end = loop_start + loop_length;

        let next = frame + 1;
        if next >= loop_end {
            Some(loop_start + (next - loop_end) % loop_length)
        } else {
            Some(next)
        }
    } else {
        let next = frame + 1;
        if next >= frame_count {
            None
        } else {
            Some(next)
        }
    }
}

fn get_sample_value_linear(data: &SampleData, frame: usize, fraction: u32, sample: &Sample) -> f64 {
    let t = fraction as f64 / u32::MAX as f64;
    let y0 = sample_value_as_f64(data, frame);
    let y1 = if let Some(next_idx) = next_frame_index(frame, sample) {
        sample_value_as_f64(data, next_idx)
    } else {
        0.0
    };
    y0 + t * (y1 - y0)
}

fn period_to_frequency(period: u32, table: FrequencyTable, use_pal_clock: bool) -> f64 {
    if period == 0 {
        return 0.0;
    }
    match table {
        FrequencyTable::Linear => 8363.0 * f64::powf(2.0, (4608.0 - period as f64) / 768.0),
        FrequencyTable::Amiga => {
            let base = if use_pal_clock { 3546895.0 } else { 3579364.0 };
            base / period as f64
        }
    }
}
