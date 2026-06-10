//! Minimal windowed-sinc resampler for converting microphone audio to the
//! 16 kHz mono stream whisper.cpp expects.
//!
//! Hand-rolled on purpose: the only job is speech-quality downsampling from
//! 44.1/48 kHz, which a Blackman-windowed sinc kernel handles well. If higher
//! fidelity is ever needed, swap this module for the `rubato` crate.

use std::f32::consts::PI;

pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// Half-width of the sinc kernel in input samples (at the lower of the two
/// rates). 24 taps per side keeps aliasing well below speech-relevant levels.
const HALF_TAPS: usize = 24;

/// Resamples mono `f32` samples from `from_rate` to 16 kHz.
pub fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    resample(samples, from_rate, WHISPER_SAMPLE_RATE)
}

pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    assert!(
        from_rate > 0 && to_rate > 0,
        "sample rates must be non-zero"
    );
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    // When downsampling, the kernel must be widened by 1/ratio (and scaled by
    // ratio) so it low-passes at the *output* Nyquist frequency.
    let cutoff = ratio.min(1.0) as f32;
    let kernel_half_width = (HALF_TAPS as f32 / cutoff).ceil() as isize;

    let out_len = (samples.len() as f64 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        // Position of this output sample on the input timeline.
        let t = i as f64 / ratio;
        let center = t.floor() as isize;
        let mut acc = 0.0f32;
        for k in (center - kernel_half_width)..=(center + kernel_half_width) {
            if k < 0 || k as usize >= samples.len() {
                continue;
            }
            let x = (t - k as f64) as f32;
            acc += samples[k as usize] * windowed_sinc(x, cutoff, kernel_half_width as f32);
        }
        out.push(acc);
    }
    out
}

fn windowed_sinc(x: f32, cutoff: f32, half_width: f32) -> f32 {
    if x.abs() >= half_width {
        return 0.0;
    }
    // Blackman window over [-half_width, half_width].
    let w = 0.42 + 0.5 * (PI * x / half_width).cos() + 0.08 * (2.0 * PI * x / half_width).cos();
    cutoff * sinc(cutoff * x) * w
}

fn sinc(x: f32) -> f32 {
    if x.abs() < 1e-6 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

/// Root mean square of a sample buffer; used as a cheap level meter and
/// silence gate.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Interleaved multi-channel → mono by averaging channels.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    let channels = channels as usize;
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, secs: f32) -> Vec<f32> {
        let n = (rate as f32 * secs) as usize;
        (0..n)
            .map(|i| (2.0 * PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    fn zero_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count()
    }

    #[test]
    fn passthrough_at_equal_rates() {
        let input = sine(440.0, 16_000, 0.1);
        assert_eq!(resample(&input, 16_000, 16_000), input);
    }

    #[test]
    fn output_length_matches_ratio() {
        let input = vec![0.0f32; 48_000];
        let out = resample_to_16k(&input, 48_000);
        assert_eq!(out.len(), 16_000);

        let input = vec![0.0f32; 44_100];
        let out = resample_to_16k(&input, 44_100);
        assert_eq!(out.len(), 16_000);
    }

    #[test]
    fn preserves_tone_frequency_when_downsampling() {
        for rate in [48_000u32, 44_100] {
            let input = sine(440.0, rate, 1.0);
            let out = resample_to_16k(&input, rate);
            let crossings = zero_crossings(&out);
            // One positive-going crossing per cycle; 440 Hz over 1 s.
            assert!(
                (430..=450).contains(&crossings),
                "expected ~440 crossings at {rate} Hz, got {crossings}"
            );
        }
    }

    #[test]
    fn attenuates_above_nyquist_content() {
        // 7.5 kHz survives (under 8 kHz output Nyquist); energy preserved.
        let pass = resample_to_16k(&sine(7_500.0, 48_000, 0.5), 48_000);
        // 11 kHz would alias; the kernel must suppress it.
        let stop = resample_to_16k(&sine(11_000.0, 48_000, 0.5), 48_000);
        assert!(rms(&pass) > 0.5, "passband rms too low: {}", rms(&pass));
        assert!(rms(&stop) < 0.05, "stopband rms too high: {}", rms(&stop));
    }

    #[test]
    fn downmix_averages_channels() {
        let stereo = vec![1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5, 0.0]);
        let mono = vec![0.1, 0.2];
        assert_eq!(downmix_to_mono(&mono, 1), mono);
    }

    #[test]
    fn rms_basics() {
        assert_eq!(rms(&[]), 0.0);
        assert!((rms(&[1.0, -1.0]) - 1.0).abs() < 1e-6);
        let quiet = vec![0.001f32; 1000];
        assert!(rms(&quiet) < 0.002);
    }
}
