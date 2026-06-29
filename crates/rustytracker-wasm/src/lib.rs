use rustytracker_core::Module;
use rustytracker_play::{PlaybackMixerMode, PlaybackSettings, PlaybackState};
use rustytracker_xm::{XM_HEADER_SIGNATURE, XM_HEADER_SIGNATURE_LENGTH};
use wasm_bindgen::prelude::*;

const PCM16_MIN: i32 = -32_768;
const PCM16_MAX: i32 = 32_767;
const PCM16_NORMALIZATION: f32 = 32_768.0;

#[wasm_bindgen]
pub struct RustyTrackerWasmEngine {
    playback: PlaybackState,
    module: Module,
}

#[wasm_bindgen]
impl RustyTrackerWasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new(module_bytes: &[u8]) -> Result<RustyTrackerWasmEngine, JsValue> {
        Self::new_with_mixer(module_bytes, PlaybackMixerMode::HiFi.cli_name())
    }

    pub fn new_with_mixer(
        module_bytes: &[u8],
        mixer_mode: &str,
    ) -> Result<RustyTrackerWasmEngine, JsValue> {
        let mixer_mode = PlaybackMixerMode::from_name(mixer_mode)
            .ok_or_else(|| JsValue::from_str("Invalid mixer mode"))?;
        let module = if module_bytes.len() >= XM_HEADER_SIGNATURE_LENGTH
            && &module_bytes[..XM_HEADER_SIGNATURE_LENGTH] == XM_HEADER_SIGNATURE
        {
            rustytracker_xm::parse_xm_module(module_bytes)
                .map_err(|e| JsValue::from_str(&format!("XM parse err: {e:?}")))?
        } else {
            rustytracker_mod::parse_mod_module(module_bytes)
                .map_err(|e| JsValue::from_str(&format!("MOD parse err: {e:?}")))?
        };

        let playback = PlaybackState::start_with_settings(
            &module,
            PlaybackSettings::with_mixer_mode(mixer_mode),
        )
        .map_err(|e| JsValue::from_str(&format!("Playback start err: {e:?}")))?;

        Ok(Self { playback, module })
    }

    pub fn render_stereo(&mut self, sample_rate: u32, out_l: &mut [f32], out_r: &mut [f32]) {
        for i in 0..out_l.len() {
            match self
                .playback
                .render_raw_stereo_frame(&self.module, sample_rate)
            {
                Ok((left_i32, right_i32)) => {
                    out_l[i] = normalize_pcm16_sample(left_i32);
                    if i < out_r.len() {
                        out_r[i] = normalize_pcm16_sample(right_i32);
                    }
                }
                Err(_) => {
                    out_l[i..].fill(0.0);
                    if i < out_r.len() {
                        out_r[i..].fill(0.0);
                    }
                    return;
                }
            }
        }

        if out_r.len() > out_l.len() {
            out_r[out_l.len()..].fill(0.0);
        }
    }

    pub fn song_ended(&self) -> bool {
        self.playback.song_ended()
    }

    pub fn current_order(&self) -> usize {
        self.playback.clock().cursor().order_index()
    }

    pub fn current_row(&self) -> u16 {
        self.playback.clock().cursor().row()
    }

    pub fn current_tick(&self) -> u16 {
        self.playback.clock().tick()
    }
}

fn normalize_pcm16_sample(sample: i32) -> f32 {
    sample.clamp(PCM16_MIN, PCM16_MAX) as f32 / PCM16_NORMALIZATION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_pcm16_sample_clamps_for_web_audio() {
        assert_eq!(normalize_pcm16_sample(0), 0.0);
        assert_eq!(normalize_pcm16_sample(PCM16_MIN), -1.0);
        assert_eq!(
            normalize_pcm16_sample(PCM16_MAX + 1),
            PCM16_MAX as f32 / PCM16_NORMALIZATION
        );
    }
}
