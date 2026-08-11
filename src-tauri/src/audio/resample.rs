//! Small self-contained audio conversion helpers.
//!
//! Linear-interpolation resampling is used instead of a polyphase resampler:
//! for 16 kHz VAD input and 24 kHz speech-model input the quality loss is
//! negligible, it handles any ratio (48 kHz and 44.1 kHz devices alike), it
//! carries state across arbitrary-sized chunks, and it is trivially testable.

pub struct LinearResampler {
    /// input samples advanced per output sample
    step: f64,
    /// position of the next output sample, relative to the current chunk;
    /// may be in [-1, 0) which addresses `prev`
    pos: f64,
    prev: f32,
    primed: bool,
}

impl LinearResampler {
    pub fn new(from_rate: u32, to_rate: u32) -> Self {
        assert!(from_rate > 0 && to_rate > 0);
        Self {
            step: from_rate as f64 / to_rate as f64,
            pos: 0.0,
            prev: 0.0,
            primed: false,
        }
    }

    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        if input.is_empty() {
            return Vec::new();
        }
        let n = input.len();
        let mut out = Vec::with_capacity((n as f64 / self.step) as usize + 2);

        if !self.primed {
            self.primed = true;
            self.prev = input[0];
            self.pos = 0.0;
        }

        loop {
            let p = self.pos;
            if p < 0.0 {
                let frac = (p + 1.0) as f32;
                out.push(self.prev + (input[0] - self.prev) * frac);
            } else {
                let i = p as usize;
                if i + 1 < n {
                    let frac = (p - i as f64) as f32;
                    out.push(input[i] + (input[i + 1] - input[i]) * frac);
                } else if i + 1 == n && (p - i as f64) < 1e-9 {
                    out.push(input[i]);
                } else {
                    break;
                }
            }
            self.pos += self.step;
        }

        self.prev = input[n - 1];
        self.pos -= n as f64;
        out
    }
}

/// Interleaved multi-channel f32 → mono (channel average).
pub fn downmix(input: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    if ch == 1 {
        return input.to_vec();
    }
    input
        .chunks_exact(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}

/// f32 [-1, 1] → i16 with clamping.
pub fn to_i16(input: &[f32]) -> Vec<i16> {
    input
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect()
}

/// Root-mean-square level of a chunk (for UI meters).
pub fn rms(input: &[f32]) -> f32 {
    if input.is_empty() {
        return 0.0;
    }
    (input.iter().map(|s| s * s).sum::<f32>() / input.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, freq: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    #[test]
    fn halves_sample_count_for_48k_to_24k() {
        let mut r = LinearResampler::new(48_000, 24_000);
        let out = r.process(&sine(48_000, 440.0, 4800));
        assert!((out.len() as i64 - 2400).abs() <= 2, "got {}", out.len());
    }

    #[test]
    fn handles_non_integer_ratio_44100_to_24000() {
        let mut r = LinearResampler::new(44_100, 24_000);
        let mut total = 0usize;
        // feed in uneven chunks to exercise carry-over state
        let input = sine(44_100, 440.0, 44_100);
        for chunk in input.chunks(997) {
            total += r.process(chunk).len();
        }
        assert!((total as i64 - 24_000).abs() <= 3, "got {total}");
    }

    #[test]
    fn preserves_a_low_frequency_tone() {
        // 100 Hz sine at 48k downsampled to 16k should still swing ±1.
        let mut r = LinearResampler::new(48_000, 16_000);
        let out = r.process(&sine(48_000, 100.0, 48_000));
        let max = out.iter().cloned().fold(0.0f32, f32::max);
        let min = out.iter().cloned().fold(0.0f32, f32::min);
        assert!(max > 0.95 && min < -0.95, "max={max} min={min}");
    }

    #[test]
    fn chunked_processing_matches_single_shot() {
        let input = sine(48_000, 300.0, 9600);
        let mut a = LinearResampler::new(48_000, 24_000);
        let single = a.process(&input);

        let mut b = LinearResampler::new(48_000, 24_000);
        let mut chunked = Vec::new();
        for c in input.chunks(731) {
            chunked.extend(b.process(c));
        }
        assert_eq!(single.len(), chunked.len());
        for (x, y) in single.iter().zip(chunked.iter()) {
            assert!((x - y).abs() < 1e-6);
        }
    }

    #[test]
    fn downmix_averages_channels() {
        let stereo = [1.0, 0.0, 0.5, 0.5, -1.0, 1.0];
        assert_eq!(downmix(&stereo, 2), vec![0.5, 0.5, 0.0]);
    }

    #[test]
    fn to_i16_clamps_out_of_range() {
        let out = to_i16(&[2.0, -2.0, 0.0, 1.0]);
        assert_eq!(out[0], 32767);
        assert_eq!(out[1], -32767);
        assert_eq!(out[2], 0);
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&[0.0; 480]), 0.0);
        assert!(rms(&[0.5; 480]) > 0.49);
    }
}
