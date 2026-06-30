use rustytracker_core::{Module, Sample, SampleLoopKind, SAMPLE_DEFAULT_PANNING};

use crate::channel::{
    ChannelSampleFrame, PlaybackChannelState, PLAYBACK_EMPTY_VOLUME, PLAYBACK_SAMPLE_FRAME_STEP,
    PLAYBACK_SAMPLE_START_FRAME,
};
use crate::error::{PlaybackError, PlaybackResult};
use crate::sample::{get_sample_value, period_to_frequency, sample_value_at_frame};
use crate::warmth::MasterWarmth;
use crate::{
    PlaybackMixerMode, RawMonoPcmFrame, RawStereoPcmFrame, SequencerCommand, PLAYBACK_MONO_SILENCE,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixerVoice {
    pub channel: u16,
    pub active: bool,
    pub sample_index: Option<usize>,
    pub sample_frame: usize,
    pub sample_frame_fraction: u32,
    pub volume: u8,
    pub panning: u8,
    pub period: u32,
    pub volume_envelope_val: u16,
    pub panning_envelope_val: u16,
    pub fadeout_volume: u32,
    pub keyon: bool,
    pub sample_backward: bool,
}

impl MixerVoice {
    pub fn empty(channel: u16) -> Self {
        Self {
            channel,
            active: false,
            sample_index: None,
            sample_frame: PLAYBACK_SAMPLE_START_FRAME,
            sample_frame_fraction: 0,
            volume: PLAYBACK_EMPTY_VOLUME,
            panning: SAMPLE_DEFAULT_PANNING,
            period: 0,
            volume_envelope_val: 256,
            panning_envelope_val: 128,
            fadeout_volume: 65536,
            keyon: true,
            sample_backward: false,
        }
    }

    pub fn stop_sample(&mut self) {
        self.active = false;
        self.sample_index = None;
        self.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
        self.sample_frame_fraction = 0;
    }

    fn advance_sample_position(&mut self, sample: &Sample, step: f64) {
        let frame_count = sample.data.frame_count();
        if frame_count == 0 {
            self.stop_sample();
            return;
        }

        let current_pos =
            self.sample_frame as f64 + (self.sample_frame_fraction as f64 / u32::MAX as f64);

        let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
        if has_loop {
            let loop_start = sample.loop_start as f64;
            let loop_length = sample.loop_length as f64;
            let loop_end = loop_start + loop_length;

            match sample.loop_kind {
                SampleLoopKind::Forward => {
                    let mut next_pos = current_pos + step;
                    if next_pos >= loop_end {
                        let over = next_pos - loop_end;
                        let wraps = (over / loop_length).floor();
                        next_pos = loop_start + over - wraps * loop_length;
                        next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                    } else {
                        next_pos = next_pos.min(loop_end - 0.000001);
                    }
                    self.sample_frame = next_pos as usize;
                    self.sample_frame_fraction =
                        ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                }
                SampleLoopKind::PingPong => {
                    if self.sample_backward {
                        let mut next_pos = current_pos - step;
                        if next_pos <= loop_start {
                            self.sample_backward = false;
                            let under = loop_start - next_pos;
                            let wraps = (under / loop_length).floor() as i32;
                            let rem = under - (wraps as f64) * loop_length;
                            if wraps % 2 == 0 {
                                next_pos = loop_start + rem;
                            } else {
                                self.sample_backward = true;
                                next_pos = loop_end - rem;
                            }
                        }
                        next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction =
                            ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                    } else {
                        let mut next_pos = current_pos + step;
                        if next_pos >= loop_end {
                            self.sample_backward = true;
                            let over = next_pos - loop_end;
                            let wraps = (over / loop_length).floor() as i32;
                            let rem = over - (wraps as f64) * loop_length;
                            if wraps % 2 == 0 {
                                next_pos = loop_end - rem;
                            } else {
                                self.sample_backward = false;
                                next_pos = loop_start + rem;
                            }
                            next_pos = next_pos.clamp(loop_start, loop_end - 0.000001);
                        } else {
                            next_pos = next_pos.min(loop_end - 0.000001);
                        }
                        self.sample_frame = next_pos as usize;
                        self.sample_frame_fraction =
                            ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
                    }
                }
                SampleLoopKind::None => unreachable!(),
            }
        } else {
            let next_pos = current_pos + step;
            if next_pos >= frame_count as f64 {
                self.stop_sample();
            } else {
                self.sample_frame = next_pos as usize;
                self.sample_frame_fraction =
                    ((next_pos - next_pos.floor()) * u32::MAX as f64) as u32;
            }
        }
    }

    fn render_sample(
        &mut self,
        module: &Module,
        sample_rate: u32,
        mixer_mode: PlaybackMixerMode,
        channels: &[PlaybackChannelState],
    ) -> PlaybackResult<Option<(f64, u8)>> {
        if !self.active {
            return Ok(None);
        }
        let Some(sample_index) = self.sample_index else {
            self.stop_sample();
            return Ok(None);
        };
        let Some(sample) = module.samples.get(sample_index) else {
            return Err(PlaybackError::MissingSample {
                channel: self.channel,
                instrument_index: channels[self.channel as usize]
                    .instrument_index
                    .unwrap_or(0),
                sample_index,
            });
        };
        if sample.data.frame_count() == 0 || self.sample_frame >= sample.data.frame_count() {
            self.stop_sample();
            return Ok(None);
        }

        let frequency = period_to_frequency(self.period, module.header.frequency_table, mixer_mode);
        let step = frequency / sample_rate as f64;

        let sample_val = get_sample_value(
            &sample.data,
            self.sample_frame,
            self.sample_frame_fraction,
            sample,
            mixer_mode,
        );

        let vol_factor = (self.volume as f64 / 255.0)
            * (self.volume_envelope_val as f64 / 256.0)
            * (self.fadeout_volume as f64 / 65536.0);

        let channel_mono_pcm = sample_val * vol_factor;

        let mut pan = self.panning as i32 + self.panning_envelope_val as i32 - 128;
        pan = pan.clamp(0, 255);

        self.advance_sample_position(sample, step);

        Ok(Some((channel_mono_pcm, pan as u8)))
    }

    fn advance_sample_frame(&mut self, sample: &Sample) {
        let frame_count = sample.data.frame_count();
        if frame_count == 0 {
            self.stop_sample();
            return;
        }

        let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;

        if has_loop {
            let loop_start = sample.loop_start as usize;
            let loop_length = sample.loop_length as usize;
            let loop_end = loop_start + loop_length;

            match sample.loop_kind {
                SampleLoopKind::Forward => {
                    let next_frame = self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
                    if next_frame >= loop_end {
                        self.sample_frame = loop_start + (next_frame - loop_end) % loop_length;
                    } else {
                        self.sample_frame = next_frame;
                    }
                }
                SampleLoopKind::PingPong => {
                    if self.sample_backward {
                        if self.sample_frame <= loop_start {
                            self.sample_backward = false;
                            self.sample_frame = (loop_start + 1).min(loop_end - 1);
                        } else {
                            self.sample_frame =
                                self.sample_frame.saturating_sub(PLAYBACK_SAMPLE_FRAME_STEP);
                        }
                    } else {
                        let next_frame =
                            self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
                        if next_frame >= loop_end {
                            self.sample_backward = true;
                            self.sample_frame =
                                (loop_end as i32 - 2).max(loop_start as i32) as usize;
                        } else {
                            self.sample_frame = next_frame;
                        }
                    }
                }
                SampleLoopKind::None => unreachable!(),
            }
        } else {
            let next_frame = self.sample_frame.saturating_add(PLAYBACK_SAMPLE_FRAME_STEP);
            if next_frame >= frame_count {
                self.stop_sample();
            } else {
                self.sample_frame = next_frame;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mixer {
    pub voices: Vec<MixerVoice>,
    warmth: MasterWarmth,
}

impl Mixer {
    pub fn new(channel_count: usize) -> Self {
        let voices = (0..channel_count)
            .map(|ch| MixerVoice::empty(ch as u16))
            .collect();
        Self {
            voices,
            warmth: MasterWarmth::new(),
        }
    }

    pub fn handle_commands(&mut self, commands: &[SequencerCommand]) {
        for cmd in commands {
            match *cmd {
                SequencerCommand::Trigger {
                    channel,
                    sample_index,
                    instrument_index: _,
                    note: _,
                    instrument: _,
                    volume,
                    panning,
                    period,
                    offset,
                } => {
                    let voice = &mut self.voices[channel as usize];
                    voice.active = true;
                    voice.sample_index = Some(sample_index);
                    voice.volume = volume;
                    voice.panning = panning;
                    voice.period = period;
                    voice.volume_envelope_val = 256;
                    voice.panning_envelope_val = 128;
                    voice.fadeout_volume = 65536;
                    voice.keyon = true;
                    voice.sample_backward = false;
                    if let Some(offset_val) = offset {
                        voice.sample_frame = offset_val;
                        voice.sample_frame_fraction = 0;
                    } else {
                        voice.sample_frame = PLAYBACK_SAMPLE_START_FRAME;
                        voice.sample_frame_fraction = 0;
                    }
                }
                SequencerCommand::Update {
                    channel,
                    sample_index,
                    volume,
                    panning,
                    period,
                    volume_envelope_val,
                    panning_envelope_val,
                    fadeout_volume,
                    keyon,
                } => {
                    let voice = &mut self.voices[channel as usize];
                    voice.sample_index = sample_index;
                    voice.volume = volume;
                    voice.panning = panning;
                    voice.period = period;
                    voice.volume_envelope_val = volume_envelope_val;
                    voice.panning_envelope_val = panning_envelope_val;
                    voice.fadeout_volume = fadeout_volume;
                    voice.keyon = keyon;
                }
                SequencerCommand::Stop { channel } => {
                    let voice = &mut self.voices[channel as usize];
                    voice.stop_sample();
                }
            }
        }
    }

    pub fn sync_to_channels(&self, channels: &mut [PlaybackChannelState]) {
        for voice in &self.voices {
            if let Some(ch) = channels.get_mut(voice.channel as usize) {
                ch.active = voice.active;
                ch.sample_index = voice.sample_index;
                ch.sample_frame = voice.sample_frame;
                ch.sample_frame_fraction = voice.sample_frame_fraction;
                ch.sample_backward = voice.sample_backward;
            }
        }
    }

    pub fn render_stereo_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
        channels: &mut [PlaybackChannelState],
        mixer_mode: PlaybackMixerMode,
    ) -> PlaybackResult<RawStereoPcmFrame> {
        let mut mixed_l = 0.0;
        let mut mixed_r = 0.0;

        for voice in &mut self.voices {
            if let Some((channel_mono_pcm, pan)) =
                voice.render_sample(module, sample_rate, mixer_mode, channels)?
            {
                let right_gain = pan as f64 / 255.0;
                let left_gain = 1.0 - right_gain;
                mixed_l += channel_mono_pcm * left_gain;
                mixed_r += channel_mono_pcm * right_gain;
            }
        }

        // Write back state to keep channels in sync
        self.sync_to_channels(channels);

        let (out_l, out_r) = if mixer_mode.uses_warmth() {
            self.warmth.process(mixed_l, mixed_r, sample_rate)
        } else {
            (mixed_l, mixed_r)
        };

        Ok((out_l as i32, out_r as i32))
    }

    pub fn render_mono_frame(
        &mut self,
        module: &Module,
        sample_rate: u32,
        channels: &mut [PlaybackChannelState],
        mixer_mode: PlaybackMixerMode,
    ) -> PlaybackResult<RawMonoPcmFrame> {
        let mut mixed = PLAYBACK_MONO_SILENCE;

        for voice in &mut self.voices {
            if let Some((channel_mono_pcm, _)) =
                voice.render_sample(module, sample_rate, mixer_mode, channels)?
            {
                mixed += channel_mono_pcm as i32;
            }
        }

        // Write back state to keep channels in sync
        self.sync_to_channels(channels);

        let out = if mixer_mode.uses_warmth() {
            self.warmth.process_mono(mixed as f64, sample_rate)
        } else {
            mixed as f64
        };

        Ok(out as i32)
    }

    pub fn step_samples(
        &mut self,
        module: &Module,
        channels: &mut [PlaybackChannelState],
    ) -> PlaybackResult<Vec<ChannelSampleFrame>> {
        let mut frames = Vec::new();
        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }
            let Some(sample_index) = voice.sample_index else {
                voice.stop_sample();
                continue;
            };
            let Some(sample) = module.samples.get(sample_index) else {
                let ch = &channels[voice.channel as usize];
                return Err(PlaybackError::MissingSample {
                    channel: voice.channel,
                    instrument_index: ch.instrument_index.unwrap_or(0),
                    sample_index,
                });
            };
            let Some(value) = sample_value_at_frame(&sample.data, voice.sample_frame) else {
                voice.stop_sample();
                continue;
            };

            let sample_frame = voice.sample_frame;
            voice.advance_sample_frame(sample);
            frames.push(ChannelSampleFrame {
                channel: voice.channel,
                sample_index,
                sample_frame,
                value,
            });
        }

        // Write back state to keep channels in sync
        self.sync_to_channels(channels);

        Ok(frames)
    }
}
