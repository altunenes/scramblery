//! Local phase scrambling with a complex steerable filter bank.
//!
//! Each plane is split into `scales` x `orientations` analytic bands, G(f, θ) = R_s(f) · A_o(θ),
//! plus a low-pass residual. R_s is a raised cosine in log2 frequency (scale 0 centred on 1/4
//! cycle per pixel, one octave per scale); A_o is cos^(M-1)(θ - πo/M) on one half-plane. The
//! squared filters sum to 1, so an unscrambled synthesis returns the input exactly.
//!
//! Scrambling rotates each band coefficient by a random offset (smooth over about one
//! wavelength, uniform on (-π, π), times the intensity) and keeps its modulus. Local amplitude
//! and the low-pass residual stay, so contrast stays where it was and only its fine arrangement
//! changes. All channels share the offsets.
//!
//! Scale s runs on a grid 2^s times smaller, which is exact since it holds nothing above
//! 0.5 / 2^s cycles per pixel. Reflect and mean padding add margins of two coarsest wavelengths,
//! and the transform size is rounded up to a product of 2, 3 and 5.

use num_complex::Complex64;
use rand::Rng;
use rayon::prelude::*;

use super::types::{LocalPhaseOptions, PaddingMode};
use crate::planes::Planes;
use crate::seed;
use crate::spectral::{bin_frequency, fft2, mirror_index};

/// Centre frequency of the finest scale in cycles per pixel.
pub const FINEST_FREQUENCY: f64 = 0.25;

/// The transform grid of one scale: its size and, for each of its bins, the bin of the full
/// transform it stands for (`None` where the scale has no energy).
struct Grid {
    width: usize,
    height: usize,
    bins: Vec<Option<usize>>,
}

struct Band {
    scale: usize,
    /// Filter values on the scale's grid.
    filter: Vec<f64>,
}

struct FilterBank {
    grids: Vec<Grid>,
    bands: Vec<Band>,
    lowpass_squared: Vec<f64>,
}

pub fn scramble(input: &Planes, options: &LocalPhaseOptions, rng: &mut seed::Rng) -> Planes {
    let offsets = PhaseOffsets::new(input.width, input.height, options, rng);
    scramble_with(input, options, &offsets)
}

/// Random phase offsets of a local phase scramble, one field per band on its grid. Kept apart
/// from the scramble so that follow motion can move them with the content.
#[derive(Clone)]
pub struct PhaseOffsets {
    width: usize,
    height: usize,
    layout: Layout,
    /// Grid width and height and the offsets of each band.
    bands: Vec<(usize, usize, Vec<f64>)>,
}

impl PhaseOffsets {
    /// The offsets `scramble` draws with the same `rng` for a `width` x `height` image.
    pub fn new(width: usize, height: usize, options: &LocalPhaseOptions, rng: &mut seed::Rng) -> Self {
        let scales = options.scales.max(1) as usize;
        let layout = Layout::new(width, height, scales, options.padding_mode);
        let bands = (0..scales)
            .flat_map(|s| (0..options.orientations.max(1)).map(move |_| s))
            .map(|s| {
                let factor = 1usize << s.min(30);
                let (gw, gh) = (layout.width.div_ceil(factor).max(1), layout.height.div_ceil(factor).max(1));
                (gw, gh, offset_field(gw, gh, scale_frequency(s) * factor as f64, rng))
            })
            .collect();
        Self { width, height, layout, bands }
    }

    /// Position in the image (pixels) of sample (gx, gy) of a `gw` x `gh` band grid.
    fn image_position(&self, gx: usize, gy: usize, gw: usize, gh: usize) -> (f32, f32) {
        let x = gx as f64 * self.layout.width as f64 / gw as f64 - self.layout.left as f64;
        let y = gy as f64 * self.layout.height as f64 / gh as f64 - self.layout.top as f64;
        (x as f32, y as f32)
    }

    /// The offsets moved with the content: `coords` gives, per image pixel, where its content
    /// came from. Offsets are interpolated as unit phasors, since averaging angles breaks at the
    /// ±π wrap. Margins use the displacement at the nearest edge.
    pub fn carry(&self, coords: &Planes) -> Self {
        let (w, h) = (self.width, self.height);
        let (xs, ys) = (&coords.color[0], &coords.color[1]);
        let bands = self
            .bands
            .par_iter()
            .map(|(gw, gh, offsets)| {
                let (gw, gh) = (*gw, *gh);
                let (sx, sy) = (self.layout.width as f64 / gw as f64, self.layout.height as f64 / gh as f64);
                let moved = (0..gw * gh)
                    .map(|j| {
                        let (x, y) = self.image_position(j % gw, j / gw, gw, gh);
                        let (cx, cy) = (x.clamp(0.0, (w - 1) as f32), y.clamp(0.0, (h - 1) as f32));
                        let dx = bilinear(xs, w, h, cx, cy) - cx;
                        let dy = bilinear(ys, w, h, cx, cy) - cy;
                        let gx = ((x + dx) as f64 + self.layout.left as f64) / sx;
                        let gy = ((y + dy) as f64 + self.layout.top as f64) / sy;
                        let (re, im) = phasor_at(offsets, gw, gh, gx, gy);
                        im.atan2(re)
                    })
                    .collect();
                (gw, gh, moved)
            })
            .collect();
        Self { bands, ..self.clone() }
    }

    /// Weighted blend (weights per image pixel) as the angle of the summed phasors. Independent
    /// uniform phases stay uniform this way, so the scramble keeps its strength.
    pub fn blend(fields: &[(&PhaseOffsets, &[f32])]) -> Self {
        let first = fields[0].0;
        let (w, h) = (first.width, first.height);
        let bands = (0..first.bands.len())
            .into_par_iter()
            .map(|b| {
                let (gw, gh, _) = first.bands[b];
                let combined = (0..gw * gh)
                    .map(|j| {
                        let (x, y) = first.image_position(j % gw, j / gw, gw, gh);
                        let i = (y.round().clamp(0.0, (h - 1) as f32) as usize) * w + x.round().clamp(0.0, (w - 1) as f32) as usize;
                        let (re, im) = fields.iter().fold((0.0, 0.0), |(re, im), (f, a)| {
                            let t = f.bands[b].2[j];
                            (re + a[i] as f64 * t.cos(), im + a[i] as f64 * t.sin())
                        });
                        im.atan2(re)
                    })
                    .collect();
                (gw, gh, combined)
            })
            .collect();
        Self { bands, ..first.clone() }
    }
}

/// Bilinear sample with coordinates clamped to the image.
fn bilinear(values: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: usize, y: usize| values[y * w + x];
    at(x0, y0) * (1.0 - fx) * (1.0 - fy) + at(x1, y0) * fx * (1.0 - fy) + at(x0, y1) * (1.0 - fx) * fy + at(x1, y1) * fx * fy
}

/// Unit phasor of the offsets interpolated bilinearly at grid position (x, y); the padded
/// transform is periodic, so positions wrap around.
fn phasor_at(offsets: &[f64], gw: usize, gh: usize, x: f64, y: f64) -> (f64, f64) {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let wrap = |v: f64, n: usize| (v as i64).rem_euclid(n as i64) as usize;
    let mut sum = (0.0, 0.0);
    for (dy, wy) in [(0.0, 1.0 - fy), (1.0, fy)] {
        for (dx, wx) in [(0.0, 1.0 - fx), (1.0, fx)] {
            let t = offsets[wrap(y0 + dy, gh) * gw + wrap(x0 + dx, gw)];
            sum = (sum.0 + wx * wy * t.cos(), sum.1 + wx * wy * t.sin());
        }
    }
    sum
}

/// A local phase scramble of `input` with the given offsets (see `PhaseOffsets`).
pub fn scramble_with(input: &Planes, options: &LocalPhaseOptions, offsets: &PhaseOffsets) -> Planes {
    let (width, height) = (input.width, input.height);
    let scales = options.scales.max(1) as usize;
    let layout = &offsets.layout;
    let (pw, ph) = (layout.width, layout.height);
    let bank = filter_bank(pw, ph, scales, options.orientations.max(1) as usize);
    let offsets: Vec<&Vec<f64>> = offsets.bands.iter().map(|(_, _, o)| o).collect();
    let strength = options.intensity.clamp(0.0, 1.0) as f64;

    let color = input
        .color
        .iter()
        .map(|plane| {
            let mut spectrum = layout.pad(plane, width, height, options.padding_mode);
            fft2(&mut spectrum, pw, ph, false);
            let scrambled: Vec<Vec<Complex64>> = bank
                .bands
                .par_iter()
                .zip(&offsets)
                .map(|(band, offset)| scramble_band(&spectrum, &bank.grids[band.scale], &band.filter, offset, strength))
                .collect();
            let mut synthesis = vec![Complex64::new(0.0, 0.0); pw * ph];
            for (band, values) in bank.bands.iter().zip(scrambled) {
                for (bin, v) in bank.grids[band.scale].bins.iter().zip(values) {
                    if let Some(i) = bin {
                        synthesis[*i] += v;
                    }
                }
            }
            // Each band contributes Z(k) + conj(Z(-k)), the spectrum of 2 Re(z): the analytic
            // band response and its mirror image together form a real signal.
            let mut output: Vec<Complex64> = (0..pw * ph)
                .into_par_iter()
                .map(|i| {
                    let m = mirror_index(i % pw, i / pw, pw, ph);
                    spectrum[i] * bank.lowpass_squared[i] + synthesis[i] + synthesis[m].conj()
                })
                .collect();
            fft2(&mut output, pw, ph, true);
            layout.crop(&output, width, height)
        })
        .collect();

    Planes { color, ..input.clone() }
}

/// The padded transform: its size and where the image sits in it.
#[derive(Clone)]
struct Layout {
    width: usize,
    height: usize,
    left: usize,
    top: usize,
}

impl Layout {
    fn new(width: usize, height: usize, scales: usize, mode: PaddingMode) -> Self {
        if mode == PaddingMode::Wrap {
            return Self { width, height, left: 0, top: 0 };
        }
        // Two wavelengths of the coarsest scale: 2 / (FINEST_FREQUENCY / 2^(S-1)) = 2^(S+2).
        let margin = 1usize << (scales + 2).min(30);
        let (mx, my) = (margin.min(width), margin.min(height));
        let (pw, ph) = (fast_size(width + 2 * mx), fast_size(height + 2 * my));
        Self { width: pw, height: ph, left: (pw - width) / 2, top: (ph - height) / 2 }
    }

    /// The plane placed in the padded transform, with mirrored or mean margins.
    fn pad(&self, plane: &[f32], width: usize, height: usize, mode: PaddingMode) -> Vec<Complex64> {
        let mean = plane.iter().map(|&v| v as f64).sum::<f64>() / plane.len().max(1) as f64;
        // Mirror about the image edges (each edge pixel repeated once), any number of times.
        let mirror = |i: isize, n: usize| {
            let period = 2 * n as isize;
            let m = i.rem_euclid(period);
            (if m < n as isize { m } else { period - 1 - m }) as usize
        };
        (0..self.width * self.height)
            .map(|i| {
                let (x, y) = ((i % self.width) as isize - self.left as isize, (i / self.width) as isize - self.top as isize);
                let inside = x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height;
                let value = match mode {
                    PaddingMode::Mean if !inside => mean,
                    _ => plane[mirror(y, height) * width + mirror(x, width)] as f64,
                };
                Complex64::new(value, 0.0)
            })
            .collect()
    }

    fn crop(&self, data: &[Complex64], width: usize, height: usize) -> Vec<f32> {
        (0..height)
            .flat_map(|y| {
                let row = (y + self.top) * self.width + self.left;
                data[row..row + width].iter().map(|v| v.re as f32)
            })
            .collect()
    }
}

/// The smallest number of at least `n` whose only prime factors are 2, 3 and 5.
fn fast_size(n: usize) -> usize {
    (n.max(1)..)
        .find(|&m| {
            let mut r = m;
            for p in [2, 3, 5] {
                while r % p == 0 {
                    r /= p;
                }
            }
            r == 1
        })
        .expect("a 5-smooth number exists above any n")
}

/// Analyses one band on its grid, adds the phase offsets and returns the band's synthesis
/// spectrum on the same grid.
fn scramble_band(spectrum: &[Complex64], grid: &Grid, filter: &[f64], offset: &[f64], strength: f64) -> Vec<Complex64> {
    let zero = Complex64::new(0.0, 0.0);
    let analysis: Vec<Complex64> =
        grid.bins.iter().zip(filter).map(|(bin, g)| bin.map_or(zero, |i| spectrum[i] * g)).collect();
    let mut band = analysis.clone();
    fft2(&mut band, grid.width, grid.height, true);
    band.iter_mut().zip(offset).for_each(|(c, t)| *c *= Complex64::from_polar(1.0, strength * t));
    fft2(&mut band, grid.width, grid.height, false);
    band.iter_mut().zip(filter).for_each(|(c, g)| *c *= g);
    // Changing the phase moves some energy outside the band, which the synthesis filter
    // removes; the band's energy is restored to that of the unchanged band.
    let original: f64 = analysis.iter().zip(filter).map(|(x, g)| (x * g).norm_sqr()).sum();
    let changed: f64 = band.iter().map(|c| c.norm_sqr()).sum();
    if changed > 0.0 {
        let gain = (original / changed).sqrt();
        band.iter_mut().for_each(|c| *c *= gain);
    }
    band
}

/// Centre frequency of scale `s` in cycles per pixel.
pub fn scale_frequency(s: usize) -> f64 {
    FINEST_FREQUENCY / (1u64 << s.min(62)) as f64
}

/// Random phase offsets on a `width` x `height` grid for a band centred on `centre` cycles per
/// sample: Gaussian white noise, low-pass filtered with a Gaussian of standard deviation
/// `centre / 2`, scaled to unit variance and mapped through the normal distribution function to
/// a uniform distribution on (-π, π).
fn offset_field(width: usize, height: usize, centre: f64, rng: &mut seed::Rng) -> Vec<f64> {
    let mut noise: Vec<Complex64> = (0..width * height)
        .map(|_| {
            // Box-Muller transform of two uniform samples.
            let (u, v) = (1.0 - rng.random::<f64>(), rng.random::<f64>());
            Complex64::new((-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos(), 0.0)
        })
        .collect();
    fft2(&mut noise, width, height, false);
    let sigma = centre / 2.0;
    for ky in 0..height {
        for kx in 0..width {
            let f2 = bin_frequency(kx, width).powi(2) + bin_frequency(ky, height).powi(2);
            noise[ky * width + kx] *= (-f2 / (2.0 * sigma * sigma)).exp();
        }
    }
    fft2(&mut noise, width, height, true);
    let n = noise.len() as f64;
    let mean = noise.iter().map(|v| v.re).sum::<f64>() / n;
    let sd = (noise.iter().map(|v| (v.re - mean).powi(2)).sum::<f64>() / n).sqrt();
    let sd = if sd > 0.0 { sd } else { 1.0 };
    noise.iter().map(|v| std::f64::consts::PI * erf((v.re - mean) / (sd * std::f64::consts::SQRT_2))).collect()
}

/// Error function (Abramowitz & Stegun 7.1.26, absolute error below 1.5e-7).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    (1.0 - poly * (-x * x).exp()).copysign(x)
}

/// Raised-cosine transition from 1 at `t <= 0` to 0 at `t >= 1`, as the cosine of a smoothstep
/// so that its slope is zero at both ends. The complementary window is the matching sine.
fn transition(t: f64) -> (f64, f64) {
    let t = t.clamp(0.0, 1.0);
    let angle = std::f64::consts::FRAC_PI_2 * t * t * (3.0 - 2.0 * t);
    (angle.cos(), angle.sin())
}

/// Radial window R_s, and with `s == scales` the low-pass residual L, at frequency `f` (cycles
/// per pixel). The squares of R_0 .. R_{S-1} and L sum to 1.
fn radial_window(f: f64, s: usize, scales: usize) -> f64 {
    if f <= 0.0 {
        return if s == scales { 1.0 } else { 0.0 };
    }
    // Octaves below the finest centre frequency: 0 at scale 0, s at scale s.
    let u = (FINEST_FREQUENCY / f).log2();
    if u <= 0.0 {
        return if s == 0 { 1.0 } else { 0.0 };
    }
    let below = u.floor() as usize;
    let (upper, lower) = transition(u - below as f64);
    if s == below && below < scales {
        upper
    } else if s == below + 1 && below < scales {
        lower
    } else if s == scales && below >= scales {
        1.0
    } else {
        0.0
    }
}

/// One-sided angular window of orientation `o` of `m` at angle `theta`, before normalisation.
/// On the boundary of the half-plane the window is 0, except with a single orientation, where
/// the boundary is shared equally between θ and θ + π.
fn angular_window(theta: f64, o: usize, m: usize) -> f64 {
    let c = (theta - std::f64::consts::PI * o as f64 / m as f64).cos();
    if c.abs() < 1e-9 {
        if m == 1 { std::f64::consts::FRAC_1_SQRT_2 } else { 0.0 }
    } else if c > 0.0 {
        c.powi(m as i32 - 1)
    } else {
        0.0
    }
}

/// Angular weight of orientation `o` at bin (kx, ky). A bin and its mirror are paired
/// explicitly (θ from the lower index, the mirror at θ + π), because on the Nyquist row or
/// column their signed frequencies do not differ by π. A self-mirrored bin gets the RMS of both.
fn angular_weight(kx: usize, ky: usize, pw: usize, ph: usize, o: usize, orientations: usize, norm: f64) -> f64 {
    let i = ky * pw + kx;
    let m = mirror_index(kx, ky, pw, ph);
    let (rx, ry) = if m < i { (m % pw, m / pw) } else { (kx, ky) };
    let theta = bin_frequency(ry, ph).atan2(bin_frequency(rx, pw));
    let a = angular_window(theta, o, orientations) / norm;
    let b = angular_window(theta + std::f64::consts::PI, o, orientations) / norm;
    if m == i {
        ((a * a + b * b) / 2.0).sqrt()
    } else if m < i {
        b
    } else {
        a
    }
}

/// The grid of scale `s`: the full transform downsampled by 2^s. Its bins cover signed
/// frequencies below 0.5 / 2^s cycles per pixel, which contain all of the scale's energy.
fn scale_grid(pw: usize, ph: usize, s: usize) -> Grid {
    let factor = 1usize << s.min(30);
    if factor == 1 {
        return Grid { width: pw, height: ph, bins: (0..pw * ph).map(Some).collect() };
    }
    let (width, height) = (pw.div_ceil(factor).max(1), ph.div_ceil(factor).max(1));
    let signed = |j: usize, n: usize| if j < n.div_ceil(2) { j as isize } else { j as isize - n as isize };
    let inside = |k: isize, full: usize| (2 * k.unsigned_abs() * factor) < full;
    let bins = (0..width * height)
        .map(|j| {
            let (sx, sy) = (signed(j % width, width), signed(j / width, height));
            (inside(sx, pw) && inside(sy, ph)).then(|| sy.rem_euclid(ph as isize) as usize * pw + sx.rem_euclid(pw as isize) as usize)
        })
        .collect();
    Grid { width, height, bins }
}

fn filter_bank(pw: usize, ph: usize, scales: usize, orientations: usize) -> FilterBank {
    // The sum over orientations of the squared windows (counting θ and θ + π) is the same for
    // every θ; it is measured once and divided out.
    let norm = (0..orientations)
        .map(|o| angular_window(0.1, o, orientations).powi(2) + angular_window(0.1 + std::f64::consts::PI, o, orientations).powi(2))
        .sum::<f64>()
        .sqrt();
    let grids: Vec<Grid> = (0..scales).map(|s| scale_grid(pw, ph, s)).collect();
    let bands = (0..scales)
        .flat_map(|s| (0..orientations).map(move |o| (s, o)))
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|(s, o)| {
            let filter = grids[s]
                .bins
                .iter()
                .map(|bin| {
                    bin.map_or(0.0, |i| {
                        let (kx, ky) = (i % pw, i / pw);
                        let radial = radial_window(bin_frequency(kx, pw).hypot(bin_frequency(ky, ph)), s, scales);
                        if radial == 0.0 { 0.0 } else { radial * angular_weight(kx, ky, pw, ph, o, orientations, norm) }
                    })
                })
                .collect();
            Band { scale: s, filter }
        })
        .collect();
    let lowpass_squared = (0..pw * ph)
        .into_par_iter()
        .map(|i| radial_window(bin_frequency(i % pw, pw).hypot(bin_frequency(i / pw, ph)), scales, scales).powi(2))
        .collect();
    FilterBank { grids, bands, lowpass_squared }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_windows_partition_unity() {
        for scales in 1..6 {
            for i in 0..2000 {
                let f = i as f64 / 2000.0 * 0.75;
                let total: f64 = (0..=scales).map(|s| radial_window(f, s, scales).powi(2)).sum();
                assert!((total - 1.0).abs() < 1e-12, "f = {f}, scales = {scales}: {total}");
            }
        }
    }

    /// Every bin of the full transform: low-pass² + Σ over bands of G(k)² + G(-k)² = 1, with
    /// band values read back from the scale grids. Also checks that no band has energy outside
    /// its grid.
    #[test]
    fn filters_reconstruct_every_bin() {
        for (pw, ph) in [(16, 12), (15, 9), (32, 32), (37, 64)] {
            for orientations in 1..6 {
                let scales = 4;
                let bank = filter_bank(pw, ph, scales, orientations);
                let mut full = vec![vec![0.0; pw * ph]; bank.bands.len()];
                for (b, band) in bank.bands.iter().enumerate() {
                    for (bin, g) in bank.grids[band.scale].bins.iter().zip(&band.filter) {
                        if let Some(i) = bin {
                            full[b][*i] = *g;
                        }
                    }
                }
                let norm = (0..orientations)
                    .map(|o| angular_window(0.1, o, orientations).powi(2) + angular_window(0.1 + std::f64::consts::PI, o, orientations).powi(2))
                    .sum::<f64>()
                    .sqrt();
                for i in 0..pw * ph {
                    let (kx, ky) = (i % pw, i / pw);
                    let f = bin_frequency(kx, pw).hypot(bin_frequency(ky, ph));
                    for (b, band) in bank.bands.iter().enumerate() {
                        let expected = radial_window(f, band.scale, scales) * angular_weight(kx, ky, pw, ph, b % orientations, orientations, norm);
                        assert!((full[b][i] - expected).abs() < 1e-12, "{pw}x{ph}: band {b} missing energy at bin {i}");
                    }
                    let m = mirror_index(kx, ky, pw, ph);
                    let total = bank.lowpass_squared[i] + full.iter().map(|g| g[i] * g[i] + g[m] * g[m]).sum::<f64>();
                    assert!((total - 1.0).abs() < 1e-9, "{pw}x{ph}, {orientations} orientations, bin {i}: {total}");
                }
            }
        }
    }

    #[test]
    fn padding_places_the_image_and_mirrors_it() {
        let (w, h) = (5, 3);
        let plane: Vec<f32> = (0..w * h).map(|i| i as f32).collect();
        let layout = Layout::new(w, h, 1, PaddingMode::Reflect);
        assert!(layout.width >= w + 2 * w.min(8) && layout.height >= h + 2 * h.min(8));
        let padded = layout.pad(&plane, w, h, PaddingMode::Reflect);
        assert_eq!(layout.crop(&padded, w, h), plane);
        // Left of the image: the first column repeated, then the second.
        let row = layout.top * layout.width + layout.left;
        assert_eq!(padded[row - 1].re, plane[0] as f64);
        assert_eq!(padded[row - 2].re, plane[1] as f64);
        assert_eq!(fast_size(1687), 1728);
        assert_eq!(fast_size(1250), 1250);
    }

    #[test]
    fn carried_offsets_without_motion_give_the_same_scramble() {
        let (w, h) = (24, 16);
        let input = Planes { width: w, height: h, color: vec![(0..w * h).map(|i| ((i * 37) % 11) as f32 / 10.0).collect()], alpha: None, sixteen_bit: false };
        let options = LocalPhaseOptions { scales: 2, orientations: 3, ..Default::default() };
        let direct = scramble(&input, &options, &mut seed::rng(5));
        let offsets = PhaseOffsets::new(w, h, &options, &mut seed::rng(5));
        let xs = (0..w * h).map(|i| (i % w) as f32).collect();
        let ys = (0..w * h).map(|i| (i / w) as f32).collect();
        let still = Planes { color: vec![xs, ys], ..input.clone() };
        let carried = offsets.carry(&still);
        let blended = PhaseOffsets::blend(&[(&carried, &vec![0.7; w * h]), (&offsets, &vec![0.3; w * h])]);
        let again = scramble_with(&input, &options, &blended);
        for (a, b) in direct.color[0].iter().zip(&again.color[0]) {
            assert!((a - b).abs() < 1e-4, "{a} vs {b}");
        }
    }

    #[test]
    fn offsets_are_uniform_on_the_circle() {
        let field = offset_field(64, 64, 0.1, &mut seed::rng(1));
        let mut counts = [0usize; 8];
        for t in &field {
            assert!(t.abs() < std::f64::consts::PI);
            counts[((t + std::f64::consts::PI) / std::f64::consts::TAU * 8.0) as usize] += 1;
        }
        let expected = field.len() as f64 / 8.0;
        assert!(counts.iter().all(|&c| (c as f64 - expected).abs() < 0.25 * expected), "{counts:?}");
    }
}
