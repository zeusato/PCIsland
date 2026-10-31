//! Audio spectrum → 16 smoothed log-spaced bands (docs/modules/lyrics-visualizer.md §B5). Pure.

use std::collections::VecDeque;
use std::sync::Arc;

use realfft::{RealFftPlanner, RealToComplex};

pub const BANDS: usize = 16;
const N: usize = 1024;
const F_MIN: f32 = 60.0;
const F_MAX: f32 = 12_000.0;
const FLOOR_DB: f32 = -60.0;
const ATTACK: f32 = 0.6;
const DECAY: f32 = 0.15;

pub struct Analyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    samples: VecDeque<f32>,
    rate: u32,
    levels: [f32; BANDS],
}

impl Analyzer {
    pub fn new() -> Self {
        let window = (0..N)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos())
            .collect();
        Self {
            fft: RealFftPlanner::<f32>::new().plan_fft_forward(N),
            window,
            samples: VecDeque::with_capacity(N * 2),
            rate: 48_000,
            levels: [0.0; BANDS],
        }
    }

    pub fn push(&mut self, mono: &[f32], rate: u32) {
        self.rate = rate.max(8_000);
        self.samples.extend(mono);
        while self.samples.len() > N {
            self.samples.pop_front();
        }
    }

    /// One frame of smoothed band levels, quantized to 0–255.
    pub fn frame(&mut self) -> [u8; BANDS] {
        let mut input: Vec<f32> = self.samples.iter().copied().collect();
        input.resize(N, 0.0);
        for (x, w) in input.iter_mut().zip(&self.window) {
            *x *= w;
        }
        let mut spectrum = self.fft.make_output_vec();
        if self.fft.process(&mut input, &mut spectrum).is_err() {
            return [0; BANDS];
        }
        let bin_hz = self.rate as f32 / N as f32;
        let mut out = [0u8; BANDS];
        for (b, slot) in out.iter_mut().enumerate() {
            let (lo, hi) = band_range(b);
            let (i0, i1) = (
                ((lo / bin_hz) as usize).max(1),
                ((hi / bin_hz) as usize).max(2).min(spectrum.len()),
            );
            let peak = spectrum[i0.min(i1 - 1)..i1]
                .iter()
                .map(|c| c.norm())
                .fold(0.0f32, f32::max);
            let db = 20.0 * (peak / (N as f32 / 4.0)).max(1e-9).log10();
            let target = ((db - FLOOR_DB) / -FLOOR_DB).clamp(0.0, 1.0);
            let k = if target > self.levels[b] {
                ATTACK
            } else {
                DECAY
            };
            self.levels[b] += (target - self.levels[b]) * k;
            *slot = (self.levels[b] * 255.0) as u8;
        }
        out
    }
}

/// Frequency range of band `b` on a log scale between F_MIN and F_MAX.
pub fn band_range(b: usize) -> (f32, f32) {
    let ratio = (F_MAX / F_MIN).powf(1.0 / BANDS as f32);
    (
        F_MIN * ratio.powi(b as i32),
        F_MIN * ratio.powi(b as i32 + 1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, rate: u32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * 0.8)
            .collect()
    }

    #[test]
    fn bands_are_contiguous_and_cover_range() {
        assert!((band_range(0).0 - F_MIN).abs() < 0.01);
        assert!((band_range(BANDS - 1).1 - F_MAX).abs() < 1.0);
        for b in 1..BANDS {
            assert!((band_range(b).0 - band_range(b - 1).1).abs() < 0.01);
        }
    }

    #[test]
    fn a_tone_lights_its_band_most() {
        let mut a = Analyzer::new();
        let (lo, hi) = band_range(8);
        let freq = (lo * hi).sqrt();
        let mut f = [0u8; BANDS];
        for _ in 0..10 {
            a.push(&sine(freq, 48_000, N), 48_000);
            f = a.frame();
        }
        let loudest = f
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| **v)
            .map(|(i, _)| i);
        assert_eq!(loudest, Some(8));
    }

    #[test]
    fn silence_decays_to_zero() {
        let mut a = Analyzer::new();
        a.push(&sine(1000.0, 48_000, N), 48_000);
        a.frame();
        for _ in 0..60 {
            a.push(&[0.0; N], 48_000);
            a.frame();
        }
        assert!(a.frame().iter().all(|v| *v < 5));
    }
}
