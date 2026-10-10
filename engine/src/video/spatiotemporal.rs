//! Spatiotemporal phase scrambling: a block of frames is treated as one volume per colour channel
//! (x, y, time). The 3D Fourier transform of each volume receives the same random phase field,
//! which is Hermitian symmetric so the result stays real and is zero at the DC term. The 3D
//! amplitude spectrum of every channel, and hence both its spatial and temporal frequency
//! content, is preserved, as are the channel means.

use num_complex::Complex32;
use rand::Rng;
use rayon::prelude::*;
use rustfft::FftPlanner;
use serde::{Deserialize, Serialize};

use crate::planes::Planes;
use crate::seed;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct SpatiotemporalOptions {
    /// Scale of the random phase (0..1). At 1 the phase is uniformly random.
    pub intensity: f32,
}

/// In-place 3D FFT of a volume stored frame by frame, row-major within frames.
/// The inverse is normalised by 1 / (w * h * t).
pub fn fft3(data: &mut [Complex32], w: usize, h: usize, t: usize, inverse: bool) {
    let mut planner = FftPlanner::<f32>::new();
    let mut plan = |n| if inverse { planner.plan_fft_inverse(n) } else { planner.plan_fft_forward(n) };
    let (fx, fy, ft) = (plan(w), plan(h), plan(t));
    let frame = w * h;

    data.par_chunks_mut(w).for_each(|row| fx.process(row));

    data.par_chunks_mut(frame).for_each(|plane| {
        let mut column = vec![Complex32::new(0.0, 0.0); h];
        for x in 0..w {
            for y in 0..h {
                column[y] = plane[y * w + x];
            }
            fy.process(&mut column);
            for y in 0..h {
                plane[y * w + x] = column[y];
            }
        }
    });

    if t > 1 {
        let lines: Vec<Vec<Complex32>> = (0..frame)
            .into_par_iter()
            .map(|p| {
                let mut line: Vec<Complex32> = (0..t).map(|k| data[k * frame + p]).collect();
                ft.process(&mut line);
                line
            })
            .collect();
        for (p, line) in lines.into_iter().enumerate() {
            for (k, v) in line.into_iter().enumerate() {
                data[k * frame + p] = v;
            }
        }
    }

    if inverse {
        let scale = 1.0 / (frame * t) as f32;
        data.par_iter_mut().for_each(|v| *v *= scale);
    }
}

/// Uniform random phases with 3D Hermitian symmetry; self-conjugate bins (including DC) get 0.
fn phase_field(w: usize, h: usize, t: usize, rng: &mut seed::Rng) -> Vec<f32> {
    let mut field = vec![0.0f32; w * h * t];
    for k in 0..t {
        for y in 0..h {
            for x in 0..w {
                let i = (k * h + y) * w + x;
                let m = (((t - k) % t) * h + (h - y) % h) * w + (w - x) % w;
                if i < m {
                    let theta = rng.random_range(-std::f32::consts::PI..std::f32::consts::PI);
                    field[i] = theta;
                    field[m] = -theta;
                }
            }
        }
    }
    field
}

/// Scrambles a block of equally sized frames as one spatiotemporal volume per channel.
/// Values are not clipped.
pub fn scramble(frames: &[Planes], options: &SpatiotemporalOptions, seed: u64) -> Vec<Planes> {
    let Some(first) = frames.first() else { return Vec::new() };
    let (w, h, t) = (first.width, first.height, frames.len());
    let frame = w * h;
    let field = phase_field(w, h, t, &mut seed::rng(seed));

    let mut output: Vec<Planes> = frames.to_vec();
    for c in 0..first.color.len() {
        let mut volume = vec![Complex32::new(0.0, 0.0); frame * t];
        volume.par_chunks_mut(frame).zip(frames.par_iter()).for_each(|(dst, src)| {
            for (d, &v) in dst.iter_mut().zip(&src.color[c]) {
                *d = Complex32::new(v, 0.0);
            }
        });
        fft3(&mut volume, w, h, t, false);
        volume.par_iter_mut().zip(&field).for_each(|(v, &theta)| *v *= Complex32::from_polar(1.0, options.intensity * theta));
        fft3(&mut volume, w, h, t, true);
        output.par_iter_mut().zip(volume.par_chunks(frame)).for_each(|(dst, src)| {
            for (d, v) in dst.color[c].iter_mut().zip(src) {
                *d = v.re;
            }
        });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(w: usize, h: usize, t: usize) -> Vec<Planes> {
        (0..t)
            .map(|k| {
                let plane: Vec<f32> = (0..w * h)
                    .map(|i| 0.5 + 0.2 * ((i % w) as f32 * 0.7 + k as f32 * 0.9).sin() * ((i / w) as f32 * 0.4).cos())
                    .collect();
                Planes { width: w, height: h, color: vec![plane.clone(), plane.iter().map(|v| v * 0.8).collect(), plane], alpha: None, sixteen_bit: false }
            })
            .collect()
    }

    fn spectrum(frames: &[Planes], c: usize) -> Vec<Complex32> {
        let (w, h, t) = (frames[0].width, frames[0].height, frames.len());
        let mut data: Vec<Complex32> = frames.iter().flat_map(|p| p.color[c].iter().map(|&v| Complex32::new(v, 0.0))).collect();
        fft3(&mut data, w, h, t, false);
        data
    }

    #[test]
    fn fft3_round_trip() {
        let frames = volume(9, 7, 5);
        let (w, h, t) = (9, 7, 5);
        let original: Vec<Complex32> = frames.iter().flat_map(|p| p.color[0].iter().map(|&v| Complex32::new(v, 0.0))).collect();
        let mut data = original.clone();
        fft3(&mut data, w, h, t, false);
        fft3(&mut data, w, h, t, true);
        assert!(original.iter().zip(&data).all(|(a, b)| (a - b).norm() < 1e-4));
    }

    #[test]
    fn preserves_the_spatiotemporal_amplitude_spectrum_and_means() {
        let frames = volume(16, 12, 10);
        let out = scramble(&frames, &SpatiotemporalOptions { intensity: 1.0 }, 3);
        for c in 0..3 {
            let (a, b) = (spectrum(&frames, c), spectrum(&out, c));
            assert!(a.iter().zip(&b).all(|(x, y)| (x.norm() - y.norm()).abs() < 1e-2));
            assert!((a[0] - b[0]).norm() < 1e-2);
        }
        let changed = frames[4].color[0].iter().zip(&out[4].color[0]).filter(|(x, y)| (*x - *y).abs() > 1e-3).count();
        assert!(changed > 100);
    }

    #[test]
    fn zero_intensity_is_identity_and_seeds_reproduce() {
        let frames = volume(8, 6, 4);
        let same = scramble(&frames, &SpatiotemporalOptions { intensity: 0.0 }, 1);
        assert!(frames.iter().zip(&same).all(|(a, b)| a.color[0].iter().zip(&b.color[0]).all(|(x, y)| (x - y).abs() < 1e-5)));
        let opts = SpatiotemporalOptions { intensity: 1.0 };
        assert_eq!(scramble(&frames, &opts, 9), scramble(&frames, &opts, 9));
    }
}
