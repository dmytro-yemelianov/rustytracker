use rustytracker_core::{FrequencyTable, Sample, SampleData, SampleLoopKind};

use crate::channel::{PlaybackSampleValue, PLAYBACK_PCM8_TO_I16_SHIFT};
use crate::{Interpolation, PlaybackMixerMode};

const AMIGA_PAL_CLOCK_HZ: f64 = 14_187_580.0;
const AMIGA_NTSC_CLOCK_HZ: f64 = 14_317_056.0;

pub(crate) fn period_to_frequency(
    period: u32,
    table: FrequencyTable,
    mixer_mode: PlaybackMixerMode,
) -> f64 {
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

pub(crate) fn get_sample_value(
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
        if frame >= loop_start {
            // In the loop region: wrap the tap into [loop_start, loop_end).
            let rel = (target as i64 - loop_start as i64).rem_euclid(loop_length as i64);
            Some((loop_start as i64 + rel) as usize)
        } else if target >= loop_end {
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

pub(crate) fn sample_value_at_frame(
    data: &SampleData,
    frame: usize,
) -> Option<PlaybackSampleValue> {
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
        let data = SampleData::pcm16((0..len).map(|i| i as i16).collect());
        if looped {
            Sample {
                data,
                loop_kind: SampleLoopKind::Forward,
                loop_start: 2,
                loop_length: (len as u32).saturating_sub(2),
                ..Default::default()
            }
        } else {
            Sample {
                data,
                ..Default::default()
            }
        }
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

    #[test]
    fn tap_index_backward_loop_wraps_to_tail() {
        // len 8, loop_start 2, loop_length 6 => loop_end 8.
        let s = ramp_sample(8, true);
        // At the loop re-entry, the previous frame is the loop TAIL, not pre-loop.
        assert_eq!(tap_index(2, -1, &s), Some(7)); // loop_end - 1
        assert_eq!(tap_index(2, -2, &s), Some(6));
    }
}
