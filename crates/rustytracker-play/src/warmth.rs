//! Master-bus "warmth" for RustySynth: tanh soft-clip into a one-pole low-pass.
//! Applied to the summed stereo/mono frame; RustySynth-only.

const DRIVE: f64 = 1.0; // soft-clip drive (ear-tuned)
const CUTOFF_HZ: f64 = 12_000.0; // one-pole low-pass cutoff (ear-tuned)
const PCM_SCALE: f64 = 32_768.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MasterWarmth {
    lp_l: f64,
    lp_r: f64,
    lp_coeff: f64,
    coeff_sample_rate: u32,
}

impl Default for MasterWarmth {
    fn default() -> Self {
        Self::new()
    }
}

impl MasterWarmth {
    pub fn new() -> Self {
        Self {
            lp_l: 0.0,
            lp_r: 0.0,
            lp_coeff: 0.0,
            coeff_sample_rate: 0,
        }
    }

    fn update_coeff(&mut self, sample_rate: u32) {
        if sample_rate != self.coeff_sample_rate && sample_rate > 0 {
            self.lp_coeff =
                1.0 - (-2.0 * std::f64::consts::PI * CUTOFF_HZ / sample_rate as f64).exp();
            self.coeff_sample_rate = sample_rate;
        }
    }

    pub fn process(&mut self, l: f64, r: f64, sample_rate: u32) -> (f64, f64) {
        self.update_coeff(sample_rate);
        let sl = soft_clip(l);
        let sr = soft_clip(r);
        self.lp_l += self.lp_coeff * (sl - self.lp_l);
        self.lp_r += self.lp_coeff * (sr - self.lp_r);
        (self.lp_l, self.lp_r)
    }

    pub fn process_mono(&mut self, x: f64, sample_rate: u32) -> f64 {
        self.update_coeff(sample_rate);
        let s = soft_clip(x);
        self.lp_l += self.lp_coeff * (s - self.lp_l);
        self.lp_l
    }
}

fn soft_clip(raw: f64) -> f64 {
    let x = raw / PCM_SCALE;
    (DRIVE * x).tanh() * PCM_SCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_clip_is_odd_and_zero_at_origin() {
        assert_eq!(soft_clip(0.0), 0.0);
        assert!((soft_clip(-5000.0) + soft_clip(5000.0)).abs() < 1e-9);
    }

    #[test]
    fn soft_clip_near_unity_for_small_and_compresses_large() {
        // small input ~ passes through
        assert!((soft_clip(100.0) - 100.0).abs() < 1.0);
        // full-scale input is compressed below itself
        assert!(soft_clip(32_768.0) < 32_768.0);
        assert!(soft_clip(32_768.0) > 20_000.0);
    }

    #[test]
    fn low_pass_passes_dc_and_attenuates_alternation() {
        let sr = 44_100;
        // DC: feed a small constant; output converges to it.
        let mut w = MasterWarmth::new();
        let mut out = 0.0;
        for _ in 0..200 {
            out = w.process_mono(100.0, sr);
        }
        assert!((out - soft_clip(100.0)).abs() < 1.0);

        // Alternation: fast +/- swings come out attenuated in amplitude.
        let mut w2 = MasterWarmth::new();
        let mut peak = 0.0_f64;
        for n in 0..200 {
            let x = if n % 2 == 0 { 5000.0 } else { -5000.0 };
            let y = w2.process_mono(x, sr);
            if n > 100 {
                peak = peak.max(y.abs());
            }
        }
        assert!(peak < 5000.0, "alternation should be attenuated, got {peak}");
    }
}
