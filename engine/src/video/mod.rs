use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use rayon::prelude::*;
use image::{DynamicImage, RgbaImage};
use log::info;
use serde::{Deserialize, Serialize};
use video_processor::{Batching, ColorMatrix, VideoProcessor, YuvFrame};
pub use video_processor::VideoFidelity;

use crate::matching::Target;
use crate::planes::Planes;
use crate::scramble::aperture::FaceBox;
use crate::scramble::local_phase::PhaseOffsets;
use crate::scramble::{postprocess, scramble_planes, BackgroundMode, ScrambleOptions, ScrambleType, TemporalCoherenceOptions};
use crate::seed;

pub mod spatiotemporal;

/// Frames without a detected face reuse the last face mask for at most this many frames.
pub const FACE_HOLD_FRAMES: usize = 15;

/// Frames over which consecutive spatiotemporal volumes are crossfaded.
pub const SPATIOTEMPORAL_OVERLAP: usize = 8;

// Follow motion. Sizes are fractions of the shorter frame side.
/// Gaussian smoothing of the flow.
const FLOW_SMOOTHING: f32 = 0.01;
/// Window for the local correlation in `blend_layers`.
const LAYER_STATISTICS_SMOOTHING: f32 = 0.03;
/// Layer distortion (see `distortion`) at which its weight is halved.
const HALF_WEIGHT_DISTORTION: f32 = 1.25;
/// Weight of the current frame's own scramble; it takes over where both layers fail.
const STILL_WEIGHT: f32 = 0.25;
/// Radial bands for estimating what a scramble keeps (see `kept_transfer`).
const KEPT_BANDS: usize = 48;
/// Allowed miss in pixels (plus 5% of the motion) of a forward-then-back flow round trip.
const ROUND_TRIP_TOLERANCE: f32 = 1.0;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum FrameSeeds {
    /// The same seed for every frame: the same noise, warp or permutation is applied to each frame.
    #[default]
    Fixed,
    /// A new seed for every frame (`seed::derive(base_seed, frame)`): dynamic noise.
    PerFrame,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoProcessingOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub scramble_options: ScrambleOptions,
    #[serde(default)]
    pub frame_seeds: FrameSeeds,
    /// Follow the original motion with optical flow (see `process_video`).
    pub temporal_coherence: Option<TemporalCoherenceOptions>,
    /// Phase-scramble blocks of frames as spatiotemporal volumes instead of using the scramble
    /// method frame by frame (see `spatiotemporal`).
    #[serde(default)]
    pub spatiotemporal: Option<spatiotemporal::SpatiotemporalOptions>,
    #[serde(default)]
    pub fidelity: VideoFidelity,
    /// Colour standard of the input, replacing what the decoder reports; also used for the
    /// output. For files whose colour description is missing or wrong.
    #[serde(default)]
    pub color_standard: Option<ColorStandard>,
    /// Memory for spatiotemporal volumes in GB (default 4). More memory allows longer volumes.
    #[serde(default)]
    pub memory_budget_gb: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum ColorStandard {
    Bt601,
    Bt709,
    Bt2020,
}

impl From<ColorStandard> for ColorMatrix {
    fn from(c: ColorStandard) -> Self {
        match c {
            ColorStandard::Bt601 => ColorMatrix::Bt601,
            ColorStandard::Bt709 => ColorMatrix::Bt709,
            ColorStandard::Bt2020 => ColorMatrix::Bt2020,
        }
    }
}

/// Settings written next to the output video.
#[derive(Debug, Serialize, Deserialize)]
pub struct VideoRunSettings {
    pub scramblery_version: String,
    pub input: PathBuf,
    pub scramble_options: ScrambleOptions,
    #[serde(default)]
    pub frame_seeds: FrameSeeds,
    pub temporal_coherence: Option<TemporalCoherenceOptions>,
    #[serde(default)]
    pub spatiotemporal: Option<spatiotemporal::SpatiotemporalOptions>,
    pub fidelity: VideoFidelity,
    pub report: Option<VideoReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VideoReport {
    pub seed: u64,
    pub frames: usize,
    /// Frames in which no face was detected (face detection only). Such frames reuse the
    /// previous face mask for up to `FACE_HOLD_FRAMES` frames, after which nothing is scrambled.
    pub frames_without_faces: Option<usize>,
    /// Number of volumes the video was split into (spatiotemporal scrambling only); consecutive
    /// volumes are crossfaded over `SPATIOTEMPORAL_OVERLAP` frames.
    #[serde(default)]
    pub chunks: Option<usize>,
    /// Bits per sample of the written video: 10 for sources with more than 8 bits encoded
    /// losslessly or at high quality, otherwise 8.
    #[serde(default)]
    pub output_bit_depth: Option<u32>,
    /// Fraction of colour samples outside the displayable range (0..1), which are limited when
    /// written; matching to a contrast the frames cannot reach shows up here.
    #[serde(default)]
    pub clipped_fraction: f64,
}

struct State {
    frame: usize,
    prev_original: Option<Planes>,
    /// Follow motion: the frame that closed the previous interval, which opens the next one.
    keyframe: Option<Keyframe>,
    prev_mask: Option<FaceMask>,
    frames_since_face: usize,
    frames_without_faces: usize,
    chunks: usize,
    /// Spatiotemporal scramble of the previous volume's lookahead frames, by frame index.
    prev_tail: Vec<(usize, Planes)>,
    /// Target for matching to a set average.
    set_target: Option<Arc<Target>>,
    /// Colour samples written so far, and how many of them were outside 0..1.
    samples: u64,
    clipped: u64,
}

/// A frame where a follow-motion interval starts or ends, and what is carried from it.
struct Keyframe {
    index: usize,
    carried: Carried,
}

/// What follow motion carries from a keyframe.
enum Carried {
    /// What the scramble changed and how much of the original it keeps (see `kept_transfer`).
    Texture { changed: Planes, transfer: Vec<[f64; KEPT_BANDS]> },
    /// The displacement of a diffeomorphic scramble's warp (see `displacement`).
    Warp(Planes),
    /// The random phase offsets of a local phase scramble.
    Phase(PhaseOffsets),
}

impl Keyframe {
    fn texture(&self) -> (&Planes, &[[f64; KEPT_BANDS]]) {
        match &self.carried {
            Carried::Texture { changed, transfer } => (changed, transfer),
            _ => unreachable!("keyframes of this scramble carry a texture"),
        }
    }

    fn warp(&self) -> &Planes {
        match &self.carried {
            Carried::Warp(field) => field,
            _ => unreachable!("keyframes of a diffeomorphic scramble carry a warp"),
        }
    }

    fn phase(&self) -> &PhaseOffsets {
        match &self.carried {
            Carried::Phase(offsets) => offsets,
            _ => unreachable!("keyframes of a local phase scramble carry phase offsets"),
        }
    }
}

/// Number of colour samples and of those outside 0..1, which the display range cannot show.
fn out_of_range(planes: &Planes) -> (u64, u64) {
    let clipped = planes.color.iter().flatten().filter(|v| !(0.0..=1.0).contains(*v)).count();
    ((planes.len() * planes.color.len()) as u64, clipped as u64)
}

impl State {
    fn count(&mut self, counts: impl IntoIterator<Item = (u64, u64)>) {
        for (n, c) in counts {
            self.samples += n;
            self.clipped += c;
        }
    }
}

/// Face weights of a frame and the face boxes they were made from.
#[derive(Clone)]
struct FaceMask {
    weights: Vec<f32>,
    boxes: Vec<FaceBox>,
}

fn detect_mask(image: &RgbaImage, options: &ScrambleOptions) -> Result<Option<FaceMask>> {
    let Some(face_opts) = &options.face_detection else { return Ok(None) };
    let session = face_detection::load_face_detector(None)?;
    let regions = face_detection::detect_face_regions(
        &DynamicImage::ImageRgba8(image.clone()),
        session,
        face_opts.confidence_threshold,
        Some(face_opts.expansion_factor),
    )?;
    if regions.is_empty() {
        return Ok(None);
    }
    let image = DynamicImage::ImageRgba8(image.clone());
    Ok(Some(FaceMask {
        weights: crate::scramble::mask::image_weights(&image, &regions, face_opts)?,
        boxes: regions.iter().map(FaceBox::from).collect(),
    }))
}

/// Colour planes with three channels, expanding a grayscale result.
fn as_rgb(mut planes: Planes) -> Planes {
    if planes.is_grayscale() {
        let gray = planes.color[0].clone();
        planes.color = vec![gray.clone(), gray.clone(), gray];
    }
    planes
}

/// Scrambles the whole frame and post-processes it against the original frame. Values are
/// not clipped here: they are limited to the stored code range when converted back to YUV,
/// so colours outside the RGB cube survive the round trip.
fn fresh_scramble(original: &Planes, options: &ScrambleOptions, seed: u64, set_target: Option<&Target>) -> Result<Planes> {
    let source = if options.linear_light { original.to_linear() } else { original.clone() };
    let mut result = scramble_planes(&source, options, seed);
    postprocess(&mut result, &source, options, set_target)?;
    if options.linear_light {
        result = result.to_encoded();
    }
    Ok(as_rgb(result))
}

/// Kr and Kb of a YCbCr matrix.
fn coefficients(matrix: ColorMatrix) -> (f32, f32) {
    match matrix {
        ColorMatrix::Bt601 => (0.299, 0.114),
        ColorMatrix::Bt709 => (0.2126, 0.0722),
        ColorMatrix::Bt2020 => (0.2627, 0.0593),
    }
}

/// Luma scale, luma offset, chroma scale and chroma centre in code values for the frame's range
/// and sample size. Limited-range 16-bit levels are the 8-bit levels times 256.
fn range(full_range: bool, max_value: u16) -> (f32, f32, f32, f32) {
    let max = max_value as f32;
    if full_range {
        (max, 0.0, max, (max + 1.0) / 2.0)
    } else {
        let s = (max + 1.0) / 256.0;
        (219.0 * s, 16.0 * s, 224.0 * s, 128.0 * s)
    }
}

/// Converts a YUV frame to RGB planes in floating point with the frame's own matrix and range.
/// Out-of-gamut colours give values outside 0..1, which are kept.
pub fn yuv_to_planes(frame: &YuvFrame) -> Planes {
    let (kr, kb) = coefficients(frame.matrix);
    let kg = 1.0 - kr - kb;
    let (y_scale, y_offset, c_scale, c_centre) = range(frame.full_range, frame.max_value);
    let n = frame.width * frame.height;
    let mut color = vec![vec![0.0f32; n]; 3];
    let (r, rest) = color.split_at_mut(1);
    let (g, b) = rest.split_at_mut(1);
    r[0].par_iter_mut()
        .zip(g[0].par_iter_mut())
        .zip(b[0].par_iter_mut())
        .enumerate()
        .for_each(|(i, ((r, g), b))| {
            let y = (frame.y[i] as f32 - y_offset) / y_scale;
            let cb = (frame.u[i] as f32 - c_centre) / c_scale;
            let cr = (frame.v[i] as f32 - c_centre) / c_scale;
            *r = y + 2.0 * (1.0 - kr) * cr;
            *b = y + 2.0 * (1.0 - kb) * cb;
            *g = (y - kr * *r - kb * *b) / kg;
        });
    Planes { width: frame.width, height: frame.height, color, alpha: None, sixteen_bit: false }
}

/// Writes RGB planes into a YUV frame with the frame's matrix and range, rounding to the
/// nearest code value and limiting to the sample range.
pub fn planes_to_yuv(planes: &Planes, frame: &mut YuvFrame) {
    let (kr, kb) = coefficients(frame.matrix);
    let kg = 1.0 - kr - kb;
    let (y_scale, y_offset, c_scale, c_centre) = range(frame.full_range, frame.max_value);
    let max = frame.max_value as f32;
    let quantize = |v: f32| v.round().clamp(0.0, max) as u16;
    let (r, g, b) = (&planes.color[0], &planes.color[1], &planes.color[2]);
    frame.y.par_iter_mut()
        .zip(frame.u.par_iter_mut())
        .zip(frame.v.par_iter_mut())
        .enumerate()
        .for_each(|(i, ((y, u), v))| {
            let luma = kr * r[i] + kg * g[i] + kb * b[i];
            *y = quantize(luma * y_scale + y_offset);
            *u = quantize((b[i] - luma) / (2.0 * (1.0 - kb)) * c_scale + c_centre);
            *v = quantize((r[i] - luma) / (2.0 * (1.0 - kr)) * c_scale + c_centre);
        });
}

/// An 8-bit RGBA image of the planes, clipped to 0..1, for the face and flow models.
fn to_rgba8(planes: &Planes) -> RgbaImage {
    let (w, h) = (planes.width as u32, planes.height as u32);
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    RgbaImage::from_fn(w, h, |x, y| {
        let i = (y * w + x) as usize;
        image::Rgba([q(planes.color[0][i]), q(planes.color[1][i]), q(planes.color[2][i]), 255])
    })
}

/// Samples every plane at `p + flow(p)` with bilinear interpolation (backward warping).
fn warp(planes: &Planes, flow: &optical_flow::FlowField) -> Planes {
    let (w, h) = (planes.width, planes.height);
    let sample = |plane: &Vec<f32>| -> Vec<f32> {
        (0..w * h)
            .into_par_iter()
            .map(|i| {
                let x = (i % w) as f32 + flow.data[i];
                let y = (i / w) as f32 + flow.data[w * h + i];
                bilinear(plane, w, h, x, y)
            })
            .collect()
    };
    Planes {
        color: planes.color.iter().map(sample).collect(),
        alpha: planes.alpha.as_ref().map(sample),
        ..planes.clone()
    }
}

fn bilinear(values: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    values[y0 * w + x0] * (1.0 - fx) * (1.0 - fy)
        + values[y0 * w + x1] * fx * (1.0 - fy)
        + values[y1 * w + x0] * (1.0 - fx) * fy
        + values[y1 * w + x1] * fx * fy
}

/// Samples `planes` at `coords` (x and y planes), Catmull-Rom.
///
/// With `cell` (block size, 1 x 1 for pixels) whole cells are copied instead: each grid cell
/// looks up its centre and takes the source cell containing it. Interpolating blocks or pixel
/// noise blurs them and rotates blocks; this keeps a mosaic of intact cells that moves in
/// steps of one cell.
fn remap(planes: &Planes, coords: &Planes, cell: Option<(usize, usize)>) -> Planes {
    let (w, h) = (planes.width, planes.height);
    let (xs, ys) = (&coords.color[0], &coords.color[1]);
    let source: Vec<usize> = match cell {
        None => Vec::new(),
        Some((cw, ch)) => (0..w * h)
            .into_par_iter()
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let (x0, y0) = (x - x % cw, y - y % ch);
                let cx = (x0 as f32 + (cw - 1) as f32 / 2.0).min((w - 1) as f32);
                let cy = (y0 as f32 + (ch - 1) as f32 / 2.0).min((h - 1) as f32);
                let snap = |v: f32, size: usize, n: usize| ((((v + 0.5) / size as f32).floor().max(0.0) as usize) * size).min(n - 1);
                let sx = (snap(bilinear(xs, w, h, cx, cy), cw, w) + x - x0).min(w - 1);
                let sy = (snap(bilinear(ys, w, h, cx, cy), ch, h) + y - y0).min(h - 1);
                sy * w + sx
            })
            .collect(),
    };
    let sample = |plane: &Vec<f32>| -> Vec<f32> {
        if cell.is_some() {
            return source.par_iter().map(|&j| plane[j]).collect();
        }
        (0..w * h).into_par_iter().map(|i| bicubic(plane, w, h, xs[i], ys[i])).collect()
    };
    Planes {
        color: planes.color.iter().map(sample).collect(),
        alpha: planes.alpha.as_ref().map(sample),
        ..planes.clone()
    }
}

fn bicubic(values: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (x.floor(), y.floor());
    let (wx, wy) = (catmull_rom(x - x0), catmull_rom(y - y0));
    let at = |i: f32, n: usize| (i.max(0.0) as usize).min(n - 1);
    let mut sum = 0.0;
    for (j, wy) in wy.iter().enumerate() {
        let row = at(y0 + j as f32 - 1.0, h) * w;
        for (i, wx) in wx.iter().enumerate() {
            sum += wx * wy * values[row + at(x0 + i as f32 - 1.0, w)];
        }
    }
    sum
}

/// Catmull-Rom weights of the four samples around a position `t` (0..1) past the second.
fn catmull_rom(t: f32) -> [f32; 4] {
    let (t2, t3) = (t * t, t * t * t);
    [
        0.5 * (-t3 + 2.0 * t2 - t),
        0.5 * (3.0 * t3 - 5.0 * t2 + 2.0),
        0.5 * (-3.0 * t3 + 4.0 * t2 + t),
        0.5 * (t3 - t2),
    ]
}

/// Reliability of the flow `there`, checked against the flow `back`: 1 where the round trip
/// returns to the pixel, 0 where it misses by twice `ROUND_TRIP_TOLERANCE`. Misses mark content
/// that is covered, uncovered or leaves the frame.
fn round_trip(there: &optical_flow::FlowField, back: &optical_flow::FlowField) -> Vec<f32> {
    let (w, h) = (there.width, there.height);
    let (bx, by) = back.data.split_at(w * h);
    (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (dx, dy) = (there.data[i], there.data[w * h + i]);
            let (x, y) = ((i % w) as f32 + dx, (i / w) as f32 + dy);
            let miss = (dx + bilinear(bx, w, h, x, y)).hypot(dy + bilinear(by, w, h, x, y));
            let tolerance = ROUND_TRIP_TOLERANCE + 0.05 * dx.hypot(dy);
            (2.0 - miss / tolerance).clamp(0.0, 1.0)
        })
        .collect()
}

/// Values of one plane moved along `flow` (see `warp`).
fn carry(values: &[f32], flow: &optical_flow::FlowField) -> Vec<f32> {
    let (w, h) = (flow.width, flow.height);
    (0..w * h)
        .into_par_iter()
        .map(|i| bilinear(values, w, h, (i % w) as f32 + flow.data[i], (i / w) as f32 + flow.data[w * h + i]))
        .collect()
}

/// Coordinates of every pixel's own position.
fn identity_coords(w: usize, h: usize) -> Planes {
    let xs = (0..w * h).map(|i| (i % w) as f32).collect();
    let ys = (0..w * h).map(|i| (i / w) as f32).collect();
    Planes { width: w, height: h, color: vec![xs, ys], alpha: None, sixteen_bit: false }
}

fn smooth_flow(flow: &optical_flow::FlowField, sigma: f32) -> optical_flow::FlowField {
    let (w, h) = (flow.width, flow.height);
    let blur = |v: &[f32]| crate::scramble::blur::gaussian_blur(v, w, h, sigma);
    let mut data = blur(&flow.data[..w * h]);
    data.extend(blur(&flow.data[w * h..]));
    optical_flow::FlowField { width: w, height: h, data }
}

/// Stretch of the carried positions: `max(s1, 1 / s2)` from the singular values of their
/// Jacobian, 1 for rigid motion.
fn distortion(coords: &Planes) -> Vec<f32> {
    let (w, h) = (coords.width, coords.height);
    let (xs, ys) = (&coords.color[0], &coords.color[1]);
    let d = |v: &[f32], i: usize, dx: bool| -> f32 {
        let (x, y) = (i % w, i / w);
        if dx {
            let (a, b) = (x.saturating_sub(1), (x + 1).min(w - 1));
            (v[y * w + b] - v[y * w + a]) / (b - a).max(1) as f32
        } else {
            let (a, b) = (y.saturating_sub(1), (y + 1).min(h - 1));
            (v[b * w + x] - v[a * w + x]) / (b - a).max(1) as f32
        }
    };
    (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (a, b, c, e) = (d(xs, i, true), d(xs, i, false), d(ys, i, true), d(ys, i, false));
            let q = ((a + e) * (a + e) + (c - b) * (c - b)).sqrt() / 2.0;
            let r = ((a - e) * (a - e) + (c + b) * (c + b)).sqrt() / 2.0;
            let (s1, s2) = (q + r, (q - r).abs());
            s1.max(1.0 / s2.max(1e-3))
        })
        .collect()
}

/// Band 0 holds only the mean; the other bands split the radial frequencies evenly.
fn radial_band(kx: usize, ky: usize, w: usize, h: usize) -> usize {
    if kx == 0 && ky == 0 {
        return 0;
    }
    let r = crate::spectral::radial_frequency(kx, ky, w, h) / std::f64::consts::FRAC_1_SQRT_2;
    1 + ((r * (KEPT_BANDS - 1) as f64) as usize).min(KEPT_BANDS - 2)
}

/// What each channel of a scramble keeps of the original per radial band: the least-squares
/// gain `g = Σ Re(S·conj(O)) / Σ |O|²`. It is 1 for untouched frequencies, the transfer
/// function for blur, and near 0 for random phases.
///
/// Bands with few coefficients can show a sizeable `g` by chance, so I shrink it to
/// `(g - 2σ) / (1 - 2σ)` with `σ = sqrt(Σ |S|²|O|²) / Σ |O|²` (conjugate pairs counted), which
/// leaves exact 1s alone. The mean is its own band. The other bands use mean-removed planes with
/// a 10% Tukey border, so the wrap-around edge is not counted as kept detail.
fn kept_transfer(original: &Planes, scrambled: &Planes) -> Vec<[f64; KEPT_BANDS]> {
    let (w, h) = (original.width, original.height);
    original
        .color
        .iter()
        .zip(&scrambled.color)
        .map(|(o, s)| {
            let mean = |v: &[f32]| v.iter().map(|&x| x as f64).sum::<f64>() / v.len().max(1) as f64;
            let (mo, ms) = (mean(o), mean(s));
            let taper = |i: usize, n: usize| -> f64 {
                let edge = (0.1 * n as f64).max(1.0);
                let d = (i as f64 + 0.5).min(n as f64 - i as f64 - 0.5);
                if d >= edge { 1.0 } else { 0.5 - 0.5 * (std::f64::consts::PI * d / edge).cos() }
            };
            let window = |v: &[f32], m: f64| -> Vec<num_complex::Complex64> {
                (0..w * h).map(|i| num_complex::Complex64::new((v[i] as f64 - m) * taper(i % w, w) * taper(i / w, h), 0.0)).collect()
            };
            let (mut fo, mut fs) = (window(o, mo), window(s, ms));
            crate::spectral::fft2(&mut fo, w, h, false);
            crate::spectral::fft2(&mut fs, w, h, false);
            let (mut num, mut den, mut var, mut err) = ([0.0; KEPT_BANDS], [0.0; KEPT_BANDS], [0.0; KEPT_BANDS], [0.0; KEPT_BANDS]);
            for i in 0..w * h {
                let b = radial_band(i % w, i / w, w, h);
                num[b] += (fs[i] * fo[i].conj()).re;
                den[b] += fo[i].norm_sqr();
                var[b] += fs[i].norm_sqr() * fo[i].norm_sqr();
                err[b] += (fs[i] - fo[i]).norm_sqr();
            }
            std::array::from_fn(|b| {
                if b == 0 {
                    return if mo.abs() > 1e-9 { (ms / mo).clamp(0.0, 1.0) } else { 1.0 };
                }
                if den[b] <= 0.0 {
                    return 0.0;
                }
                // Exact agreement does not happen by chance.
                if err[b] <= 1e-8 * den[b] {
                    return 1.0;
                }
                let g = num[b] / den[b];
                let margin = 2.0 * var[b].sqrt() / den[b];
                if margin >= 1.0 {
                    return 0.0;
                }
                ((g - margin) / (1.0 - margin)).clamp(0.0, 1.0)
            })
        })
        .collect()
}

/// The part of `planes` that a scramble with this transfer keeps.
fn kept_part(planes: &Planes, transfer: &[[f64; KEPT_BANDS]]) -> Planes {
    let (w, h) = (planes.width, planes.height);
    let color = planes
        .color
        .iter()
        .zip(transfer)
        .map(|(p, gain)| {
            if gain.iter().all(|&g| g == 0.0) {
                return vec![0.0; w * h];
            }
            let mut f = crate::spectral::to_complex(p);
            crate::spectral::fft2(&mut f, w, h, false);
            f.par_iter_mut().enumerate().for_each(|(i, v)| *v *= gain[radial_band(i % w, i / w, w, h)]);
            crate::spectral::fft2(&mut f, w, h, true);
            f.iter().map(|v| v.re as f32).collect()
        })
        .collect();
    Planes { color, alpha: None, ..planes.clone() }
}

/// `a + sign * b` for every plane.
fn combine(a: &Planes, b: &Planes, sign: f32) -> Planes {
    Planes {
        color: a.color.iter().zip(&b.color).map(|(x, y)| x.iter().zip(y).map(|(x, y)| x + sign * y).collect()).collect(),
        ..a.clone()
    }
}

/// Takes each cell from `x0` with probability `a`, else from `x1`, by a fixed random threshold
/// per cell (`salt` picks the thresholds). Averaging pixel noise smooths it and averaging blocks
/// ghosts them; a dissolve keeps both looking like the rest of the frame.
fn dissolve(x0: &Planes, x1: &Planes, a: &[f32], salt: u64, cell: (usize, usize)) -> Planes {
    let (w, (cw, ch)) = (x0.width, (cell.0.max(1), cell.1.max(1)));
    let cols = w.div_ceil(cw) as u64;
    let pick: Vec<bool> = (0..a.len())
        .map(|i| {
            let id = ((i / w) / ch) as u64 * cols + ((i % w) / cw) as u64;
            ((seed::derive(salt, id) >> 40) as f32 / (1u64 << 24) as f32) < a[i]
        })
        .collect();
    let choose = |p0: &Vec<f32>, p1: &Vec<f32>| -> Vec<f32> { pick.iter().zip(p0.iter().zip(p1)).map(|(&k, (x, y))| if k { *x } else { *y }).collect() };
    Planes {
        color: x0.color.iter().zip(&x1.color).map(|(p0, p1)| choose(p0, p1)).collect(),
        ..x0.clone()
    }
}

/// `a * x0 + (1 - a) * x1` without the contrast loss of mixing unrelated textures (up to
/// sqrt(2), which would mark moving areas): deviations from the mean are divided by
/// `sqrt(a² + b² + 2abρ)`, `ρ` the local luminance correlation. Agreeing layers mix plainly.
fn blend_layers(x0: &Planes, x1: &Planes, a: &[f32]) -> Planes {
    let (w, h) = (x0.width, x0.height);
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32;
    let deviation = |v: Vec<f32>| -> Vec<f32> {
        let m = mean(&v);
        v.into_iter().map(|x| x - m).collect()
    };
    let (d0, d1) = (deviation(x0.luminance()), deviation(x1.luminance()));
    let sigma = LAYER_STATISTICS_SMOOTHING * w.min(h) as f32;
    let local = |v: Vec<f32>| crate::scramble::blur::gaussian_blur(&v, w, h, sigma);
    let c01 = local(d0.iter().zip(&d1).map(|(x, y)| x * y).collect());
    let c00 = local(d0.iter().map(|x| x * x).collect());
    let c11 = local(d1.iter().map(|x| x * x).collect());
    let scale: Vec<f32> = (0..w * h)
        .map(|i| {
            let rho = (c01[i] / (c00[i] * c11[i]).sqrt().max(1e-12)).clamp(0.0, 1.0);
            let (a, b) = (a[i], 1.0 - a[i]);
            1.0 / (a * a + b * b + 2.0 * a * b * rho).sqrt()
        })
        .collect();
    let blend = |p0: &Vec<f32>, p1: &Vec<f32>| -> Vec<f32> {
        let (m0, m1) = (mean(p0), mean(p1));
        (0..w * h)
            .map(|i| {
                let (a, b) = (a[i], 1.0 - a[i]);
                a * m0 + b * m1 + (a * (p0[i] - m0) + b * (p1[i] - m1)) * scale[i]
            })
            .collect()
    };
    Planes {
        color: x0.color.iter().zip(&x1.color).map(|(p0, p1)| blend(p0, p1)).collect(),
        ..x0.clone()
    }
}

/// `a * (1 - t) + b * t` per pixel, for every plane.
fn mix(a: &Planes, b: &Planes, t: &[f32]) -> Planes {
    let blend = |pa: &Vec<f32>, pb: &Vec<f32>| -> Vec<f32> {
        pa.iter().zip(pb).zip(t).map(|((x, y), t)| x * (1.0 - t) + y * t).collect()
    };
    Planes {
        color: a.color.iter().zip(&b.color).map(|(x, y)| blend(x, y)).collect(),
        alpha: match (&a.alpha, &b.alpha) {
            (Some(x), Some(y)) => Some(blend(x, y)),
            _ => a.alpha.clone(),
        },
        ..a.clone()
    }
}

/// Face mask for one frame: the detected faces, or the previous mask for up to
/// `FACE_HOLD_FRAMES` frames when none are found (moved along `flow` when given).
fn resolve_mask(detected: Option<FaceMask>, flow: Option<&optical_flow::FlowField>, planes: &Planes, state: &mut State) -> FaceMask {
    if let Some(mask) = detected {
        state.frames_since_face = 0;
        return mask;
    }
    state.frames_without_faces += 1;
    state.frames_since_face += 1;
    match &state.prev_mask {
        Some(prev) if state.frames_since_face <= FACE_HOLD_FRAMES => match flow {
            Some(flow) => {
                let p = Planes { color: vec![prev.weights.clone()], alpha: None, ..planes.clone() };
                FaceMask { weights: warp(&p, flow).color.remove(0), boxes: prev.boxes.iter().map(|b| move_box(b, flow)).collect() }
            }
            None => prev.clone(),
        },
        _ => FaceMask { weights: vec![0.0; planes.len()], boxes: Vec::new() },
    }
}

/// A box moved with the content at its centre. The flow is backward (see `warp`), so content
/// at `c` moves to about `c - flow(c)`.
fn move_box(b: &FaceBox, flow: &optical_flow::FlowField) -> FaceBox {
    let (w, h) = (flow.width, flow.height);
    let (cx, cy) = ((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
    let i = (cy.clamp(0.0, (h - 1) as f32) as usize) * w + cx.clamp(0.0, (w - 1) as f32) as usize;
    let (dx, dy) = (flow.data[i], flow.data[w * h + i]);
    FaceBox { x0: b.x0 - dx, y0: b.y0 - dy, x1: b.x1 - dx, y1: b.y1 - dy }
}

/// Scrambled frame inside the face mask, original (or black) outside it.
fn composite(original: &Planes, scrambled: Planes, mask: Option<&FaceMask>, scramble: &ScrambleOptions) -> Planes {
    match (mask, &scramble.face_detection) {
        (Some(mask), Some(face_opts)) => {
            let background = match face_opts.background_mode {
                BackgroundMode::Include => original.clone(),
                BackgroundMode::Exclude => Planes {
                    color: original.color.iter().map(|p| vec![0.0; p.len()]).collect(),
                    ..original.clone()
                },
            };
            mix(&background, &scrambled, &mask.weights)
        }
        _ => scrambled,
    }
}

/// The aperture is applied to what is written, not to the state carried to the next frame.
fn with_aperture(mut planes: Planes, scramble: &ScrambleOptions, mask: Option<&FaceMask>) -> Planes {
    if let Some(aperture) = &scramble.aperture {
        crate::scramble::aperture::apply(&mut planes, aperture, mask.map(|m| m.boxes.as_slice()));
    }
    planes
}

fn frame_seed(options: &VideoProcessingOptions, base_seed: u64, index: usize) -> u64 {
    match options.frame_seeds {
        FrameSeeds::Fixed => base_seed,
        FrameSeeds::PerFrame => seed::derive(base_seed, index as u64),
    }
}

/// Frames that do not depend on each other: detection and scrambling run in parallel over the
/// batch; only the face-mask hold-over runs in frame order.
fn process_independent(frames: &mut [YuvFrame], options: &VideoProcessingOptions, scramble: &ScrambleOptions, base_seed: u64, state: &mut State) -> Result<()> {
    let work = prepare(frames, scramble, state)?;
    let target = state.set_target.clone();

    let counts: Vec<(u64, u64)> = frames
        .par_iter_mut()
        .zip(work)
        .map(|(frame, (planes, mask))| -> Result<(u64, u64)> {
            let scrambled = fresh_scramble(&planes, scramble, frame_seed(options, base_seed, frame.index), target.as_deref())?;
            let output = with_aperture(composite(&planes, scrambled, mask.as_ref(), scramble), scramble, mask.as_ref());
            planes_to_yuv(&output, frame);
            Ok(out_of_range(&output))
        })
        .collect::<Result<_>>()?;
    state.count(counts);
    Ok(())
}

/// Converts frames to RGB and finds face masks: detection in parallel, the hold-over of masks
/// for frames without a detected face in frame order.
fn prepare(frames: &[YuvFrame], scramble: &ScrambleOptions, state: &mut State) -> Result<Vec<(Planes, Option<FaceMask>)>> {
    let prepared: Vec<(Planes, Option<FaceMask>)> = frames
        .par_iter()
        .map(|frame| {
            let planes = yuv_to_planes(frame);
            let detected = if scramble.face_detection.is_some() { detect_mask(&to_rgba8(&planes), scramble)? } else { None };
            Ok((planes, detected))
        })
        .collect::<Result<_>>()?;

    let mut work = Vec::with_capacity(prepared.len());
    for (planes, detected) in prepared {
        let mask = scramble.face_detection.is_some().then(|| resolve_mask(detected, None, &planes, state));
        state.prev_mask = mask.clone();
        state.frame += 1;
        work.push((planes, mask));
    }
    Ok(work)
}

/// Equal-power crossfade from `from` to `to` at position `u` (0..1): the deviations from each
/// frame's channel means are mixed with weights cos and sin, so that the contrast of two
/// independent scrambles stays constant, and the means are mixed linearly.
fn crossfade(from: &Planes, to: &Planes, u: f32) -> Planes {
    let (a, b) = ((u * std::f32::consts::FRAC_PI_2).cos(), (u * std::f32::consts::FRAC_PI_2).sin());
    let mean = |p: &Vec<f32>| p.iter().sum::<f32>() / p.len().max(1) as f32;
    Planes {
        color: from
            .color
            .iter()
            .zip(&to.color)
            .map(|(f, t)| {
                let (mf, mt) = (mean(f), mean(t));
                let m = (1.0 - u) * mf + u * mt;
                f.iter().zip(t).map(|(x, y)| m + a * (x - mf) + b * (y - mt)).collect()
            })
            .collect(),
        ..to.clone()
    }
}

/// A block of frames scrambled as one spatiotemporal volume, then matched, composited and
/// windowed frame by frame. The volume also covers the lookahead frames of the next block;
/// their scramble is kept, and where the next volume starts it is crossfaded into the new one.
fn process_spatiotemporal(frames: &mut [YuvFrame], st: &spatiotemporal::SpatiotemporalOptions, scramble: &ScrambleOptions, base_seed: u64, state: &mut State) -> Result<()> {
    let output = frames.iter().position(|f| f.lookahead).unwrap_or(frames.len());
    let (originals, masks): (Vec<Planes>, Vec<Option<FaceMask>>) = prepare(&frames[..output], scramble, state)?.into_iter().unzip();
    let lookahead: Vec<Planes> = frames[output..].par_iter().map(yuv_to_planes).collect();
    let linear = scramble.linear_light;
    let to_space = |p: &Planes| if linear { p.to_linear() } else { p.clone() };
    let volume: Vec<Planes> = originals.iter().map(to_space).chain(lookahead.iter().map(to_space)).collect();
    let mut scrambled = spatiotemporal::scramble(&volume, st, seed::derive(base_seed, state.chunks as u64));
    state.chunks += 1;

    // Crossfade the start of this volume from the previous volume's scramble of the same frames.
    let overlap = std::mem::take(&mut state.prev_tail);
    let span = overlap.len().max(1) as f32;
    for (k, (index, previous)) in overlap.iter().enumerate() {
        if let Some(pos) = frames[..output].iter().position(|f| f.index == *index) {
            scrambled[pos] = crossfade(previous, &scrambled[pos], (k as f32 + 0.5) / span);
        }
    }
    let tail = scrambled.split_off(output);
    let target = state.set_target.clone();
    state.prev_tail = frames[output..].iter().map(|f| f.index).zip(tail).collect();

    let counts: Vec<(u64, u64)> = frames[..output]
        .par_iter_mut()
        .zip(originals.into_par_iter().zip(masks))
        .zip(scrambled)
        .map(|((frame, (planes, mask)), mut result)| -> Result<(u64, u64)> {
            postprocess(&mut result, &to_space(&planes), scramble, target.as_deref())?;
            if linear {
                result = result.to_encoded();
            }
            let output = with_aperture(composite(&planes, as_rgb(result), mask.as_ref(), scramble), scramble, mask.as_ref());
            planes_to_yuv(&output, frame);
            Ok(out_of_range(&output))
        })
        .collect::<Result<_>>()?;
    state.count(counts);
    Ok(())
}

/// Displacement (x and y planes) of the warp a diffeomorphic scramble applies with `seed`.
fn displacement(w: usize, h: usize, o: &crate::scramble::DiffeomorphicOptions, seed: u64) -> Planes {
    let (wx, wy) = crate::scramble::diffeomorphic::warp_field(w, h, o, &mut seed::rng(seed));
    let mut field = identity_coords(w, h);
    for (p, absolute) in field.color.iter_mut().zip([wx, wy]) {
        p.iter_mut().zip(absolute).for_each(|(own, to)| *own = to - *own);
    }
    field
}

/// Weighted mean of displacement fields. Fields pointing different ways would weaken each
/// other, so the local RMS is restored; the result is smoothed so the warp bends rather than
/// tears where neighbours move differently.
fn blend_fields(fields: &[(&Planes, Vec<f32>)], sigma: f32) -> Planes {
    let (w, h) = (fields[0].0.width, fields[0].0.height);
    let n = w * h;
    let total: Vec<f32> = (0..n).map(|i| fields.iter().map(|(_, a)| a[i]).sum::<f32>().max(1e-6)).collect();
    let mut mean = vec![vec![0.0f32; n]; 2];
    let mut energy = vec![0.0f32; n];
    for (field, a) in fields {
        for i in 0..n {
            let share = a[i] / total[i];
            let (dx, dy) = (field.color[0][i], field.color[1][i]);
            mean[0][i] += share * dx;
            mean[1][i] += share * dy;
            energy[i] += share * (dx * dx + dy * dy);
        }
    }
    let blur = |v: &[f32]| crate::scramble::blur::gaussian_blur(v, w, h, sigma);
    let wanted = blur(&energy);
    let got = blur(&(0..n).map(|i| mean[0][i] * mean[0][i] + mean[1][i] * mean[1][i]).collect::<Vec<f32>>());
    let scale: Vec<f32> = wanted.iter().zip(&got).map(|(a, b)| (a / b.max(1e-12)).sqrt().clamp(1.0, 2.0)).collect();
    let color = mean.iter().map(|p| blur(&p.iter().zip(&scale).map(|(v, s)| v * s).collect::<Vec<f32>>())).collect();
    Planes { color, ..fields[0].0.clone() }
}

/// A scramble of `original` made by `scramble` (instead of with a seed), post-processed as in
/// `fresh_scramble`.
fn scramble_with(original: &Planes, options: &ScrambleOptions, set_target: Option<&Target>, scramble: impl Fn(&Planes) -> Planes) -> Result<Planes> {
    let source = if options.linear_light { original.to_linear() } else { original.clone() };
    let mut result = scramble(&source);
    postprocess(&mut result, &source, options, set_target)?;
    if options.linear_light {
        result = result.to_encoded();
    }
    Ok(as_rgb(result))
}

fn keyframe(original: &Planes, index: usize, options: &VideoProcessingOptions, scramble: &ScrambleOptions, base_seed: u64, state: &State) -> Result<Keyframe> {
    let seed = frame_seed(options, base_seed, index);
    let carried = match &scramble.scramble_type {
        ScrambleType::Diffeomorphic(o) => Carried::Warp(displacement(original.width, original.height, o, seed)),
        ScrambleType::LocalPhase(o) => Carried::Phase(PhaseOffsets::new(original.width, original.height, o, &mut seed::rng(seed))),
        _ => {
            let fresh = fresh_scramble(original, scramble, seed, state.set_target.as_deref())?;
            let transfer = kept_transfer(original, &fresh);
            let changed = combine(&fresh, &kept_part(original, &transfer), -1.0);
            Carried::Texture { changed, transfer }
        }
    };
    Ok(Keyframe { index, carried })
}

/// Follow motion for one interval: the batch's frames plus the next batch's first frame as
/// lookahead (none at the end of the video). See docs/methods.md.
///
/// Bidirectional advection (Bousseau et al. 2007): a forward layer from the interval's first
/// frame and a reverse layer from the lookahead, weighted `cos²` / `sin²`. Uncovered background
/// has no past but has a future, which is why I carry both. Layers store positions and sample
/// their scramble once, so nothing blurs over time. Distorted or untrusted layers (see
/// `round_trip`) give way to the other one, and to the frame's own scramble where both fail.
///
/// Only what the scramble changed is carried; what it keeps comes from the current frame.
/// Diffeomorphic and local phase transform the current frame, so for them I carry the warp or
/// the phase offsets instead of the result.
fn process_following(frames: &mut [YuvFrame], options: &VideoProcessingOptions, scramble: &ScrambleOptions, base_seed: u64, session: &Mutex<onnx::Session>, state: &mut State) -> Result<()> {
    let tc = options.temporal_coherence.as_ref().expect("follow-motion processing needs temporal coherence options");
    let output = frames.iter().position(|f| f.lookahead).unwrap_or(frames.len());
    if output == 0 {
        return Ok(());
    }
    let end = if output < frames.len() { output } else { output - 1 };
    let originals: Vec<Planes> = frames[..=end].par_iter().map(yuv_to_planes).collect();
    let images: Vec<RgbaImage> = originals.par_iter().map(to_rgba8).collect();
    let (w, h) = (originals[0].width, originals[0].height);
    let sigma = FLOW_SMOOTHING * w.min(h) as f32;

    let opening = match state.keyframe.take() {
        Some(k) if k.index == frames[0].index => k,
        _ => keyframe(&originals[0], frames[0].index, options, scramble, base_seed, state)?,
    };
    let closing = if end > 0 { Some(keyframe(&originals[end], frames[end].index, options, scramble, base_seed, state)?) } else { None };

    // Reverse layer for every frame, plus the backward flows the forward pass will need.
    let mut reverse = vec![(identity_coords(w, h), vec![1.0; w * h])];
    let mut backward: Vec<Option<(Vec<f32>, optical_flow::FlowField)>> = (0..=end).map(|_| None).collect();
    for t in (0..end).rev() {
        let there = optical_flow::compute_optical_flow(&images[t], &images[t + 1], session)?;
        let back = optical_flow::compute_optical_flow(&images[t + 1], &images[t], session)?;
        let reliable = round_trip(&there, &back);
        let (coords, trust) = reverse.last().expect("starts with one layer");
        let flow = smooth_flow(&there, sigma);
        let trust = carry(trust, &flow).iter().zip(&reliable).map(|(a, b)| a * b).collect();
        reverse.push((warp(coords, &flow), trust));
        backward[t + 1] = Some((round_trip(&back, &there), back));
    }
    reverse.reverse();

    let fit = |coords: &Planes, trust: &[f32], weight: f32| -> Vec<f32> {
        let d = crate::scramble::blur::gaussian_blur(&distortion(coords), w, h, sigma);
        let trust = crate::scramble::blur::gaussian_blur(trust, w, h, sigma);
        d.iter().zip(&trust).map(|(d, r)| weight * r / (1.0 + ((d - 1.0) / (HALF_WEIGHT_DISTORTION - 1.0)).powi(2))).collect()
    };
    // Pixel and block scrambles move and mix as whole cells (see `remap`, `dissolve`).
    let cell = match &scramble.scramble_type {
        ScrambleType::Pixel => Some((1, 1)),
        ScrambleType::Block(o) => Some((o.block_size.0 as usize, o.block_size.1 as usize)),
        _ => None,
    };
    let combine_layers = |x0: &Planes, x1: &Planes, a: &[f32], salt: u64| match cell { Some(c) => dissolve(x0, x1, a, salt, c), None => blend_layers(x0, x1, a) };
    let mut forward = identity_coords(w, h);
    let mut forward_trust = vec![1.0; w * h];
    for t in 0..output {
        let index = frames[t].index;
        let original = &originals[t];
        let needs_flow = t > 0 || tc.export_flow || scramble.face_detection.is_some();
        let first;
        let flow = match (t, &state.prev_original) {
            (0, Some(prev)) if needs_flow => {
                first = optical_flow::compute_optical_flow(&images[0], &to_rgba8(prev), session)?;
                Some(&first)
            }
            (0, _) => None,
            _ => backward[t].as_ref().map(|(_, back)| back),
        };
        if let (Some(flow), true, Some(dir)) = (flow, tc.export_flow, &tc.flow_output_dir) {
            std::fs::create_dir_all(dir)?;
            optical_flow::export_flo_file(flow, &dir.join(format!("frame_{index:06}.flo")))?;
        }
        let mask = if scramble.face_detection.is_some() {
            let detected = detect_mask(&images[t], scramble)?;
            Some(resolve_mask(detected, flow, original, state))
        } else {
            None
        };
        if let Some((reliable, back)) = &backward[t] {
            let flow = smooth_flow(back, sigma);
            forward = warp(&forward, &flow);
            forward_trust = carry(&forward_trust, &flow).iter().zip(reliable).map(|(a, b)| a * b).collect();
        }

        let s = (std::f32::consts::FRAC_PI_2 * t as f32 / end.max(1) as f32).cos().powi(2);
        let f0 = fit(&forward, &forward_trust, s);
        let f1 = match &closing {
            Some(_) => fit(&reverse[t].0, &reverse[t].1, 1.0 - s),
            None => vec![0.0; w * h],
        };
        let seed = frame_seed(options, base_seed, index);
        let target = state.set_target.as_deref();
        let still = vec![STILL_WEIGHT; w * h];
        let scrambled = match &scramble.scramble_type {
            ScrambleType::Diffeomorphic(o) => {
                // The warp moves with the content and is applied to the current frame.
                let own = match (&closing, t) {
                    (_, 0) => opening.warp().clone(),
                    (Some(k), t) if t == end => k.warp().clone(),
                    _ => displacement(w, h, o, seed),
                };
                let carried_forward = remap(opening.warp(), &forward, None);
                let carried_back = closing.as_ref().map(|k| remap(k.warp(), &reverse[t].0, None));
                let mut fields = vec![(&carried_forward, f0.clone()), (&own, still)];
                if let Some(back) = &carried_back {
                    fields.push((back, f1.clone()));
                }
                let field = blend_fields(&fields, sigma);
                let (wx, wy): (Vec<f32>, Vec<f32>) = (0..w * h).map(|i| ((i % w) as f32 + field.color[0][i], (i / w) as f32 + field.color[1][i])).unzip();
                scramble_with(original, scramble, target, |p| crate::scramble::diffeomorphic::apply(p, &wx, &wy))?
            }
            ScrambleType::LocalPhase(o) => {
                // The phase offsets move with the content and scramble the current frame.
                let own = match (&closing, t) {
                    (_, 0) => opening.phase().clone(),
                    (Some(k), t) if t == end => k.phase().clone(),
                    _ => PhaseOffsets::new(w, h, o, &mut seed::rng(seed)),
                };
                let carried_forward = opening.phase().carry(&forward);
                let carried_back = closing.as_ref().map(|k| k.phase().carry(&reverse[t].0));
                let mut fields = vec![(&carried_forward, f0.as_slice()), (&own, still.as_slice())];
                if let Some(back) = &carried_back {
                    fields.push((back, f1.as_slice()));
                }
                let offsets = PhaseOffsets::blend(&fields);
                scramble_with(original, scramble, target, |p| crate::scramble::local_phase::scramble_with(p, o, &offsets))?
            }
            _ => {
                let (opening_changed, opening_transfer) = opening.texture();
                let (kept, changed) = match (&closing, t) {
                    (_, 0) => (kept_part(original, opening_transfer), opening_changed.clone()),
                    (Some(k), t) if t == end => (kept_part(original, k.texture().1), k.texture().0.clone()),
                    _ => {
                        let kept = kept_part(original, opening_transfer);
                        let fresh = fresh_scramble(original, scramble, seed, target)?;
                        let changed = combine(&fresh, &kept, -1.0);
                        (kept, changed)
                    }
                };
                let layers = match &closing {
                    Some(k) => {
                        let a: Vec<f32> = f0.iter().zip(&f1).map(|(x, y)| x / (x + y).max(1e-6)).collect();
                        combine_layers(&remap(opening_changed, &forward, cell), &remap(k.texture().0, &reverse[t].0, cell), &a, 1)
                    }
                    None => remap(opening_changed, &forward, cell),
                };
                let moving: Vec<f32> = f0.iter().zip(&f1).map(|(x, y)| (x + y) / (x + y + STILL_WEIGHT)).collect();
                combine(&kept, &combine_layers(&layers, &changed, &moving, 2), 1.0)
            }
        };

        let output = composite(original, scrambled, mask.as_ref(), scramble);
        let written = with_aperture(output, scramble, mask.as_ref());
        planes_to_yuv(&written, &mut frames[t]);
        state.count([out_of_range(&written)]);
        state.prev_mask = mask;
        state.frame += 1;
    }
    state.prev_original = Some(originals[output - 1].clone());
    state.keyframe = closing;
    Ok(())
}

/// Loads the optical flow model and returns where it runs, e.g. "CUDA" or "CPU (CUDA not available)".
pub fn optical_flow_device() -> Result<String> {
    optical_flow::load_optical_flow_model(None)?;
    Ok(onnx::device().unwrap_or_else(|| "CPU".to_string()))
}

/// Scrambles every frame of a video.
///
/// Frames are converted from YUV to float RGB with the video's own matrix and range, so an
/// unchanged frame is written back unchanged. Each frame is scrambled whole with its seed
/// (`frame_seeds`); with face detection, the result shows only inside that frame's faces.
/// Independent frames run in parallel batches; with `temporal_coherence` the scramble follows
/// the optical flow (see `process_following`). The settings and seed go to `<output>.json`.
pub fn process_video(options: &VideoProcessingOptions, progress_callback: impl Fn(f32) + Send + Sync + 'static) -> Result<VideoReport> {
    process_video_with(options, None, progress_callback)
}

/// Like `process_video`, with the target used when frames are matched to a set average.
pub fn process_video_with(
    options: &VideoProcessingOptions,
    set_target: Option<Target>,
    progress_callback: impl Fn(f32) + Send + Sync + 'static,
) -> Result<VideoReport> {
    if matches!(options.scramble_options.matching, Some(m) if m.target == crate::matching::MatchTarget::SetAverage) && set_target.is_none() {
        anyhow::bail!("Matching to a set average needs a reference set");
    }
    let processor = VideoProcessor::new()?;
    let base_seed = seed::resolve(options.scramble_options.seed);
    let scramble = ScrambleOptions { seed: Some(base_seed), ..options.scramble_options.clone() };

    let flow_session = if options.temporal_coherence.is_some() {
        info!("Following motion; loading the optical flow model");
        Some(optical_flow::load_optical_flow_model(None)?)
    } else {
        None
    };
    // Peak bytes per pixel and frame. Independent frames: about 160 (a Fourier scramble).
    // Spatiotemporal: YUV 3 + RGB in and out 24 + complex volume and copy 16 + phases 4, plus
    // codec buffers. Follow motion keeps a whole interval: RGB, image, reverse layer and flow.
    let batching = if let Some(tc) = &options.temporal_coherence {
        Batching { max_frames: tc.keyframe_interval.max(2), bytes_per_pixel: 64, memory_budget: 2 << 30, balance: false, overlap: 1 }
    } else if options.spatiotemporal.is_some() {
        let gb = options.memory_budget_gb.unwrap_or(4.0).clamp(0.5, 1024.0) as f64;
        Batching { max_frames: usize::MAX, bytes_per_pixel: 80, memory_budget: (gb * (1u64 << 30) as f64) as usize, balance: true, overlap: SPATIOTEMPORAL_OVERLAP }
    } else {
        Batching { max_frames: rayon::current_num_threads().max(1) * 2, bytes_per_pixel: 160, memory_budget: 2 << 30, balance: false, overlap: 0 }
    };

    let state = Arc::new(Mutex::new(State {
        frame: 0,
        prev_original: None,
        keyframe: None,
        prev_mask: None,
        frames_since_face: 0,
        frames_without_faces: 0,
        chunks: 0,
        prev_tail: Vec::new(),
        set_target: set_target.map(Arc::new),
        samples: 0,
        clipped: 0,
    }));
    let frame_state = state.clone();
    let frame_options = options.clone();

    let output_bit_depth = processor.process_video(
        &options.input_path,
        &options.output_path,
        options.fidelity,
        batching,
        options.color_standard.map(ColorMatrix::from),
        move |frames: &mut [YuvFrame]| {
            let mut state = frame_state.lock().map_err(|e| anyhow::anyhow!("State mutex poisoned: {e}"))?;
            match (flow_session, &frame_options.spatiotemporal) {
                (Some(session), _) => process_following(frames, &frame_options, &scramble, base_seed, session, &mut state),
                (None, Some(st)) => process_spatiotemporal(frames, st, &scramble, base_seed, &mut state),
                (None, None) => process_independent(frames, &frame_options, &scramble, base_seed, &mut state),
            }
        },
        progress_callback,
    )?;

    let (frames, frames_without_faces, chunks, clipped_fraction) = {
        let state = state.lock().map_err(|e| anyhow::anyhow!("State mutex poisoned: {e}"))?;
        (state.frame, state.frames_without_faces, state.chunks, state.clipped as f64 / state.samples.max(1) as f64)
    };
    let report = VideoReport {
        seed: base_seed,
        frames,
        frames_without_faces: options.scramble_options.face_detection.as_ref().map(|_| frames_without_faces),
        chunks: options.spatiotemporal.is_some().then_some(chunks),
        output_bit_depth: Some(output_bit_depth),
        clipped_fraction,
    };
    let settings = VideoRunSettings {
        scramblery_version: crate::VERSION.to_string(),
        input: options.input_path.clone(),
        scramble_options: ScrambleOptions { seed: Some(base_seed), ..options.scramble_options.clone() },
        frame_seeds: options.frame_seeds,
        temporal_coherence: options.temporal_coherence.clone(),
        spatiotemporal: options.spatiotemporal,
        fidelity: options.fidelity,
        report: Some(report.clone()),
    };
    crate::io::write_json(&settings, &options.output_path.with_extension("json"))?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fidelity_serialises_as_the_strings_the_ui_sends() {
        for (value, expected) in [
            (VideoFidelity::Lossless, "\"Lossless\""),
            (VideoFidelity::HighQuality, "\"HighQuality\""),
            (VideoFidelity::Fast, "\"Fast\""),
        ] {
            assert_eq!(serde_json::to_string(&value).unwrap(), expected);
            assert_eq!(
                serde_json::from_str::<VideoFidelity>(expected).unwrap(),
                value
            );
        }
    }
    #[test]
    fn omitted_fidelity_defaults_to_fast() {
        let json = r#"{
            "input_path": "in.mp4",
            "output_path": "out.mp4",
            "scramble_options": {
                "scramble_type": "Pixel",
                "intensity": 1.0,
                "seed": null,
                "face_detection": null
            },
            "temporal_coherence": null
        }"#;
        let opts: VideoProcessingOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.fidelity, VideoFidelity::Fast);
        assert_eq!(opts.frame_seeds, FrameSeeds::Fixed);
    }

    fn planes(w: usize, h: usize, f: impl Fn(usize, usize) -> f32) -> Planes {
        let plane: Vec<f32> = (0..w * h).map(|i| f(i % w, i / w)).collect();
        Planes { width: w, height: h, color: vec![plane.clone(), plane.clone(), plane], alpha: None, sixteen_bit: false }
    }

    fn uniform_flow(w: usize, h: usize, dx: f32, dy: f32) -> optical_flow::FlowField {
        let mut data = vec![dx; w * h];
        data.extend(vec![dy; w * h]);
        optical_flow::FlowField { width: w, height: h, data }
    }

    #[test]
    fn yuv_round_trip_is_exact() {
        for (matrix, full_range, max_value) in [
            (ColorMatrix::Bt709, false, 255u16),
            (ColorMatrix::Bt601, false, 255),
            (ColorMatrix::Bt2020, true, 255),
            (ColorMatrix::Bt709, false, 1023),
            (ColorMatrix::Bt2020, true, 4095),
            (ColorMatrix::Bt709, false, 65535),
        ] {
            let n = 4096;
            let code = |i: usize, k: usize| ((i * k * 251 + 17) % (max_value as usize + 1)) as u16;
            let original = YuvFrame {
                index: 0,
                width: 64,
                height: 64,
                y: (0..n).map(|i| code(i, 7)).collect(),
                u: (0..n).map(|i| code(i, 13)).collect(),
                v: (0..n).map(|i| code(i, 29)).collect(),
                max_value,
                matrix,
                full_range,
                lookahead: false,
            };
            let mut copy = original.clone();
            planes_to_yuv(&yuv_to_planes(&original), &mut copy);
            assert_eq!((copy.y, copy.u, copy.v), (original.y, original.u, original.v), "{matrix:?}");
        }
    }

    #[test]
    fn equal_power_crossfade_keeps_contrast_of_independent_noise() {
        let noise = |seed: u32| -> Planes {
            let mut x = seed;
            let plane: Vec<f32> = (0..20000)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 17;
                    x ^= x << 5;
                    0.5 + (x as f32 / u32::MAX as f32 - 0.5) * 0.4
                })
                .collect();
            Planes { width: 200, height: 100, color: vec![plane], alpha: None, sixteen_bit: false }
        };
        let (a, b) = (noise(1), noise(7));
        let sd = |p: &Planes| crate::analysis::mean_sd(&p.color[0]).1;
        let mid = crossfade(&a, &b, 0.5);
        assert!((sd(&mid) - sd(&a)).abs() / sd(&a) < 0.03);
        assert!((crate::analysis::mean_sd(&mid.color[0]).0 - 0.5).abs() < 0.01);
        assert!(crossfade(&a, &b, 0.0).color[0].iter().zip(&a.color[0]).all(|(x, y)| (x - y).abs() < 1e-5));
    }

    #[test]
    fn warping_follows_the_flow() {
        let p = planes(16, 8, |x, _| x as f32 / 16.0);
        let warped = warp(&p, &uniform_flow(16, 8, 2.0, 0.0));
        // Backward flow: output pixel x samples the previous frame at x + 2.
        assert!((warped.color[0][3] - p.color[0][5]).abs() < 1e-6);
    }

    #[test]
    fn dissolve_takes_each_pixel_from_one_layer_in_proportion_to_its_weight() {
        let (a, b) = (planes(64, 64, |_, _| 1.0), planes(64, 64, |_, _| 0.0));
        let mixed = dissolve(&a, &b, &[0.25; 64 * 64], 1, (1, 1));
        assert!(mixed.color[0].iter().all(|&v| v == 0.0 || v == 1.0));
        let share = mixed.color[0].iter().sum::<f32>() / (64.0 * 64.0);
        assert!((share - 0.25).abs() < 0.03, "share {share}");
    }

    #[test]
    fn moved_cells_stay_intact_blocks_on_the_grid() {
        // Blocks of 4 x 4 with distinct values, carried by a rotation about the centre.
        let reference = planes(32, 32, |x, y| ((x / 4) * 8 + y / 4) as f32);
        let (c, a) = (16.0f32, 0.3f32);
        let mut coords = identity_coords(32, 32);
        for i in 0..32 * 32 {
            let (x, y) = ((i % 32) as f32 - c, (i / 32) as f32 - c);
            coords.color[0][i] = c + a.cos() * x - a.sin() * y;
            coords.color[1][i] = c + a.sin() * x + a.cos() * y;
        }
        let moved = remap(&reference, &coords, Some((4, 4)));
        for i in 0..32 * 32 {
            let (x, y) = (i % 32, i / 32);
            let corner = (y - y % 4) * 32 + (x - x % 4);
            assert_eq!(moved.color[0][i], moved.color[0][corner], "cell at ({x}, {y}) is not one block");
        }
    }

    #[test]
    fn blended_warps_keep_their_strength() {
        let field = |dx: f32, dy: f32| Planes { color: vec![vec![dx; 32 * 32], vec![dy; 32 * 32]], ..identity_coords(32, 32) };
        let (a, b) = (field(3.0, 0.0), field(0.0, 3.0));
        let same = blend_fields(&[(&a, vec![0.5; 32 * 32]), (&a, vec![0.5; 32 * 32])], 1.0);
        assert!(same.color[0].iter().all(|v| (v - 3.0).abs() < 1e-4) && same.color[1].iter().all(|v| v.abs() < 1e-4));
        let mixed = blend_fields(&[(&a, vec![0.5; 32 * 32]), (&b, vec![0.5; 32 * 32])], 1.0);
        let length = mixed.color[0][500].hypot(mixed.color[1][500]);
        assert!((length - 3.0).abs() < 1e-3, "length {length}");
    }

    #[test]
    fn round_trip_marks_flow_that_the_flow_back_does_not_undo() {
        let (there, back) = (uniform_flow(32, 16, 3.0, 1.0), uniform_flow(32, 16, -3.0, -1.0));
        assert!(round_trip(&there, &back).iter().all(|&r| r == 1.0));
        let still = uniform_flow(32, 16, 0.0, 0.0);
        assert!(round_trip(&there, &still).iter().all(|&r| r == 0.0));
    }

    #[test]
    fn carried_coordinates_move_the_texture_without_blurring_it() {
        let reference = planes(32, 8, |x, y| ((x * 7 + y * 13) % 5) as f32 / 4.0);
        let flow = uniform_flow(32, 8, 0.5, 0.0);
        let mut coords = identity_coords(32, 8);
        for _ in 0..8 {
            coords = warp(&coords, &flow);
        }
        // Eight half-pixel steps move the texture by exactly 4 pixels.
        let moved = remap(&reference, &coords, None);
        for x in 0..20 {
            assert!((moved.color[0][x] - reference.color[0][x + 4]).abs() < 1e-5);
        }
    }

    #[test]
    fn kept_part_separates_what_a_scramble_leaves_unchanged() {
        // A scramble that keeps everything keeps the whole image.
        let noise = planes(128, 128, |x, y| (((x as u32 * 73 + y as u32 * 151) ^ 99).wrapping_mul(2654435761) >> 24) as f32 / 255.0);
        let all = kept_part(&noise, &kept_transfer(&noise, &noise));
        assert!(all.color[0].iter().zip(&noise.color[0]).all(|(a, b)| (a - b).abs() < 1e-3));
        // Blur keeps low frequencies and removes high ones.
        let transfer = kept_transfer(&noise, &crate::scramble::blur::scramble(&noise, 2.0));
        assert!(transfer[0][4] > 0.5 && transfer[0][20] < 0.2 && transfer[0][40] < 0.05, "{:?}", transfer[0]);
        // A full phase scramble keeps the mean and nothing else, also at the lowest frequencies.
        let blobs = planes(96, 64, |x, y| (((x as f32 / 9.0).sin() * (y as f32 / 7.0).cos()) + 1.0) / 2.0);
        let options = crate::scramble::FourierOptions::default();
        for seed in 0..8 {
            let scrambled = crate::scramble::fourier::scramble(&blobs, &options, &mut seed::rng(seed));
            let transfer = kept_transfer(&blobs, &scrambled);
            assert!(transfer[0][0] > 0.99, "seed {seed}: mean {}", transfer[0][0]);
            assert!(transfer[0][1..].iter().all(|&g| g < 0.05), "seed {seed}: {:?}", transfer[0]);
        }
    }

    #[test]
    fn layer_blend_keeps_contrast_and_leaves_agreeing_layers_unchanged() {
        let noise = |seed: u32| planes(64, 64, |x, y| (((x as u32 * 73 + y as u32 * 151) ^ seed).wrapping_mul(2654435761) >> 24) as f32 / 255.0);
        let (a, b) = (noise(1), noise(99));
        let sd = |p: &Planes| crate::analysis::mean_sd(&p.color[0]).1;
        let mid = blend_layers(&a, &b, &[0.5; 64 * 64]);
        assert!((sd(&mid) - sd(&a)).abs() / sd(&a) < 0.1, "{} vs {}", sd(&mid), sd(&a));
        let same = blend_layers(&a, &a, &[0.3; 64 * 64]);
        assert!(same.color[0].iter().zip(&a.color[0]).all(|(x, y)| (x - y).abs() < 1e-4));
    }
}
