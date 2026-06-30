use rustytracker_core::{FrequencyTable, Module, Note};

mod channel;
mod cursor;
mod effects;
mod envelope;
mod error;
mod flow;
mod mixer;
mod preview;
mod sample;
mod timing;
mod warmth;

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
pub use mixer::{Mixer, MixerVoice};
pub use preview::PreviewVoice;
pub use timing::{
    PlaybackTiming, PLAYBACK_MIN_BPM, PLAYBACK_MIN_TICK_SPEED, PLAYBACK_XM_TICK_NANOS_AT_ONE_BPM,
};

pub type RawMonoPcmFrame = i32;
pub type RawStereoPcmFrame = (i32, i32);

pub const PLAYBACK_MONO_SILENCE: RawMonoPcmFrame = 0;
pub const PLAYBACK_STEREO_SILENCE: RawStereoPcmFrame = (0, 0);
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

    pub fn uses_warmth(self) -> bool {
        matches!(self, Self::RustySynth)
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
