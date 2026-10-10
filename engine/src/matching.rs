//! Matching low-level properties of an image to a target: the luminance mean and RMS contrast,
//! the histogram, or the amplitude spectrum. Targets come from a single image (e.g. the
//! original of a scrambled image), from fixed values, or from the average over a set of images.

use num_complex::Complex64;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::analysis::mean_sd;
use crate::planes::Planes;
use crate::spectral::{fft2, to_complex};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MatchProperty {
    /// Mean luminance and RMS contrast; all colour channels are shifted and scaled together.
    Luminance,
    /// The histogram of each channel (exact histogram specification).
    Histogram,
    /// The amplitude spectrum of each channel; the phase is kept.
    Spectrum,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MatchTarget {
    /// The image that was scrambled.
    Original,
    /// Fixed luminance statistics (luminance matching only).
    Fixed { mean: f64, rms_contrast: f64 },
    /// The average over a reference set of images.
    SetAverage,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchOptions {
    pub property: MatchProperty,
    pub target: MatchTarget,
}

/// A computed target for one property.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Luminance { mean: f64, rms_contrast: f64 },
    /// Per channel: values at evenly spaced quantiles, ascending.
    Histogram(Vec<Vec<f64>>),
    /// Per channel: amplitude of every DFT bin of a `width` x `height` image.
    Spectrum { width: usize, height: usize, amplitudes: Vec<Vec<f64>> },
}

impl Target {
    pub fn property(&self) -> MatchProperty {
        match self {
            Target::Luminance { .. } => MatchProperty::Luminance,
            Target::Histogram(_) => MatchProperty::Histogram,
            Target::Spectrum { .. } => MatchProperty::Spectrum,
        }
    }

    /// The target given by a single image.
    pub fn of_image(planes: &Planes, property: MatchProperty) -> Self {
        let mut acc = SetAccumulator::new(property, planes.len());
        acc.add(planes).expect("a single image always forms a valid set");
        acc.finish().expect("a single image always forms a valid set")
    }
}

/// Builds the average target of a set one image at a time, so the whole set never has to be
/// held in memory.
pub struct SetAccumulator {
    property: MatchProperty,
    resolution: usize,
    count: usize,
    channels: Option<usize>,
    size: Option<(usize, usize)>,
    luminance: (f64, f64),
    sums: Vec<Vec<f64>>,
}

impl SetAccumulator {
    /// `resolution` is the number of quantiles kept for histograms; use the largest pixel count
    /// in the set for exact histogram specification.
    pub fn new(property: MatchProperty, resolution: usize) -> Self {
        Self {
            property,
            resolution: resolution.max(1),
            count: 0,
            channels: None,
            size: None,
            luminance: (0.0, 0.0),
            sums: Vec::new(),
        }
    }

    pub fn add(&mut self, planes: &Planes) -> crate::Result<()> {
        let channels = planes.color.len();
        match self.channels {
            Some(c) if c != channels => anyhow::bail!("All images in a set must be either colour or grayscale"),
            _ => self.channels = Some(channels),
        }
        match self.property {
            MatchProperty::Luminance => {
                let (mean, sd) = mean_sd(&planes.luminance());
                self.luminance.0 += mean;
                self.luminance.1 += sd;
            }
            MatchProperty::Histogram => {
                let quantiles: Vec<Vec<f64>> = planes.color.par_iter().map(|p| resample_sorted(p, self.resolution)).collect();
                self.accumulate(quantiles);
            }
            MatchProperty::Spectrum => {
                let size = (planes.width, planes.height);
                match self.size {
                    Some(s) if s != size => anyhow::bail!("Spectrum matching needs all images to have the same size"),
                    _ => self.size = Some(size),
                }
                let amplitudes: Vec<Vec<f64>> = planes
                    .color
                    .par_iter()
                    .map(|p| {
                        let mut data = to_complex(p);
                        fft2(&mut data, size.0, size.1, false);
                        data.iter().map(|v| v.norm()).collect()
                    })
                    .collect();
                self.accumulate(amplitudes);
            }
        }
        self.count += 1;
        Ok(())
    }

    fn accumulate(&mut self, values: Vec<Vec<f64>>) {
        if self.sums.is_empty() {
            self.sums = values;
        } else {
            for (sum, v) in self.sums.iter_mut().zip(values) {
                sum.iter_mut().zip(v).for_each(|(s, x)| *s += x);
            }
        }
    }

    pub fn finish(self) -> crate::Result<Target> {
        if self.count == 0 {
            anyhow::bail!("The reference set is empty");
        }
        let n = self.count as f64;
        let average = |sums: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            sums.into_iter().map(|s| s.into_iter().map(|v| v / n).collect()).collect()
        };
        Ok(match self.property {
            MatchProperty::Luminance => Target::Luminance { mean: self.luminance.0 / n, rms_contrast: self.luminance.1 / n },
            MatchProperty::Histogram => Target::Histogram(average(self.sums)),
            MatchProperty::Spectrum => {
                let (width, height) = self.size.unwrap_or_default();
                Target::Spectrum { width, height, amplitudes: average(self.sums) }
            }
        })
    }
}

/// Matches `planes` to `target` in place. Values may leave 0..1; clip or compress afterwards.
pub fn apply(planes: &mut Planes, target: &Target) -> crate::Result<()> {
    match target {
        Target::Luminance { mean, rms_contrast } => {
            let (m, sd) = mean_sd(&planes.luminance());
            // Luma weights sum to one, so the same affine map on every channel maps luma the same way.
            let gain = if sd > 0.0 { rms_contrast / sd } else { 1.0 };
            for plane in &mut planes.color {
                plane.iter_mut().for_each(|v| *v = ((*v as f64 - m) * gain + mean) as f32);
            }
        }
        Target::Histogram(quantiles) => {
            check_channels(planes, quantiles.len())?;
            planes.color.par_iter_mut().zip(quantiles).for_each(|(plane, q)| specify_histogram(plane, q));
        }
        Target::Spectrum { width, height, amplitudes } => {
            check_channels(planes, amplitudes.len())?;
            if (planes.width, planes.height) != (*width, *height) {
                anyhow::bail!(
                    "Spectrum target is {width} x {height} but the image is {} x {}",
                    planes.width,
                    planes.height
                );
            }
            let (w, h) = (*width, *height);
            planes.color.par_iter_mut().zip(amplitudes).for_each(|(plane, amplitude)| {
                let mut data = to_complex(plane);
                fft2(&mut data, w, h, false);
                for (v, &a) in data.iter_mut().zip(amplitude) {
                    let phase = if v.norm() > 0.0 { v.arg() } else { 0.0 };
                    *v = Complex64::from_polar(a, phase);
                }
                fft2(&mut data, w, h, true);
                *plane = data.iter().map(|v| v.re as f32).collect();
            });
        }
    }
    Ok(())
}

fn check_channels(planes: &Planes, channels: usize) -> crate::Result<()> {
    if planes.color.len() != channels {
        anyhow::bail!("Target has {channels} channel(s) but the image has {}", planes.color.len());
    }
    Ok(())
}

/// Sorted values of `plane` resampled to `resolution` evenly spaced quantiles.
fn resample_sorted(plane: &[f32], resolution: usize) -> Vec<f64> {
    let mut sorted = plane.to_vec();
    sorted.sort_by(f32::total_cmp);
    (0..resolution)
        .map(|i| quantile(&sorted, (i as f64 + 0.5) / resolution as f64, |v| v as f64))
        .collect()
}

/// Exact histogram specification: pixels are ranked (ties broken by position) and given the
/// target value at the same quantile.
fn specify_histogram(plane: &mut Vec<f32>, target: &[f64]) {
    let n = plane.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| plane[a].total_cmp(&plane[b]).then(a.cmp(&b)));
    let mut out = vec![0.0f32; n];
    for (rank, &idx) in order.iter().enumerate() {
        out[idx] = quantile(target, (rank as f64 + 0.5) / n as f64, |v| v) as f32;
    }
    *plane = out;
}

/// Linear interpolation into ascending `sorted` values at quantile `q`.
fn quantile<T: Copy>(sorted: &[T], q: f64, value: impl Fn(T) -> f64) -> f64 {
    let pos = q * sorted.len() as f64 - 0.5;
    let i = pos.floor().clamp(0.0, (sorted.len() - 1) as f64) as usize;
    let j = (i + 1).min(sorted.len() - 1);
    let t = (pos - i as f64).clamp(0.0, 1.0);
    value(sorted[i]) * (1.0 - t) + value(sorted[j]) * t
}
