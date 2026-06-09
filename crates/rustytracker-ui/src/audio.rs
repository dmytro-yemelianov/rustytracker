use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustytracker_core::Module;
use rustytracker_play::PlaybackState;

const FALLBACK_SAMPLE_RATE: u32 = 44_100;
const PCM16_MIN: i32 = -32_768;
const PCM16_MAX: i32 = 32_767;
const PCM16_NORMALIZATION: f32 = 32_768.0;

pub(crate) struct AudioEngineState {
    pub(crate) playback: Option<PlaybackState>,
    pub(crate) module: Option<Module>,
    pub(crate) is_playing: bool,
    sample_rate: u32,
}

pub(crate) struct AudioPlaybackEngine {
    pub(crate) state: Arc<Mutex<AudioEngineState>>,
    _stream: Option<cpal::Stream>,
}

impl AudioPlaybackEngine {
    pub(crate) fn new() -> Self {
        let host = cpal::default_host();
        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("No default audio output device found!");
                return Self {
                    state: Arc::new(Mutex::new(AudioEngineState::silent(FALLBACK_SAMPLE_RATE))),
                    _stream: None,
                };
            }
        };

        let config = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get default output config: {e}");
                return Self {
                    state: Arc::new(Mutex::new(AudioEngineState::silent(FALLBACK_SAMPLE_RATE))),
                    _stream: None,
                };
            }
        };

        let sample_rate = config.sample_rate().0;
        let state = Arc::new(Mutex::new(AudioEngineState::silent(sample_rate)));

        let state_clone = Arc::clone(&state);
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    write_audio(data, &state_clone);
                },
                err_fn,
                None,
            ),
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
            state,
            _stream: stream,
        }
    }
}

impl AudioEngineState {
    fn silent(sample_rate: u32) -> Self {
        Self {
            playback: None,
            module: None,
            is_playing: false,
            sample_rate,
        }
    }
}

fn write_audio<T>(output: &mut [T], state_lock: &Arc<Mutex<AudioEngineState>>)
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    let mut state_guard = match state_lock.lock() {
        Ok(s) => s,
        Err(_) => {
            write_silence(output);
            return;
        }
    };

    let state = &mut *state_guard;
    if !state.is_playing {
        write_silence(output);
        return;
    }

    let AudioEngineState {
        playback,
        module,
        sample_rate,
        ..
    } = state;

    let playback = match playback {
        Some(pb) => pb,
        None => {
            write_silence(output);
            return;
        }
    };

    let module = match module {
        Some(m) => m,
        None => {
            write_silence(output);
            return;
        }
    };

    let sample_rate = *sample_rate;
    let mut song_ended = false;

    for frame in output.chunks_mut(2) {
        let (left_sample, right_sample) = if !song_ended {
            match playback.render_raw_stereo_frame(module, sample_rate) {
                Ok((raw_l, raw_r)) => {
                    if playback.song_ended() {
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
            }
        } else {
            (0.0, 0.0)
        };

        if frame.len() >= 2 {
            frame[0] = T::from_sample(left_sample);
            frame[1] = T::from_sample(right_sample);
        } else if !frame.is_empty() {
            frame[0] = T::from_sample(left_sample);
        }
    }

    if song_ended {
        state.is_playing = false;
        state.playback = None;
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

    #[test]
    fn normalize_pcm16_sample_clamps_to_output_range() {
        assert_eq!(normalize_pcm16_sample(0), 0.0);
        assert_eq!(normalize_pcm16_sample(PCM16_MIN), -1.0);
        assert_eq!(
            normalize_pcm16_sample(PCM16_MAX + 1),
            PCM16_MAX as f32 / PCM16_NORMALIZATION
        );
    }
}
