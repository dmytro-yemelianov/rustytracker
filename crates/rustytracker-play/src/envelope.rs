use rustytracker_core::Envelope;

pub(crate) const XM_ENVELOPE_ENABLED_FLAG: u8 = 0x01;
const XM_ENVELOPE_SUSTAIN_FLAG: u8 = 0x02;
const XM_ENVELOPE_LOOP_FLAG: u8 = 0x04;

pub(crate) const PLAYBACK_DEFAULT_FADEOUT_VOLUME: u32 = 65536;
const PLAYBACK_ENVELOPE_INTERPOLATION_SHIFT: u32 = 16;
pub(crate) const PLAYBACK_ENVELOPE_DEFAULT_VOLUME: u16 = 256;
pub(crate) const PLAYBACK_ENVELOPE_DEFAULT_PANNING: u16 = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackEnvelopeState {
    pub a: usize,
    pub b: usize,
    pub step: u16,
}

impl PlaybackEnvelopeState {
    pub fn new() -> Self {
        Self {
            a: 0,
            b: 1,
            step: 0,
        }
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.b = 1;
        self.step = 0;
    }

    pub fn advance(&mut self, env: &Envelope, keyon: bool) {
        if env.points.is_empty() || (env.flags & XM_ENVELOPE_ENABLED_FLAG) == 0 {
            return;
        }

        let num = env.points.len();

        let is_sustain_point = (env.flags & XM_ENVELOPE_SUSTAIN_FLAG) != 0
            && self.a == env.sustain_point as usize
            && self.a < num
            && self.step == env.points[self.a].frame;
        if is_sustain_point && keyon {
            return;
        }

        if self.b < num && self.step != env.points[self.b].frame {
            self.step += 1;
        }

        if self.b < num && self.step == env.points[self.b].frame {
            if (env.flags & XM_ENVELOPE_LOOP_FLAG) != 0 {
                let break_loop = !keyon
                    && (env.flags & XM_ENVELOPE_SUSTAIN_FLAG) != 0
                    && env.sustain_point == env.loop_end_point;

                if !break_loop && self.b == env.loop_end_point as usize {
                    self.a = env.loop_start_point as usize;
                    self.b = (env.loop_start_point + 1) as usize;
                    if self.a < num {
                        self.step = env.points[self.a].frame;
                    }
                    return;
                }
            }

            if self.b < num - 1 {
                self.a += 1;
                self.b += 1;
            }
        }
    }

    pub fn get_value(&self, env: &Envelope, default_val: u16) -> u16 {
        if env.points.is_empty() || (env.flags & XM_ENVELOPE_ENABLED_FLAG) == 0 {
            return default_val;
        }

        let num = env.points.len();
        let idx_a = self.a.min(num - 1);
        let idx_b = self.b.min(num - 1);

        if idx_a == idx_b {
            return env.points[idx_a].value;
        }

        let p_a = env.points[idx_a];
        let p_b = env.points[idx_b];

        let mut dx = p_b.frame as i32 - p_a.frame as i32;
        if dx == 0 {
            dx = 1;
        }

        let t = (p_b.frame as i32 - self.step as i32) * PLAYBACK_DEFAULT_FADEOUT_VOLUME as i32 / dx;
        let y0 = p_a.value as i32;
        let y1 = p_b.value as i32;

        let y = (y0 * t) + (y1 * (PLAYBACK_DEFAULT_FADEOUT_VOLUME as i32 - t));
        (y >> PLAYBACK_ENVELOPE_INTERPOLATION_SHIFT) as u16
    }
}

impl Default for PlaybackEnvelopeState {
    fn default() -> Self {
        Self::new()
    }
}
