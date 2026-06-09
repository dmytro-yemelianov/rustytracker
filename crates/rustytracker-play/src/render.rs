use std::io::{Cursor, Seek, SeekFrom, Write};

use rustytracker_core::{FrequencyTable, Module, Sample, SampleData, SampleLoopKind};

use crate::channel::{PlaybackChannelState, PLAYBACK_PCM8_TO_I16_SHIFT};
use crate::error::validate_sample_rate;
use crate::{PlaybackResult, PlaybackState, TickAdvance};

pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const PLAYBACK_STEREO_SILENCE: RawStereoPcmFrame = (0, 0);

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

impl PlaybackState {
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

        let mut buffer = Cursor::new(Vec::new());

        // Write a dummy header first.
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

            // Safety limit (1 hour of audio max).
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
        let Some(current_bpm) = self.prepare_render_frame(module, sample_rate)? else {
            return Ok(PLAYBACK_STEREO_SILENCE);
        };

        let mut mixed_l = 0.0;
        let mut mixed_r = 0.0;
        for channel in &mut self.channels {
            if let Some(channel_mono_pcm) = render_channel_mono_pcm(
                channel,
                module,
                sample_rate,
                module.header.frequency_table,
                self.use_pal_clock,
            ) {
                let (left_gain, right_gain) = channel_pan_gains(channel);
                mixed_l += channel_mono_pcm * left_gain;
                mixed_r += channel_mono_pcm * right_gain;
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
        let Some(current_bpm) = self.prepare_render_frame(module, sample_rate)? else {
            return Ok(PLAYBACK_MONO_SILENCE);
        };

        let mut mixed = PLAYBACK_MONO_SILENCE;
        for channel in &mut self.channels {
            if let Some(channel_mono_pcm) = render_channel_mono_pcm(
                channel,
                module,
                sample_rate,
                module.header.frequency_table,
                self.use_pal_clock,
            ) {
                mixed += channel_mono_pcm as i32;
            }
        }

        self.tick_samples_fractional_rem -= 2 * current_bpm;
        Ok(mixed)
    }

    fn prepare_render_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
    ) -> PlaybackResult<Option<i64>> {
        let mut current_bpm = self.clock.timing().bpm() as i64;

        if !self.initialized {
            self.tick_samples_fractional_rem = 5 * sample_rate as i64;
            self.initialized = true;
        }

        while self.tick_samples_fractional_rem <= 0 {
            if self.song_ended {
                return Ok(None);
            }

            match self.advance_tick(module)? {
                TickAdvance::SongEnd => {
                    self.song_ended = true;
                    return Ok(None);
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

        Ok(Some(current_bpm))
    }
}

fn render_channel_mono_pcm(
    channel: &mut PlaybackChannelState,
    module: &Module,
    sample_rate: u32,
    frequency_table: FrequencyTable,
    use_pal_clock: bool,
) -> Option<f64> {
    if !channel.active {
        return None;
    }

    let sample = module.samples.get(channel.sample_index?)?;
    if sample.data.frame_count() == 0 {
        return None;
    }

    let frequency = period_to_frequency(channel.period, frequency_table, use_pal_clock);
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

    channel.advance_sample_position(sample, step);

    Some(channel_mono_pcm)
}

fn channel_pan_gains(channel: &PlaybackChannelState) -> (f64, f64) {
    let pan = (channel.panning as i32 + channel.panning_envelope_val as i32 - 128).clamp(0, 255);
    let right_gain = pan as f64 / 255.0;
    let left_gain = 1.0 - right_gain;
    (left_gain, right_gain)
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
