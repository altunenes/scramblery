//! Low-level image statistics for checking what a manipulation preserved.
//!
//! Luminance is Rec. 709 luma computed from the encoded channel values in the range 0..1,
//! which is the quantity most stimulus-control toolboxes (e.g. SHINE) operate on.

use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::planes::Planes;
use crate::scramble::blur::gaussian_blur;
use crate::spectral::{radial_spectrum, spectral_slope};

pub const HISTOGRAM_BINS: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageStats {
    pub width: usize,
    pub height: usize,
    /// Mean luminance (0..1).
    pub mean_luminance: f64,
    /// RMS contrast: standard deviation of luminance (0..1).
    pub rms_contrast: f64,
    /// Mean of each colour channel (one entry for grayscale images).
    pub channel_means: Vec<f64>,
    /// Fraction of luminance values at 0 or 1, i.e. at the ends of the range.
    pub saturated_fraction: f64,
    /// Luminance histogram with `HISTOGRAM_BINS` equal bins over 0..1.
    pub histogram: Vec<u32>,
    /// Centres of the radial spectrum bins in cycles per pixel.
    pub spectrum_frequencies: Vec<f64>,
    /// Rotationally averaged amplitude of the luminance spectrum.
    pub spectrum_amplitudes: Vec<f64>,
    /// Slope of log amplitude against log frequency.
    pub spectral_slope: f64,
}

pub fn analyze(image: &DynamicImage) -> ImageStats {
    analyze_planes(&Planes::from_image(image))
}

pub fn analyze_planes(planes: &Planes) -> ImageStats {
    let lum = planes.luminance();
    let (mean, sd) = mean_sd(&lum);
    let spectrum = radial_spectrum(&lum, planes.width, planes.height);

    let mut histogram = vec![0u32; HISTOGRAM_BINS];
    let mut saturated = 0usize;
    for &v in &lum {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.0 || v >= 1.0 {
            saturated += 1;
        }
        let bin = ((v * HISTOGRAM_BINS as f32) as usize).min(HISTOGRAM_BINS - 1);
        histogram[bin] += 1;
    }

    ImageStats {
        width: planes.width,
        height: planes.height,
        mean_luminance: mean,
        rms_contrast: sd,
        channel_means: planes.color.iter().map(|p| mean_sd(p).0).collect(),
        saturated_fraction: saturated as f64 / lum.len().max(1) as f64,
        histogram,
        spectral_slope: spectral_slope(&spectrum),
        spectrum_frequencies: spectrum.frequencies,
        spectrum_amplitudes: spectrum.amplitudes,
    }
}

pub fn mean_sd(values: &[f32]) -> (f64, f64) {
    let n = values.len().max(1) as f64;
    let mean = values.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = values.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
    (mean, var.sqrt())
}

/// Structural similarity of the luminance of two equally sized images
/// (Wang et al., 2004; 11x11 Gaussian window with sigma 1.5).
/// Returns `None` when the sizes differ.
pub fn ssim(a: &DynamicImage, b: &DynamicImage) -> Option<f64> {
    let (pa, pb) = (Planes::from_image(a), Planes::from_image(b));
    ssim_planes(&pa, &pb)
}

pub fn ssim_planes(a: &Planes, b: &Planes) -> Option<f64> {
    if a.width != b.width || a.height != b.height || a.is_empty() {
        return None;
    }
    let (w, h) = (a.width, a.height);
    let (x, y) = (a.luminance(), b.luminance());
    let blur = |v: &[f32]| gaussian_blur(v, w, h, 1.5);
    let product = |p: &[f32], q: &[f32]| p.iter().zip(q).map(|(a, b)| a * b).collect::<Vec<f32>>();

    let (mx, my) = (blur(&x), blur(&y));
    let (sxx, syy, sxy) = (blur(&product(&x, &x)), blur(&product(&y, &y)), blur(&product(&x, &y)));
    let (c1, c2) = (0.01f64.powi(2), 0.03f64.powi(2));

    let total: f64 = (0..w * h)
        .map(|i| {
            let (mx, my) = (mx[i] as f64, my[i] as f64);
            let vx = sxx[i] as f64 - mx * mx;
            let vy = syy[i] as f64 - my * my;
            let cov = sxy[i] as f64 - mx * my;
            ((2.0 * mx * my + c1) * (2.0 * cov + c2)) / ((mx * mx + my * my + c1) * (vx + vy + c2))
        })
        .sum();
    Some(total / (w * h) as f64)
}
