//! Phase scrambling in the Fourier domain.
//!
//! The same random phase field is added to every colour channel, which preserves the phase
//! differences between channels and therefore avoids spurious colours. The field is Hermitian
//! symmetric so the result stays real, and the DC term is never changed, so the mean of each
//! channel is preserved. With `PaddingMode::Wrap` the amplitude spectrum is preserved exactly.

use num_complex::Complex64;
use rand::Rng;
use rayon::prelude::*;

use super::types::{FourierOptions, FrequencyRange, PaddingMode};
use crate::planes::Planes;
use crate::seed;
use crate::spectral::{fft2, mirror_index, radial_frequency};

pub fn scramble(input: &Planes, options: &FourierOptions, rng: &mut seed::Rng) -> Planes {
    let source = if options.grayscale && !input.is_grayscale() {
        input.to_grayscale()
    } else {
        input.clone()
    };
    let (width, height) = (source.width, source.height);
    let (pw, ph) = match options.padding_mode {
        PaddingMode::Wrap => (width, height),
        PaddingMode::Reflect | PaddingMode::Mean => (2 * width, 2 * height),
    };

    let phase_noise = if options.phase_scramble {
        random_phase_field(pw, ph, rng)
    } else {
        vec![0.0; pw * ph]
    };
    let band = band_mask(options, width, pw, ph);

    let color = source
        .color
        .par_iter()
        .map(|plane| {
            let mut data = pad(plane, width, height, pw, ph, options.padding_mode);
            fft2(&mut data, pw, ph, false);
            for (i, v) in data.iter_mut().enumerate() {
                if i == 0 {
                    continue;
                }
                if band[i] {
                    *v *= Complex64::from_polar(1.0, options.intensity as f64 * phase_noise[i]);
                } else if options.remove_outside_range {
                    *v = Complex64::new(0.0, 0.0);
                }
            }
            fft2(&mut data, pw, ph, true);
            crop(&data, pw, width, height)
        })
        .collect();

    Planes { color, ..source }
}

/// Uniform random phases in (-pi, pi] with Hermitian symmetry; self-conjugate bins get 0.
/// Drawn for every bin regardless of the frequency range, so the same seed gives the same
/// noise whatever range is chosen.
fn random_phase_field(width: usize, height: usize, rng: &mut seed::Rng) -> Vec<f64> {
    let mut field = vec![0.0f64; width * height];
    for ky in 0..height {
        for kx in 0..width {
            let i = ky * width + kx;
            let m = mirror_index(kx, ky, width, height);
            if i < m {
                let theta = rng.random_range(-std::f64::consts::PI..std::f64::consts::PI);
                field[i] = theta;
                field[m] = -theta;
            }
        }
    }
    field
}

/// Which bins of a `pw` x `ph` transform fall inside the frequency range.
fn band_mask(options: &FourierOptions, image_width: usize, pw: usize, ph: usize) -> Vec<bool> {
    let cpp = |v: f32| options.unit.to_cycles_per_pixel(v, image_width);
    let range = match options.frequency_range {
        FrequencyRange::All => (0.0, f64::INFINITY),
        FrequencyRange::LowPass(c) => (0.0, cpp(c)),
        FrequencyRange::HighPass(c) => (cpp(c), f64::INFINITY),
        FrequencyRange::BandPass { low, high } => (cpp(low), cpp(high)),
    };
    (0..ph)
        .flat_map(|ky| (0..pw).map(move |kx| (kx, ky)))
        .map(|(kx, ky)| {
            let f = radial_frequency(kx, ky, pw, ph);
            f >= range.0 && f <= range.1
        })
        .collect()
}

fn pad(plane: &[f32], width: usize, height: usize, pw: usize, ph: usize, mode: PaddingMode) -> Vec<Complex64> {
    let mean = plane.iter().map(|&v| v as f64).sum::<f64>() / plane.len().max(1) as f64;
    let mut out = vec![Complex64::new(mean, 0.0); pw * ph];
    for y in 0..ph {
        for x in 0..pw {
            let value = match mode {
                PaddingMode::Wrap => Some(plane[y * width + x]),
                PaddingMode::Reflect => {
                    let sx = if x < width { x } else { 2 * width - 1 - x };
                    let sy = if y < height { y } else { 2 * height - 1 - y };
                    Some(plane[sy * width + sx])
                }
                PaddingMode::Mean => (x < width && y < height).then(|| plane[y * width + x]),
            };
            if let Some(v) = value {
                out[y * pw + x] = Complex64::new(v as f64, 0.0);
            }
        }
    }
    out
}

fn crop(data: &[Complex64], pw: usize, width: usize, height: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(width * height);
    for y in 0..height {
        out.extend(data[y * pw..y * pw + width].iter().map(|v| v.re as f32));
    }
    out
}
