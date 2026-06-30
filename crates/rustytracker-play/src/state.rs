use rustytracker_core::{FrequencyTable, Module};

use crate::channel::{ChannelSampleFrame, PlaybackChannelState};
use crate::error::{validate_sample_rate, PlaybackResult};
use crate::mixer::{Mixer, MixerTrackControl};
use crate::sequencer::Sequencer;
use crate::{PlaybackClock, PlaybackRowState, TickAdvance};
use crate::{
    PlaybackMixerMode, PlaybackSettings, RawMonoPcmFrame, RawStereoPcmFrame, PLAYBACK_MONO_SILENCE,
    PLAYBACK_STEREO_SILENCE,
};

const MILKY_MIXER_BASE_FREQUENCY: i64 = 48_000;
const MILKY_MIXER_TIMER_FREQUENCY: i64 = 250;
const MILKY_MIXER_BEAT_LENGTH: i64 = MILKY_MIXER_BASE_FREQUENCY / MILKY_MIXER_TIMER_FREQUENCY;
const MILKY_BPM_TICK_BASE: i64 = 625;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackState {
    pub(crate) sequencer: Sequencer,
    pub(crate) mixer: Mixer,
    tick_samples_fractional_rem: i64,
    initialized: bool,
    settings: PlaybackSettings,
}

impl PlaybackState {
    pub fn start(module: &Module) -> PlaybackResult<Self> {
        Self::start_with_settings(module, PlaybackSettings::default())
    }

    pub fn start_with_config(module: &Module, use_pal_clock: bool) -> PlaybackResult<Self> {
        let mixer_mode = if use_pal_clock {
            PlaybackMixerMode::Amiga
        } else {
            PlaybackMixerMode::HiFi
        };
        Self::start_with_mixer_mode(module, mixer_mode)
    }

    pub fn start_with_mixer_mode(
        module: &Module,
        mixer_mode: PlaybackMixerMode,
    ) -> PlaybackResult<Self> {
        Self::start_with_settings(module, PlaybackSettings::with_mixer_mode(mixer_mode))
    }

    pub fn start_with_settings(
        module: &Module,
        settings: PlaybackSettings,
    ) -> PlaybackResult<Self> {
        let sequencer = Sequencer::start_with_config(module)?;
        let mixer = Mixer::new(module.header.channel_count as usize);

        let mut state = Self {
            sequencer,
            mixer,
            tick_samples_fractional_rem: 0,
            initialized: false,
            settings,
        };

        // Sync initial state of sequencer to mixer
        let initial_cmds = state.sequencer.generate_initial_commands();
        state.mixer.handle_commands(&initial_cmds);
        state.mixer.sync_to_channels(&mut state.sequencer.channels);

        Ok(state)
    }

    pub fn settings(&self) -> PlaybackSettings {
        self.settings
    }

    pub fn clock(&self) -> PlaybackClock {
        self.sequencer.clock
    }

    pub fn channels(&self) -> &[PlaybackChannelState] {
        &self.sequencer.channels
    }

    pub fn set_track_controls(&mut self, controls: &[MixerTrackControl]) {
        self.mixer.set_track_controls(controls);
    }

    pub fn track_activity_mask(&self) -> u32 {
        let mut mask = 0u32;

        for channel in &self.sequencer.channels {
            if channel.active {
                let shift = usize::from(channel.channel);
                if shift < u32::BITS as usize {
                    mask |= 1u32 << shift;
                }
            }
        }

        mask
    }

    pub fn global_volume(&self) -> u8 {
        self.sequencer.global_volume
    }

    pub fn song_ended(&self) -> bool {
        self.sequencer.song_ended
    }

    pub fn row_state(&self, module: &Module) -> PlaybackResult<PlaybackRowState> {
        self.sequencer.clock.row_state(module)
    }

    pub fn advance_tick(&mut self, module: &Module) -> PlaybackResult<TickAdvance> {
        let (advance, commands) = self.sequencer.advance_tick(module)?;
        self.mixer.handle_commands(&commands);
        self.mixer.sync_to_channels(&mut self.sequencer.channels);
        Ok(advance)
    }

    pub fn step_samples(&mut self, module: &Module) -> PlaybackResult<Vec<ChannelSampleFrame>> {
        self.mixer
            .step_samples(module, &mut self.sequencer.channels)
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

        let mut total_frames_written: u32 = 0;

        while !self.song_ended() {
            let (left_i32, right_i32) = self.render_raw_stereo_frame(module, sample_rate)?;
            let left_i16 = left_i32.clamp(-32768, 32767) as i16;
            let right_i16 = right_i32.clamp(-32768, 32767) as i16;

            buffer
                .write_all(&left_i16.to_le_bytes())
                .expect("writing to memory buffer should not fail");
            buffer
                .write_all(&right_i16.to_le_bytes())
                .expect("writing to memory buffer should not fail");
            total_frames_written += 1;

            // Safety limit (1 hour of audio max)
            if total_frames_written > sample_rate * 3600 {
                break;
            }
        }

        buffer
            .seek(SeekFrom::Start(0))
            .expect("seeking in memory buffer should not fail");

        let data_size = total_frames_written * 4;
        let file_size = data_size + 36;
        let byte_rate = sample_rate * 4;
        let block_align: u16 = 4;
        let bits_per_sample: u16 = 16;
        let num_channels: u16 = 2;

        let mut header = [0u8; 44];
        header[0..4].copy_from_slice(b"RIFF");
        header[4..8].copy_from_slice(&file_size.to_le_bytes());
        header[8..12].copy_from_slice(b"WAVE");
        header[12..16].copy_from_slice(b"fmt ");
        let subchunk1_size: u32 = 16;
        header[16..20].copy_from_slice(&subchunk1_size.to_le_bytes());
        let audio_format: u16 = 1; // PCM
        header[20..22].copy_from_slice(&audio_format.to_le_bytes());
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
        let mut current_bpm = self.sequencer.clock.timing().bpm() as i64;
        let (tick_numerator, mut tick_denominator) =
            render_tick_clock(sample_rate, current_bpm, module.header.frequency_table);

        if !self.initialized {
            self.tick_samples_fractional_rem = tick_numerator;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.sequencer.song_ended {
                return Ok(PLAYBACK_STEREO_SILENCE);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    return Ok(PLAYBACK_STEREO_SILENCE);
                }
                _ => {
                    let new_bpm = self.sequencer.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = tick_denominator;
                        let (_, new_denom) =
                            render_tick_clock(sample_rate, new_bpm, module.header.frequency_table);
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                        tick_denominator = new_denom;
                    }
                    self.tick_samples_fractional_rem += tick_numerator;
                }
            }
        }

        let frame = self.mixer.render_stereo_frame(
            module,
            sample_rate,
            &mut self.sequencer.channels,
            self.settings.mixer_mode,
        )?;

        self.tick_samples_fractional_rem -= tick_denominator;
        Ok(frame)
    }

    pub fn render_raw_mono_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<RawMonoPcmFrame> {
        let mut current_bpm = self.sequencer.clock.timing().bpm() as i64;
        let (tick_numerator, mut tick_denominator) =
            render_tick_clock(sample_rate, current_bpm, module.header.frequency_table);

        if !self.initialized {
            self.tick_samples_fractional_rem = tick_numerator;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.sequencer.song_ended {
                return Ok(PLAYBACK_MONO_SILENCE);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    return Ok(PLAYBACK_MONO_SILENCE);
                }
                _ => {
                    let new_bpm = self.sequencer.clock.timing().bpm() as i64;
                    if new_bpm != current_bpm {
                        let old_denom = tick_denominator;
                        let (_, new_denom) =
                            render_tick_clock(sample_rate, new_bpm, module.header.frequency_table);
                        self.tick_samples_fractional_rem =
                            (self.tick_samples_fractional_rem * new_denom) / old_denom;
                        current_bpm = new_bpm;
                        tick_denominator = new_denom;
                    }
                    self.tick_samples_fractional_rem += tick_numerator;
                }
            }
        }

        let frame = self.mixer.render_mono_frame(
            module,
            sample_rate,
            &mut self.sequencer.channels,
            self.settings.mixer_mode,
        )?;

        self.tick_samples_fractional_rem -= tick_denominator;
        Ok(frame)
    }
}

// Helpers
fn render_tick_clock(sample_rate: u32, bpm: i64, table: FrequencyTable) -> (i64, i64) {
    match table {
        FrequencyTable::Linear => (5 * sample_rate as i64, 2 * bpm),
        FrequencyTable::Amiga => {
            let beat_packet_size =
                (MILKY_MIXER_BEAT_LENGTH * sample_rate as i64) / MILKY_MIXER_BASE_FREQUENCY;
            (beat_packet_size * MILKY_BPM_TICK_BASE, bpm)
        }
    }
}
