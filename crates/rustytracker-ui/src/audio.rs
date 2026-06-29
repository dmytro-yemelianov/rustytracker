use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustytracker_core::Module;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState, PreviewVoice};

const PCM16_MIN: i32 = -32_768;
const PCM16_MAX: i32 = 32_767;
const PCM16_NORMALIZATION: f32 = 32_768.0;

pub(crate) enum AudioCommand {
    Play,
    Pause,
    Stop,
    UpdateModule(Module),
    SetPlayback(Option<PlaybackState>),
    PreviewNoteOn {
        instrument: u8,
        note: u8,
        mixer_mode: PlaybackMixerMode,
    },
    PreviewNoteOff,
}

pub(crate) struct AudioStatus {
    pub(crate) is_playing: AtomicBool,
    pub(crate) order_index: AtomicUsize,
    pub(crate) row: AtomicU32,
}

impl AudioStatus {
    pub(crate) fn new() -> Self {
        Self {
            is_playing: AtomicBool::new(false),
            order_index: AtomicUsize::new(0),
            row: AtomicU32::new(0),
        }
    }
}

struct AudioThreadState {
    playback: Option<PlaybackState>,
    module: Option<Module>,
    is_playing: bool,
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
                return Self {
                    producer: Mutex::new(producer),
                    status: Arc::new(AudioStatus::new()),
                    _stream: None,
                };
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get default output config: {e}");
                let (producer, _) = rtrb::RingBuffer::new(1);
                return Self {
                    producer: Mutex::new(producer),
                    status: Arc::new(AudioStatus::new()),
                    _stream: None,
                };
            }
        };

        let sample_rate = config.sample_rate().0;
        let (producer, consumer) = rtrb::RingBuffer::new(256);
        let status = Arc::new(AudioStatus::new());

        let status_clone = Arc::clone(&status);
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");

        let mut consumer_opt = Some(consumer);
        let mut local_state_opt = Some(AudioThreadState {
            playback: None,
            module: None,
            is_playing: false,
            sample_rate,
            preview: PreviewVoice::new(),
        });

        let status_inner_clone = Arc::clone(&status_clone);
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status_inner_clone);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status_inner_clone);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut consumer = consumer_opt.take().unwrap();
                let mut local_state = local_state_opt.take().unwrap();
                let status_inner = Arc::clone(&status_inner_clone);
                device.build_output_stream(
                    &config.into(),
                    move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                        write_audio(data, &mut consumer, &status_inner, &mut local_state);
                    },
                    err_fn,
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
        self.status.is_playing.load(Ordering::Relaxed)
    }

    pub(crate) fn get_position(&self) -> (usize, u16) {
        let order_index = self.status.order_index.load(Ordering::Relaxed);
        let row = self.status.row.load(Ordering::Relaxed) as u16;
        (order_index, row)
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
                local_state.is_playing = true;
            }
            AudioCommand::Pause => {
                local_state.is_playing = false;
            }
            AudioCommand::Stop => {
                local_state.is_playing = false;
                local_state.playback = None;
            }
            AudioCommand::UpdateModule(module) => {
                local_state.module = Some(module);
            }
            AudioCommand::SetPlayback(playback) => {
                local_state.playback = playback;
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
            status.is_playing.store(false, Ordering::Relaxed);
            return;
        }
    };

    let is_playing = local_state.is_playing;
    let playback_opt = &mut local_state.playback;
    let preview = &mut local_state.preview;
    let mut song_ended = false;

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

        let left = (module_l + preview_l).clamp(-1.0, 1.0);
        let right = (module_r + preview_r).clamp(-1.0, 1.0);

        if frame.len() >= 2 {
            frame[0] = T::from_sample(left);
            frame[1] = T::from_sample(right);
        } else if !frame.is_empty() {
            frame[0] = T::from_sample(left);
        }
    }

    if song_ended {
        local_state.is_playing = false;
        local_state.playback = None;
    }

    status
        .is_playing
        .store(local_state.is_playing, Ordering::Relaxed);

    if local_state.is_playing {
        if let (Some(pb), Some(module)) = (&local_state.playback, &local_state.module) {
            if let Ok(pos) = pb.clock().position(module) {
                status.order_index.store(pos.order_index, Ordering::Relaxed);
                status.row.store(pos.row as u32, Ordering::Relaxed);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rustytracker_core::{FrequencyTable, Module, SampleData};
    use rustytracker_play::PlaybackSettings;

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
            is_playing: false,
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
    }
}
