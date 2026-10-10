use num_complex::Complex64;
use rayon::prelude::*;
use rustfft::FftPlanner;

/// In-place 2D FFT of a row-major `width` x `height` array.
/// The inverse transform is normalised by 1 / (width * height).
pub fn fft2(data: &mut [Complex64], width: usize, height: usize, inverse: bool) {
    let mut planner = FftPlanner::new();
    let (row_fft, col_fft) = if inverse {
        (planner.plan_fft_inverse(width), planner.plan_fft_inverse(height))
    } else {
        (planner.plan_fft_forward(width), planner.plan_fft_forward(height))
    };

    data.par_chunks_mut(width).for_each(|row| row_fft.process(row));

    let mut transposed = transpose(data, width, height);
    transposed.par_chunks_mut(height).for_each(|col| col_fft.process(col));
    data.copy_from_slice(&transpose(&transposed, height, width));

    if inverse {
        let scale = 1.0 / (width * height) as f64;
        data.par_iter_mut().for_each(|v| *v *= scale);
    }
}

fn transpose(data: &[Complex64], width: usize, height: usize) -> Vec<Complex64> {
    let mut out = vec![Complex64::new(0.0, 0.0); data.len()];
    out.par_chunks_mut(height).enumerate().for_each(|(x, col)| {
        for (y, v) in col.iter_mut().enumerate() {
            *v = data[y * width + x];
        }
    });
    out
}

/// Signed frequency of FFT bin `k` for a transform of length `n`, in cycles per sample.
pub fn bin_frequency(k: usize, n: usize) -> f64 {
    let signed = if k <= n / 2 { k as f64 } else { k as f64 - n as f64 };
    signed / n as f64
}

/// Radial spatial frequency of bin (kx, ky) in cycles per pixel.
pub fn radial_frequency(kx: usize, ky: usize, width: usize, height: usize) -> f64 {
    bin_frequency(kx, width).hypot(bin_frequency(ky, height))
}

/// Index of the bin holding the complex conjugate of bin (kx, ky) for a real signal.
pub fn mirror_index(kx: usize, ky: usize, width: usize, height: usize) -> usize {
    ((height - ky) % height) * width + (width - kx) % width
}

pub fn to_complex(values: &[f32]) -> Vec<Complex64> {
    values.iter().map(|&v| Complex64::new(v as f64, 0.0)).collect()
}

/// Rotationally averaged amplitude spectrum.
pub struct RadialSpectrum {
    /// Bin centres in cycles per pixel.
    pub frequencies: Vec<f64>,
    /// Mean amplitude per bin, normalised by the number of pixels.
    pub amplitudes: Vec<f64>,
}

/// Rotationally averaged amplitude spectrum of a plane, excluding the DC term.
/// Bins are 1 / min(width, height) cycles per pixel wide and stop at Nyquist (0.5).
pub fn radial_spectrum(values: &[f32], width: usize, height: usize) -> RadialSpectrum {
    let mut data = to_complex(values);
    fft2(&mut data, width, height, false);

    let bins = (width.min(height) / 2).max(1);
    let bin_width = 0.5 / bins as f64;
    let mut sums = vec![0.0f64; bins];
    let mut counts = vec![0usize; bins];
    let norm = (width * height) as f64;

    for ky in 0..height {
        for kx in 0..width {
            if kx == 0 && ky == 0 {
                continue;
            }
            let f = radial_frequency(kx, ky, width, height);
            let bin = (f / bin_width).floor() as usize;
            if bin < bins {
                sums[bin] += data[ky * width + kx].norm() / norm;
                counts[bin] += 1;
            }
        }
    }

    let mut frequencies = Vec::with_capacity(bins);
    let mut amplitudes = Vec::with_capacity(bins);
    for b in 0..bins {
        if counts[b] > 0 {
            frequencies.push((b as f64 + 0.5) * bin_width);
            amplitudes.push(sums[b] / counts[b] as f64);
        }
    }
    RadialSpectrum { frequencies, amplitudes }
}

/// Slope of a least-squares line through log10(amplitude) against log10(frequency).
/// Natural images typically fall near -1 to -1.2.
pub fn spectral_slope(spectrum: &RadialSpectrum) -> f64 {
    let points: Vec<(f64, f64)> = spectrum
        .frequencies
        .iter()
        .zip(&spectrum.amplitudes)
        .filter(|(_, a)| **a > 0.0)
        .map(|(f, a)| (f.log10(), a.log10()))
        .collect();
    if points.len() < 2 {
        return f64::NAN;
    }
    let n = points.len() as f64;
    let mx = points.iter().map(|p| p.0).sum::<f64>() / n;
    let my = points.iter().map(|p| p.1).sum::<f64>() / n;
    let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
    sxy / sxx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_round_trip_non_square() {
        let (w, h) = (7, 5);
        let values: Vec<f32> = (0..w * h).map(|i| (i as f32 * 0.37).sin()).collect();
        let mut data = to_complex(&values);
        fft2(&mut data, w, h, false);
        fft2(&mut data, w, h, true);
        for (a, b) in values.iter().zip(&data) {
            assert!((*a as f64 - b.re).abs() < 1e-9);
            assert!(b.im.abs() < 1e-9);
        }
    }
}
