use rustytracker_core::{
    FrequencyTable, Module, Note, Sample, SampleData, SampleLoopKind, SAMPLE_DEFAULT_PANNING,
};

mod channel;
mod cursor;
mod effects;
mod envelope;
mod error;
mod flow;
mod preview;
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
pub use preview::PreviewVoice;
pub use timing::{
    PlaybackTiming, PLAYBACK_MIN_BPM, PLAYBACK_MIN_TICK_SPEED, PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM,
};

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const PLAYBACK_STEREO_SILENCE: RawStereoPcmFrame = (0, 0);
const AMIGA_PAL_CLOCK_HZ: f64 = 14_187_580.0;
const AMIGA_NTSC_CLOCK_HZ: f64 = 14_317_056.0;
const MILKY_MIXER_BASE_FREQUENCY: i64 = 48_000;
const MILKY_MIXER_TIMER_FREQUENCY: i64 = 250;
const MILKY_MIXER_BEAT_LENGTH: i64 = MILKY_MIXER_BASE_FREQUENCY / MILKY_MIXER_TIMER_FREQUENCY;
const MILKY_BPM_TICK_BASE: i64 = 625;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequencerCommand {
    Trigger {
        channel: u16,
        sample_index: usize,
        instrument_index: usize,
        note: Note,
        instrument: u8,
        volume: u8,
        panning: u8,
        period: u32,
        offset: Option<usize>,
    },
    Update {
        channel: u16,
        volume: u8,
        panning: u8,
        period: u32,
        volume_envelope_val: u16,
        panning_envelope_val: u16,
        fadeout_volume: u32,
        keyon: bool,
    },
    Stop {
        channel: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sequencer {
    pub clock: PlaybackClock,
    pub channels: Vec<PlaybackChannelState>,
    pub song_ended: bool,
}

impl Sequencer {
    pub fn start_with_config(module: &Module) -> PlaybackResult<Self> {
        let clock = PlaybackClock::start(module)?;
        let row_state = clock.row_state(module)?;
        let channels = row_state
            .channels
            .iter()
            .map(|channel| PlaybackChannelState::empty(channel.channel))
            .collect();
        let mut seq = Self {
            clock,
            channels,
            song_ended: false,
        };
        seq.apply_row_state(module, &row_state)?;
        Ok(seq)
    }

    pub fn advance_tick(
        &mut self,
        module: &Module,
    ) -> PlaybackResult<(TickAdvance, Vec<SequencerCommand>)> {
        if self.song_ended {
            return Ok((TickAdvance::SongEnd, Vec::new()));
        }

        // Reset transient flags
        for ch in &mut self.channels {
            ch.triggered = None;
            ch.stopped = false;
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

        // Generate commands
        let mut commands = Vec::new();
        for channel in &self.channels {
            if channel.stopped {
                commands.push(SequencerCommand::Stop {
                    channel: channel.channel,
                });
            } else if let Some(offset) = channel.triggered {
                if let (Some(sample_index), Some(instrument_index)) =
                    (channel.sample_index, channel.instrument_index)
                {
                    commands.push(SequencerCommand::Trigger {
                        channel: channel.channel,
                        sample_index,
                        instrument_index,
                        note: channel.note,
                        instrument: channel.instrument,
                        volume: channel.volume,
                        panning: channel.panning,
                        period: channel.period,
                        offset,
                    });
                }
            } else if channel.active {
                commands.push(SequencerCommand::Update {
                    channel: channel.channel,
                    volume: channel.volume,
                    panning: channel.panning,
                    period: channel.period,
                    volume_envelope_val: channel.volume_envelope_val,
                    panning_envelope_val: channel.panning_envelope_val,
                    fadeout_volume: channel.fadeout_volume,
                    keyon: channel.keyon,
                });
            }
        }

        Ok((advance, commands))
    }

    pub fn generate_initial_commands(&self) -> Vec<SequencerCommand> {
        let mut commands = Vec::new();
        for channel in &self.channels {
            if channel.active {
                if let (Some(sample_index), Some(instrument_index)) =
                    (channel.sample_index, channel.instrument_index)
                {
                    commands.push(SequencerCommand::Trigger {
                        channel: channel.channel,
                        sample_index,
                        instrument_index,
                        note: channel.note,
                        instrument: channel.instrument,
                        volume: channel.volume,
                        panning: channel.panning,
                        period: channel.period,
                        offset: if channel.sample_frame > 0 {
                            Some(channel.sample_frame)
                        } else {
                            None
                        },
                    });
                }
            }
        }
        commands
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
        let mut requested_order = None;
        let mut requested_row = None;

        for channel in &row_state.channels {
            for effect in &channel.cell.effects {
                if effect.effect == EFFECT_SET_SPEED_BPM {
                    if effect.operand == 0 {
                        self.song_ended = true;
                    } else if effect.operand < SPEED_BPM_THRESHOLD {
                        self.clock.set_tick_speed(u16::from(effect.operand))?;
                    } else {
                        self.clock.set_bpm(u16::from(effect.operand))?;
                    }
                } else if effect.effect == EFFECT_POSITION_JUMP {
                    requested_order = Some(usize::from(effect.operand));
                } else if effect.effect == EFFECT_PATTERN_BREAK {
                    let bcd = effect.operand;
                    let row = u16::from(bcd >> 4) * 10 + u16::from(bcd & 0x0f);
                    requested_row = Some(row);
                }
            }
        }

        if requested_order.is_some() || requested_row.is_some() {
            let current_pos = self.clock.position(module)?;
            let target_order = match requested_order {
                Some(order) => order,
                None => {
                    let next_order = current_pos.order_index + 1;
                    if next_order >= module.orders.len() {
                        usize::from(module.header.restart_position)
                    } else {
                        next_order
                    }
                }
            };
            let target_row = requested_row.unwrap_or_default();
            self.clock.set_jump_target(PlaybackPosition {
                order_index: target_order,
                pattern_index: usize::from(module.orders[target_order]),
                row: target_row,
            });
        }

        for channel in &row_state.channels {
            let ch_state = &mut self.channels[usize::from(channel.channel)];
            ch_state.apply_cell(module, &channel.cell)?;
            ch_state.process_tick_effects(module, 0);
        }

        Ok(())
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mixer {
    pub voices: Vec<MixerVoice>,
}

impl Mixer {
    pub fn new(channel_count: usize) -> Self {
        let voices = (0..channel_count)
            .map(|ch| MixerVoice::empty(ch as u16))
            .collect();
        Self { voices }
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
                    volume,
                    panning,
                    period,
                    volume_envelope_val,
                    panning_envelope_val,
                    fadeout_volume,
                    keyon,
                } => {
                    let voice = &mut self.voices[channel as usize];
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
            if sample.data.frame_count() == 0 || voice.sample_frame >= sample.data.frame_count() {
                voice.stop_sample();
                continue;
            }

            let frequency =
                period_to_frequency(voice.period, module.header.frequency_table, mixer_mode);
            let step = frequency / sample_rate as f64;

            let sample_val = get_sample_value(
                &sample.data,
                voice.sample_frame,
                voice.sample_frame_fraction,
                sample,
                mixer_mode,
            );

            let vol_factor = (voice.volume as f64 / 255.0)
                * (voice.volume_envelope_val as f64 / 256.0)
                * (voice.fadeout_volume as f64 / 65536.0);

            let channel_mono_pcm = sample_val * vol_factor;

            let mut pan = voice.panning as i32 + voice.panning_envelope_val as i32 - 128;
            pan = pan.clamp(0, 255);

            let right_gain = pan as f64 / 255.0;
            let left_gain = 1.0 - right_gain;

            mixed_l += channel_mono_pcm * left_gain;
            mixed_r += channel_mono_pcm * right_gain;

            voice.advance_sample_position(sample, step);
        }

        // Write back state to keep channels in sync
        self.sync_to_channels(channels);

        Ok((mixed_l as i32, mixed_r as i32))
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
            if sample.data.frame_count() == 0 || voice.sample_frame >= sample.data.frame_count() {
                voice.stop_sample();
                continue;
            }

            let frequency =
                period_to_frequency(voice.period, module.header.frequency_table, mixer_mode);
            let step = frequency / sample_rate as f64;

            let sample_val = get_sample_value(
                &sample.data,
                voice.sample_frame,
                voice.sample_frame_fraction,
                sample,
                mixer_mode,
            );

            let vol_factor = (voice.volume as f64 / 255.0)
                * (voice.volume_envelope_val as f64 / 256.0)
                * (voice.fadeout_volume as f64 / 65536.0);

            let channel_mono_pcm = sample_val * vol_factor;
            mixed += channel_mono_pcm as i32;

            voice.advance_sample_position(sample, step);
        }

        // Write back state to keep channels in sync
        self.sync_to_channels(channels);

        Ok(mixed)
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

fn period_to_frequency(period: u32, table: FrequencyTable, mixer_mode: PlaybackMixerMode) -> f64 {
    if period == 0 {
        return 0.0;
    }

    match table {
        FrequencyTable::Linear => 8363.0 * f64::powf(2.0, (4608.0 - period as f64) / 768.0),
        FrequencyTable::Amiga => {
            let base = if mixer_mode.uses_pal_clock() {
                AMIGA_PAL_CLOCK_HZ
            } else {
                AMIGA_NTSC_CLOCK_HZ
            };
            base / period as f64
        }
    }
}

fn get_sample_value(
    data: &SampleData,
    frame: usize,
    fraction: u32,
    sample: &Sample,
    mixer_mode: PlaybackMixerMode,
) -> f64 {
    match mixer_mode.interpolation() {
        Interpolation::Linear => get_sample_value_linear(data, frame, fraction, sample),
        Interpolation::Cubic => get_sample_value_cubic(data, frame, fraction, sample),
        Interpolation::Stepped => sample_value_as_f64(data, frame),
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

/// Loop-aware sample index `offset` frames from `frame`.
/// Generalizes `next_frame_index` (which is `tap_index(frame, 1, _)`) to
/// arbitrary offsets for cubic interpolation. Kept separate so the linear
/// path stays byte-identical. `None` means "past the end of a non-looping
/// sample" (caller treats it as 0.0); negative targets clamp to frame 0.
fn tap_index(frame: usize, offset: i64, sample: &Sample) -> Option<usize> {
    let frame_count = sample.data.frame_count();
    if frame_count == 0 {
        return None;
    }
    let target = frame as i64 + offset;
    if target < 0 {
        return Some(0);
    }
    let target = target as usize;
    let has_loop = sample.loop_length > 0 && sample.loop_kind != SampleLoopKind::None;
    if has_loop {
        let loop_start = sample.loop_start as usize;
        let loop_length = sample.loop_length as usize;
        let loop_end = loop_start + loop_length;
        if target >= loop_end {
            Some(loop_start + (target - loop_end) % loop_length)
        } else {
            Some(target)
        }
    } else if target >= frame_count {
        None
    } else {
        Some(target)
    }
}

fn tap_value(data: &SampleData, frame: usize, offset: i64, sample: &Sample) -> f64 {
    match tap_index(frame, offset, sample) {
        Some(index) => sample_value_as_f64(data, index),
        None => 0.0,
    }
}

fn catmull_rom(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
}

fn get_sample_value_cubic(data: &SampleData, frame: usize, fraction: u32, sample: &Sample) -> f64 {
    let t = fraction as f64 / u32::MAX as f64;
    let p0 = tap_value(data, frame, -1, sample);
    let p1 = sample_value_as_f64(data, frame);
    let p2 = tap_value(data, frame, 1, sample);
    let p3 = tap_value(data, frame, 2, sample);
    catmull_rom(p0, p1, p2, p3, t)
}

fn sample_value_as_f64(data: &SampleData, index: usize) -> f64 {
    match data {
        SampleData::Empty => 0.0,
        SampleData::Pcm8(values) => values
            .get(index)
            .map(|value| (i32::from(*value) << PLAYBACK_PCM8_TO_I16_SHIFT) as f64)
            .unwrap_or_default(),
        SampleData::Pcm16(values) => values
            .get(index)
            .map(|value| f64::from(*value))
            .unwrap_or_default(),
    }
}

fn sample_value_at_frame(data: &SampleData, frame: usize) -> Option<PlaybackSampleValue> {
    match data {
        SampleData::Empty => None,
        SampleData::Pcm8(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm8),
        SampleData::Pcm16(values) => values.get(frame).copied().map(PlaybackSampleValue::Pcm16),
    }
}

#[cfg(test)]
mod cubic_tests {
    use super::*;
    use rustytracker_core::{Sample, SampleData, SampleLoopKind};

    fn ramp_sample(len: usize, looped: bool) -> Sample {
        let mut s = Sample::default();
        s.data = SampleData::pcm16((0..len as i16).collect());
        if looped {
            s.loop_kind = SampleLoopKind::Forward;
            s.loop_start = 2;
            s.loop_length = (len as u32).saturating_sub(2);
        }
        s
    }

    #[test]
    fn catmull_rom_endpoints() {
        assert!((catmull_rom(0.0, 7.0, 9.0, 3.0, 0.0) - 7.0).abs() < 1e-9);
        assert!((catmull_rom(0.0, 7.0, 9.0, 3.0, 1.0) - 9.0).abs() < 1e-9);
    }

    #[test]
    fn catmull_rom_known_midpoint() {
        // p0=0,p1=1,p2=1,p3=0 at t=0.5 => 1.125 (curvature overshoot)
        assert!((catmull_rom(0.0, 1.0, 1.0, 0.0, 0.5) - 1.125).abs() < 1e-9);
    }

    #[test]
    fn tap_index_non_looping_clamps_and_ends() {
        let s = ramp_sample(8, false);
        assert_eq!(tap_index(0, -1, &s), Some(0)); // before start clamps to 0
        assert_eq!(tap_index(3, 1, &s), Some(4));
        assert_eq!(tap_index(7, 1, &s), None); // past end -> None (caller uses 0.0)
        assert_eq!(tap_index(7, 2, &s), None);
    }

    #[test]
    fn tap_index_forward_loop_wraps() {
        // len 8, loop_start 2, loop_length 6 => loop_end 8
        let s = ramp_sample(8, true);
        assert_eq!(tap_index(7, 1, &s), Some(2)); // wraps to loop_start
        assert_eq!(tap_index(7, 2, &s), Some(3));
    }
}
