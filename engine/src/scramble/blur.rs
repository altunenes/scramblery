use rayon::prelude::*;

use crate::planes::Planes;

/// Gaussian blur with standard deviation `sigma` pixels, applied to colour and alpha.
pub fn scramble(input: &Planes, sigma: f32) -> Planes {
    let blur = |plane: &Vec<f32>| gaussian_blur(plane, input.width, input.height, sigma);
    Planes {
        color: input.color.par_iter().map(blur).collect(),
        alpha: input.alpha.as_ref().map(blur),
        ..input.clone()
    }
}

/// Separable Gaussian filter with mirrored borders. The kernel extends to 3 sigma.
pub fn gaussian_blur(plane: &[f32], width: usize, height: usize, sigma: f32) -> Vec<f32> {
    if sigma <= 0.0 || plane.is_empty() {
        return plane.to_vec();
    }
    let radius = (3.0 * sigma).ceil() as isize;
    let mut kernel: Vec<f32> = (-radius..=radius)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    kernel.iter_mut().for_each(|k| *k /= sum);

    let mirror = |i: isize, n: usize| -> usize {
        let n = n as isize;
        if n == 1 {
            return 0;
        }
        let period = 2 * n - 2;
        let m = i.rem_euclid(period);
        (if m < n { m } else { period - m }) as usize
    };

    let mut horizontal = vec![0.0f32; plane.len()];
    horizontal.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        let src = &plane[y * width..(y + 1) * width];
        for (x, out) in row.iter_mut().enumerate() {
            *out = kernel
                .iter()
                .enumerate()
                .map(|(k, w)| w * src[mirror(x as isize + k as isize - radius, width)])
                .sum();
        }
    });

    let mut output = vec![0.0f32; plane.len()];
    output.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        for (x, out) in row.iter_mut().enumerate() {
            *out = kernel
                .iter()
                .enumerate()
                .map(|(k, w)| w * horizontal[mirror(y as isize + k as isize - radius, height) * width + x])
                .sum();
        }
    });
    output
}
