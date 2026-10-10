//! Local phase scrambling with a complex steerable filter bank.
//!
//! Each colour plane is split in the frequency domain into `scales` × `orientations` complex
//! bands and a low-pass residual. Band (s, o) has the filter G(f, θ) = R_s(f) · A_o(θ):
//!
//! - R_s is a raised-cosine window in log2 frequency. Scale 0 is centred on 1/4 cycle per
//!   pixel and also covers all higher frequencies; each further scale is one octave lower.
//!   Adjacent windows overlap so that the sum of the squared windows plus the squared low-pass
//!   residual is exactly 1 at every frequency.
//! - A_o is cos^(M-1)(θ - θ_o) on the half-plane facing θ_o = π o / M and zero elsewhere, so the
//!   band response is complex. The squared windows of the M orientations sum to a constant.
//!
//! The band responses are analytic, and their modulus and argument are the local amplitude and
//! local phase at each position, scale and orientation. Scrambling adds a random phase offset,
//! scaled by the intensity, to every band coefficient while keeping its modulus, and the image
//! is resynthesised with the same filters. The offsets of a band vary smoothly over about one
//! wavelength of the band and are uniformly distributed on (-π, π). Without scrambling the
//! synthesis returns the input exactly. The low-pass residual, which holds the mean and the
//! coarsest structure, is kept, so contrast energy stays where it was in the image and only its
//! fine arrangement is randomised. The same offsets are used for every colour channel.
//!
//! Scale s holds only frequencies below 0.5 / 2^s cycles per pixel, so its bands are processed
//! on a grid 2^s times smaller than the image. This is exact: the band spectrum is moved to the
//! smaller grid and back without loss.
//!
//! With reflect or mean padding the image gets margins of two wavelengths of the coarsest scale
//! on every side (at most the image size), mirrored or filled with the mean, and the transform
//! size is rounded up to a product of 2, 3 and 5.

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
    /// Downsampling factor relative to the full transform.
    factor: usize,
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
    let (width, height) = (input.width, input.height);
    let scales = options.scales.max(1) as usize;
    let layout = Layout::new(width, height, scales, options.padding_mode);
    let (pw, ph) = (layout.width, layout.height);
    let bank = filter_bank(pw, ph, scales, options.orientations.max(1) as usize);
    let offsets: Vec<Vec<f64>> = bank
        .bands
        .iter()
        .map(|band| {
            let grid = &bank.grids[band.scale];
            offset_field(grid.width, grid.height, scale_frequency(band.scale) * grid.factor as f64, rng)
        })
        .collect();
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

/// Angular weight of orientation `o` at bin (kx, ky) of a `pw` x `ph` transform.
///
/// A bin and its mirror image are treated as a pair, the mirror at θ + π, where θ is taken from
/// the lower-indexed bin. On the Nyquist row or column the signed frequencies of the two bins
/// do not differ by π, so the pairing has to be explicit. A bin that is its own mirror stands
/// for both angles and gets the root mean square of the two one-sided values.
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
        return Grid { width: pw, height: ph, factor, bins: (0..pw * ph).map(Some).collect() };
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
    Grid { width, height, factor, bins }
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
