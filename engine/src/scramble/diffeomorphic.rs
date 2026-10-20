// Diffeomorphic scrambling based on Stojanoski & Cusack (2014).
// Code adapted from the original MATLAB implementation https://github.com/rhodricusack/diffeomorph/, MIT License.

use rand::Rng;
use rayon::prelude::*;

use super::types::DiffeomorphicOptions;
use crate::planes::Planes;
use crate::seed;

pub fn scramble(input: &Planes, options: &DiffeomorphicOptions, rng: &mut seed::Rng) -> Planes {
    let (warp_x, warp_y) = make_warp_field(input.width, input.height, options, rng);
    apply(input, &warp_x, &warp_y)
}

/// The warp that `scramble` applies with the same `rng`: absolute source coordinates for every
/// output pixel.
pub fn warp_field(width: usize, height: usize, options: &DiffeomorphicOptions, rng: &mut seed::Rng) -> (Vec<f32>, Vec<f32>) {
    make_warp_field(width, height, options, rng)
}

/// Samples every plane of `input` at the absolute source coordinates `warp_x`, `warp_y`.
pub fn apply(input: &Planes, warp_x: &[f32], warp_y: &[f32]) -> Planes {
    let warp = |plane: &Vec<f32>| apply_warp(plane, input.width, input.height, warp_x, warp_y);
    Planes {
        color: input.color.par_iter().map(warp).collect(),
        alpha: input.alpha.as_ref().map(warp),
        ..input.clone()
    }
}

/// Generates a diffeomorphic warp from random DCT components, following `getdiffeo` in
/// Stojanoski & Cusack (2014). Returns absolute source coordinates for every output pixel.
fn make_warp_field(width: usize, height: usize, options: &DiffeomorphicOptions, rng: &mut seed::Rng) -> (Vec<f32>, Vec<f32>) {
    let n = options.n_comp as usize;
    let nsteps = options.n_steps.max(1) as usize;
    let two_pi = 2.0 * std::f32::consts::PI;

    // Four random phases and two amplitudes per component, drawn in the MATLAB order.
    let mut ph = vec![[0.0f32; 4]; n * n];
    let mut amp_x = vec![0.0f32; n * n];
    let mut amp_y = vec![0.0f32; n * n];
    for c in 0..n * n {
        for phase in ph[c].iter_mut() {
            *phase = rng.random::<f32>() * two_pi;
        }
        amp_x[c] = rng.random::<f32>() * two_pi;
        amp_y[c] = rng.random::<f32>() * two_pi;
    }

    // Each term a * cos(xc*X + p1) * cos(yc*Y + p2) is separable. Expanding the cosines lets the
    // sum be evaluated per row with tables of cos/sin over x, at O(n) work per pixel.
    let axis_tables = |len: usize| -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
        (0..n)
            .map(|i| {
                let freq = (i + 1) as f32;
                (0..len)
                    .map(|p| {
                        let a = freq * ((p + 1) as f32 / len as f32) * two_pi;
                        (a.cos(), a.sin())
                    })
                    .unzip()
            })
            .unzip()
    };
    let (cos_x, sin_x) = axis_tables(width);
    let (cos_y, sin_y) = axis_tables(height);

    let field = |amp: &[f32], px: usize, py: usize| -> Vec<f32> {
        let mut out = vec![0.0f32; width * height];
        out.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
            // For each x frequency i: row coefficients of cos(i X) and sin(i X).
            let mut cc = vec![0.0f32; n];
            let mut ss = vec![0.0f32; n];
            for i in 0..n {
                for j in 0..n {
                    let c = i * n + j;
                    let ycos = cos_y[j][y] * ph[c][py].cos() - sin_y[j][y] * ph[c][py].sin();
                    let term = amp[c] * ycos;
                    cc[i] += term * ph[c][px].cos();
                    ss[i] += term * ph[c][px].sin();
                }
            }
            for (x, v) in row.iter_mut().enumerate() {
                *v = (0..n).map(|i| cos_x[i][x] * cc[i] - sin_x[i][x] * ss[i]).sum();
            }
        });
        out
    };
    let mut field_x = field(&amp_x, 0, 1);
    let mut field_y = field(&amp_y, 2, 3);

    // Normalise each axis to unit RMS, then scale to max_distortion / n_steps per step.
    let npix = (width * height) as f32;
    for f in [&mut field_x, &mut field_y] {
        let rms = (f.iter().map(|v| v * v).sum::<f32>() / npix).sqrt();
        if rms > 1e-8 {
            let scale = options.max_distortion / (nsteps as f32 * rms);
            f.iter_mut().for_each(|v| *v *= scale);
        }
    }

    // Compose n_steps small warps: new_warp(p) = old_warp(p + field(p)).
    let mut warp_x: Vec<f32> = (0..width * height).map(|i| (i % width) as f32).collect();
    let mut warp_y: Vec<f32> = (0..width * height).map(|i| (i / width) as f32).collect();
    for _ in 0..nsteps {
        let next: Vec<(f32, f32)> = (0..width * height)
            .into_par_iter()
            .map(|i| {
                let sx = (i % width) as f32 + field_x[i];
                let sy = (i / width) as f32 + field_y[i];
                (bilinear(&warp_x, width, height, sx, sy), bilinear(&warp_y, width, height, sx, sy))
            })
            .collect();
        (warp_x, warp_y) = next.into_iter().unzip();
    }
    (warp_x, warp_y)
}

fn apply_warp(plane: &[f32], width: usize, height: usize, warp_x: &[f32], warp_y: &[f32]) -> Vec<f32> {
    (0..width * height)
        .into_par_iter()
        .map(|i| bilinear(plane, width, height, warp_x[i], warp_y[i]))
        .collect()
}

/// Bilinear sample with coordinates clamped to the image.
fn bilinear(values: &[f32], width: usize, height: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (width - 1) as f32);
    let y = y.clamp(0.0, (height - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(width - 1), (y0 + 1).min(height - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    values[y0 * width + x0] * (1.0 - fx) * (1.0 - fy)
        + values[y0 * width + x1] * fx * (1.0 - fy)
        + values[y1 * width + x0] * (1.0 - fx) * fy
        + values[y1 * width + x1] * fx * fy
}
