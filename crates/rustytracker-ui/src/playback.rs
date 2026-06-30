use std::sync::atomic::{
    AtomicBool, AtomicU16, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering,
};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustytracker_core::Module;
use rustytracker_play::{
    MixerTrackControl, PlaybackMixerMode, PlaybackSettings, PlaybackState, PreviewVoice,
};

const PCM16_MIN: i32 = -32_768;
const PCM16_MAX: i32 = 32_767;
const PCM16_NORMALIZATION: f32 = 32_768.0;
const POSITION_ROW_BITS: u64 = 16;
const POSITION_ROW_MASK: u64 = (1 << POSITION_ROW_BITS) - 1;
const METER_SCALE: f32 = 1_000.0;
const METER_MAX: u32 = METER_SCALE as u32;

pub(crate) enum AudioCommand {
    Play,
    Pause,
    Stop,
    UpdateModule(Module),
    SetPlayback(Option<PlaybackState>),
    SetTrackControls(Vec<MixerTrackControl>),
    PreviewNoteOn {
        instrument: u8,
        note: u8,
        mixer_mode: PlaybackMixerMode,
    },
    PreviewNoteOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaybackTransportState {
    Stopped,
    Playing,
    Paused,
}

impl PlaybackTransportState {
    fn as_u8(self) -> u8 {
        match self {
            Self::Stopped => 0,
            Self::Playing => 1,
            Self::Paused => 2,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Playing,
            2 => Self::Paused,
            _ => Self::Stopped,
        }
    }

    pub(crate) fn is_playing(self) -> bool {
        matches!(self, Self::Playing)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaybackMeterSnapshot {
    pub(crate) master_left_peak: f32,
    pub(crate) master_right_peak: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaybackStatusSnapshot {
    pub(crate) transport: PlaybackTransportState,
    pub(crate) order_index: usize,
    pub(crate) row: u16,
    pub(crate) track_activity_mask: u32,
    pub(crate) active_track: Option<u8>,
    pub(crate) device_error: bool,
    pub(crate) meters: PlaybackMeterSnapshot,
}

pub(crate) struct AudioStatus {
    pub(crate) transport: AtomicU8,
    pub(crate) is_playing: AtomicBool,
    position: AtomicU64,
    pub(crate) order_index: AtomicUsize,
    pub(crate) row: AtomicU32,
    track_activity_mask: AtomicU32,
    active_track: AtomicU16,
    pub(crate) device_error: AtomicBool,
    master_left_peak: AtomicU32,
    master_right_peak: AtomicU32,
}

impl AudioStatus {
    pub(crate) fn new() -> Self {
        Self {
            transport: AtomicU8::new(PlaybackTransportState::Stopped.as_u8()),
            is_playing: AtomicBool::new(false),
            position: AtomicU64::new(pack_position(0, 0)),
            order_index: AtomicUsize::new(0),
            row: AtomicU32::new(0),
            track_activity_mask: AtomicU32::new(0),
            active_track: AtomicU16::new(u16::MAX),
            device_error: AtomicBool::new(false),
            master_left_peak: AtomicU32::new(0),
            master_right_peak: AtomicU32::new(0),
        }
    }

    fn publish_transport(&self, transport: PlaybackTransportState) {
        self.transport.store(transport.as_u8(), Ordering::Relaxed);
        self.is_playing
            .store(transport.is_playing(), Ordering::Relaxed);
    }

    fn publish_position(&self, order_index: usize, row: u16) {
        self.position
            .store(pack_position(order_index, row), Ordering::Relaxed);
        self.order_index.store(order_index, Ordering::Relaxed);
        self.row.store(row as u32, Ordering::Relaxed);
    }

    fn publish_master_peaks(&self, left_peak: f32, right_peak: f32) {
        self.master_left_peak
            .store(encode_meter_peak(left_peak), Ordering::Relaxed);
        self.master_right_peak
            .store(encode_meter_peak(right_peak), Ordering::Relaxed);
    }

    fn publish_track_state(&self, track_activity_mask: u32, active_track: Option<u16>) {
        self.track_activity_mask
            .store(track_activity_mask, Ordering::Relaxed);
        self.active_track
            .store(active_track.unwrap_or(u16::MAX), Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> PlaybackStatusSnapshot {
        let (order_index, row) = unpack_position(self.position.load(Ordering::Relaxed));
        PlaybackStatusSnapshot {
            transport: PlaybackTransportState::from_u8(self.transport.load(Ordering::Relaxed)),
            order_index,
            row,
            track_activity_mask: self.track_activity_mask.load(Ordering::Relaxed),
            active_track: match self.active_track.load(Ordering::Relaxed) {
                value if value == u16::MAX => None,
                value => Some(value as u8),
            },
            device_error: self.device_error.load(Ordering::Relaxed),
            meters: PlaybackMeterSnapshot {
                master_left_peak: decode_meter_peak(self.master_left_peak.load(Ordering::Relaxed)),
                master_right_peak: decode_meter_peak(
                    self.master_right_peak.load(Ordering::Relaxed),
                ),
            },
        }
    }
}

struct AudioThreadState {
    playback: Option<PlaybackState>,
    module: Option<Module>,
    transport: PlaybackTransportState,
    track_controls: Vec<MixerTrackControl>,
    sample_rate: u32,
    preview: PreviewVoice,
}

pub(crate) struct AudioPlaybackEngine {
    pub(crate) producer: Mutex<rtrb::Producer<AudioCommand>>,
    pub(crate) status: Arc<AudioStatus>,
    _stream: Option<cpal::Stream>,
}

impl AudioPlaybackEngine {
    pub(crate) fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("No default audio output device found!");
                let (producer, _) = rtrb::RingBuffer::new(1);
                let status = Arc::new(AudioStatus::new());
                status.device_error.store(true, Ordering::Relaxed);
                return Self {
                    producer: Mutex::new(producer),
                    status,
                    _stream: None,
                };
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get default output config: {e}");
                let (producer, _) = rtrb::RingBuffer::new(1);
                let status = Arc::new(AudioStatus::new());
                status.device_error.store(true, Ordering::Relaxed);
                return Self {
                    producer: Mutex::new(producer),
                    status,
                    _stream: None,
                };
            }
        };

        let sample_rate = config.sample_rate().0;
        let (producer, consumer) = rtrb::RingBuffer::new(256);
        let status = Arc::new(AudioStatus::new());

        let mut consumer_opt = Some(consumer);
        let mut local_state_opt = Some(AudioThreadState {
            playback: None,
            module: None,
            transport: PlaybackTransportState::Stopped,
            track_controls: Vec::new(),
            sample_rate,
            preview: PreviewVoice::new(),
        });

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status);
                let status_err = Arc::clone(&status);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    move |err| {
                        eprintln!("an error occurred on stream: {err}");
                        status_err.device_error.store(true, Ordering::Relaxed);
                        status_err.publish_transport(PlaybackTransportState::Stopped);
                        status_err.publish_master_peaks(0.0, 0.0);
                        status_err.publish_track_state(0, None);
                    },
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status);
                let status_err = Arc::clone(&status);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    move |err| {
                        eprintln!("an error occurred on stream: {err}");
                        status_err.device_error.store(true, Ordering::Relaxed);
                        status_err.publish_transport(PlaybackTransportState::Stopped);
                        status_err.publish_master_peaks(0.0, 0.0);
                        status_err.publish_track_state(0, None);
                    },
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status);
                let status_err = Arc::clone(&status);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    move |err| {
                        eprintln!("an error occurred on stream: {err}");
                        status_err.device_error.store(true, Ordering::Relaxed);
                        status_err.publish_transport(PlaybackTransportState::Stopped);
                        status_err.publish_master_peaks(0.0, 0.0);
                        status_err.publish_track_state(0, None);
                    },
                    None,
                )
            }
            _ => Err(cpal::BuildStreamError::DeviceNotAvailable),
        };

        let stream = match stream {
            Ok(s) => {
                let _ = s.play();
                Some(s)
            }
            Err(e) => {
                eprintln!("Failed to build audio output stream: {e}");
                status.device_error.store(true, Ordering::Relaxed);
                None
            }
        };

        Self {
            producer: Mutex::new(producer),
            status,
            _stream: stream,
        }
    }

    pub(crate) fn update_module(&self, module: Module) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::UpdateModule(module));
        }
    }

    pub(crate) fn play(&self) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::Play);
        }
    }

    pub(crate) fn pause(&self) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::Pause);
        }
    }

    pub(crate) fn stop(&self) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::Stop);
        }
    }

    pub(crate) fn set_playback(&self, playback: Option<PlaybackState>) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::SetPlayback(playback));
        }
    }

    pub(crate) fn set_track_controls(&self, controls: Vec<MixerTrackControl>) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::SetTrackControls(controls));
        }
    }

    pub(crate) fn preview_note_on(&self, instrument: u8, note: u8, mixer_mode: PlaybackMixerMode) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::PreviewNoteOn {
                instrument,
                note,
                mixer_mode,
            });
        }
    }

    pub(crate) fn preview_note_off(&self) {
        if let Ok(mut prod) = self.producer.lock() {
            let _ = prod.push(AudioCommand::PreviewNoteOff);
        }
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playback_status().transport.is_playing()
    }

    pub(crate) fn device_error(&self) -> bool {
        self.playback_status().device_error
    }

    pub(crate) fn get_position(&self) -> (usize, u16) {
        let status = self.playback_status();
        (status.order_index, status.row)
    }

    pub(crate) fn playback_status(&self) -> PlaybackStatusSnapshot {
        self.status.snapshot()
    }
}

fn write_audio<T>(
    output: &mut [T],
    consumer: &mut rtrb::Consumer<AudioCommand>,
    status: &Arc<AudioStatus>,
    local_state: &mut AudioThreadState,
) where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    while let Ok(cmd) = consumer.pop() {
        match cmd {
            AudioCommand::Play => {
                local_state.transport = PlaybackTransportState::Playing;
            }
            AudioCommand::Pause => {
                local_state.transport = PlaybackTransportState::Paused;
            }
            AudioCommand::Stop => {
                local_state.transport = PlaybackTransportState::Stopped;
                local_state.playback = None;
            }
            AudioCommand::UpdateModule(module) => {
                local_state.module = Some(module);
            }
            AudioCommand::SetPlayback(playback) => {
                local_state.playback = playback;
                if let Some(pb) = &mut local_state.playback {
                    pb.set_track_controls(&local_state.track_controls);
                }
            }
            AudioCommand::SetTrackControls(controls) => {
                local_state.track_controls = controls;
                if let Some(pb) = &mut local_state.playback {
                    pb.set_track_controls(&local_state.track_controls);
                }
            }
            AudioCommand::PreviewNoteOn {
                instrument,
                note,
                mixer_mode,
            } => {
                if let Some(module) = &local_state.module {
                    let _ = local_state.preview.note_on(
                        module,
                        instrument,
                        note,
                        PlaybackSettings::with_mixer_mode(mixer_mode),
                    );
                }
            }
            AudioCommand::PreviewNoteOff => {
                local_state.preview.note_off();
            }
        }
    }

    let sample_rate = local_state.sample_rate;

    // A module is required to render either song playback or preview.
    let module = match local_state.module.as_ref() {
        Some(m) => m,
        None => {
            write_silence(output);
            status.publish_transport(PlaybackTransportState::Stopped);
            status.publish_master_peaks(0.0, 0.0);
            status.publish_track_state(0, None);
            return;
        }
    };

    let is_playing = local_state.transport.is_playing();
    let playback_opt = &mut local_state.playback;
    let preview = &mut local_state.preview;
    let mut song_ended = false;
    let mut master_left_peak = 0.0f32;
    let mut master_right_peak = 0.0f32;

    for frame in output.chunks_mut(2) {
        let (module_l, module_r) = if is_playing && !song_ended {
            match playback_opt.as_mut() {
                Some(pb) => match pb.render_raw_stereo_frame(module, sample_rate) {
                    Ok((raw_l, raw_r)) => {
                        if pb.song_ended() {
                            song_ended = true;
                            (0.0, 0.0)
                        } else {
                            (normalize_pcm16_sample(raw_l), normalize_pcm16_sample(raw_r))
                        }
                    }
                    Err(_) => {
                        song_ended = true;
                        (0.0, 0.0)
                    }
                },
                None => (0.0, 0.0),
            }
        } else {
            (0.0, 0.0)
        };

        let (preview_l, preview_r) = match preview.render_stereo_frame(module, sample_rate) {
            Ok((raw_l, raw_r)) => (normalize_pcm16_sample(raw_l), normalize_pcm16_sample(raw_r)),
            Err(_) => {
                preview.note_off();
                (0.0, 0.0)
            }
        };

        // When the preview voice is inactive its contribution is exactly 0.0,
        // and `module_l`/`module_r` are already in [-1.0, 1.0] (from
        // `normalize_pcm16_sample`), so `+ 0.0` and `clamp` are no-ops: a
        // preview-silent frame stays bit-identical to plain module output.
        // This is what keeps HiFi rendering byte-identical — do not change the
        // sum/clamp without preserving that property.
        let left = (module_l + preview_l).clamp(-1.0, 1.0);
        let right = (module_r + preview_r).clamp(-1.0, 1.0);
        master_left_peak = master_left_peak.max(left.abs());
        master_right_peak = master_right_peak.max(right.abs());

        if frame.len() >= 2 {
            frame[0] = T::from_sample(left);
            frame[1] = T::from_sample(right);
        } else if !frame.is_empty() {
            frame[0] = T::from_sample(left);
        }
    }

    if song_ended {
        local_state.transport = PlaybackTransportState::Stopped;
        local_state.playback = None;
    }

    status.publish_transport(local_state.transport);
    status.publish_master_peaks(master_left_peak, master_right_peak);
    if let Some(pb) = &local_state.playback {
        let track_activity_mask = pb.track_activity_mask();
        let active_track = track_activity_mask_to_index(track_activity_mask);
        status.publish_track_state(track_activity_mask, active_track);
    } else {
        status.publish_track_state(0, None);
    }

    if local_state.transport.is_playing() {
        if let (Some(pb), Some(module)) = (&local_state.playback, &local_state.module) {
            if let Ok(pos) = pb.clock().position(module) {
                status.publish_position(pos.order_index, pos.row);
            }
        }
    }
}

fn write_silence<T>(output: &mut [T])
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    for sample in output.iter_mut() {
        *sample = T::from_sample(0.0);
    }
}

fn normalize_pcm16_sample(sample: i32) -> f32 {
    sample.clamp(PCM16_MIN, PCM16_MAX) as f32 / PCM16_NORMALIZATION
}

fn pack_position(order_index: usize, row: u16) -> u64 {
    ((order_index as u64) << POSITION_ROW_BITS) | row as u64
}

fn unpack_position(position: u64) -> (usize, u16) {
    let order_index = (position >> POSITION_ROW_BITS) as usize;
    let row = (position & POSITION_ROW_MASK) as u16;
    (order_index, row)
}

fn encode_meter_peak(peak: f32) -> u32 {
    (peak.clamp(0.0, 1.0) * METER_SCALE) as u32
}

fn decode_meter_peak(peak: u32) -> f32 {
    peak.min(METER_MAX) as f32 / METER_SCALE
}

fn track_activity_mask_to_index(mask: u32) -> Option<u16> {
    if mask == 0 {
        None
    } else {
        Some(mask.trailing_zeros() as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustytracker_core::{FrequencyTable, Module, SampleData};
    use rustytracker_play::PlaybackSettings;

    #[test]
    fn audio_status_device_error_atomic_plumbing() {
        let status = Arc::new(AudioStatus::new());
        assert!(
            !status.device_error.load(Ordering::Relaxed),
            "fresh AudioStatus should have device_error == false"
        );
        status.device_error.store(true, Ordering::Relaxed);
        assert!(
            status.device_error.load(Ordering::Relaxed),
            "device_error should be true after store"
        );
    }

    #[test]
    fn audio_status_snapshot_reports_transport_position_and_meters() {
        let status = AudioStatus::new();

        assert_eq!(
            status.snapshot(),
            PlaybackStatusSnapshot {
                transport: PlaybackTransportState::Stopped,
                order_index: 0,
                row: 0,
                track_activity_mask: 0,
                active_track: None,
                device_error: false,
                meters: PlaybackMeterSnapshot {
                    master_left_peak: 0.0,
                    master_right_peak: 0.0,
                },
            }
        );

        status.publish_transport(PlaybackTransportState::Playing);
        status.publish_position(3, 42);
        status.publish_master_peaks(0.25, 1.25);

        let snapshot = status.snapshot();
        assert_eq!(snapshot.transport, PlaybackTransportState::Playing);
        assert_eq!(snapshot.order_index, 3);
        assert_eq!(snapshot.row, 42);
        assert_eq!(snapshot.meters.master_left_peak, 0.25);
        assert_eq!(snapshot.meters.master_right_peak, 1.0);

        status.publish_transport(PlaybackTransportState::Paused);
        let snapshot = status.snapshot();
        assert_eq!(snapshot.transport, PlaybackTransportState::Paused);
        assert!(!snapshot.transport.is_playing());
    }

    #[test]
    fn audio_status_snapshot_includes_track_activity() {
        let status = AudioStatus::new();
        status.publish_track_state(0b1010, Some(3));

        let snapshot = status.snapshot();
        assert_eq!(snapshot.track_activity_mask, 0b1010);
        assert_eq!(snapshot.active_track, Some(3));
    }

    #[test]
    fn track_activity_mask_to_index_picks_first_active_track() {
        assert_eq!(track_activity_mask_to_index(0), None);
        assert_eq!(track_activity_mask_to_index(0b0001), Some(0));
        assert_eq!(track_activity_mask_to_index(0b0010), Some(1));
        assert_eq!(track_activity_mask_to_index(0b1010), Some(1));
    }

    #[test]
    fn normalize_pcm16_sample_clamps_to_output_range() {
        assert_eq!(normalize_pcm16_sample(0), 0.0);
        assert_eq!(normalize_pcm16_sample(PCM16_MIN), -1.0);
        assert_eq!(
            normalize_pcm16_sample(PCM16_MAX + 1),
            PCM16_MAX as f32 / PCM16_NORMALIZATION
        );
    }

    fn module_with_preview_sample() -> Module {
        let mut module = Module::empty_with_channels(2).unwrap();
        module.header.frequency_table = FrequencyTable::Linear;
        let map_len = module.instruments[0].note_sample_map.len().max(96);
        module.instruments[0].note_sample_map = vec![Some(0); map_len];
        module.samples[0].volume = 255;
        module.samples[0].data = SampleData::pcm16(vec![12_000; 64]);
        module
    }

    #[test]
    fn write_audio_mixes_preview_while_module_stopped() {
        let module = module_with_preview_sample();
        let mut local_state = AudioThreadState {
            playback: None,
            module: Some(module.clone()),
            transport: PlaybackTransportState::Stopped,
            track_controls: Vec::new(),
            sample_rate: 44_100,
            preview: PreviewVoice::new(),
        };
        local_state
            .preview
            .note_on(&module, 1, 49, PlaybackSettings::default())
            .unwrap();

        let status = Arc::new(AudioStatus::new());
        let (_producer, mut consumer) = rtrb::RingBuffer::<AudioCommand>::new(4);
        let mut output = vec![0.0f32; 64];

        write_audio(&mut output, &mut consumer, &status, &mut local_state);

        assert!(
            output.iter().any(|&sample| sample != 0.0),
            "preview voice should be mixed into output even when the song is stopped"
        );
        let snapshot = status.snapshot();
        assert!(
            snapshot.meters.master_left_peak > 0.0 || snapshot.meters.master_right_peak > 0.0,
            "mixed preview audio should publish master peak meters"
        );
    }

    #[test]
    fn write_audio_publishes_transport_commands() {
        let module = module_with_preview_sample();
        let mut local_state = AudioThreadState {
            playback: None,
            module: Some(module),
            transport: PlaybackTransportState::Stopped,
            track_controls: Vec::new(),
            sample_rate: 44_100,
            preview: PreviewVoice::new(),
        };
        let status = Arc::new(AudioStatus::new());
        let (mut producer, mut consumer) = rtrb::RingBuffer::<AudioCommand>::new(4);
        let mut output = vec![0.0f32; 8];

        producer.push(AudioCommand::Play).unwrap();
        write_audio(&mut output, &mut consumer, &status, &mut local_state);
        assert_eq!(status.snapshot().transport, PlaybackTransportState::Playing);
        assert!(status.is_playing.load(Ordering::Relaxed));

        producer.push(AudioCommand::Pause).unwrap();
        write_audio(&mut output, &mut consumer, &status, &mut local_state);
        assert_eq!(status.snapshot().transport, PlaybackTransportState::Paused);
        assert!(!status.is_playing.load(Ordering::Relaxed));

        producer.push(AudioCommand::Stop).unwrap();
        write_audio(&mut output, &mut consumer, &status, &mut local_state);
        assert_eq!(status.snapshot().transport, PlaybackTransportState::Stopped);
        assert!(!status.is_playing.load(Ordering::Relaxed));
    }
}
