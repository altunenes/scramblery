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
use crate::scramble::{postprocess, scramble_planes, BackgroundMode, ScrambleOptions, TemporalCoherenceOptions};
use crate::seed;

pub mod spatiotemporal;

/// Frames without a detected face reuse the last face mask for at most this many frames.
pub const FACE_HOLD_FRAMES: usize = 15;

/// Frames over which consecutive spatiotemporal volumes are crossfaded.
pub const SPATIOTEMPORAL_OVERLAP: usize = 8;

/// Luminance error (0..1) between a motion-compensated previous frame and the current frame
/// below which a warped pixel is fully trusted, and above which it is replaced.
const RELIABLE_ERROR: f32 = 0.03;
const UNRELIABLE_ERROR: f32 = 0.10;
/// Standard deviation of the Gaussian that smooths the follow-motion weights, as a fraction of
/// the shorter frame side.
const WEIGHT_SMOOTHING: f32 = 0.03;

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
    prev_scrambled: Option<Planes>,
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

/// Per-pixel weight (0..1) of the motion-compensated previous result: low where the warped
/// previous original disagrees with the current frame (occlusions, failed motion estimates)
/// or where the motion comes from outside the frame.
fn reliability(current: &Planes, warped_original: &Planes, flow: &optical_flow::FlowField) -> Vec<f32> {
    let (w, h) = (current.width, current.height);
    let (a, b) = (current.luminance(), warped_original.luminance());
    (0..w * h)
        .into_par_iter()
        .map(|i| {
            let x = (i % w) as f32 + flow.data[i];
            let y = (i / w) as f32 + flow.data[w * h + i];
            if x < 0.0 || y < 0.0 || x > (w - 1) as f32 || y > (h - 1) as f32 {
                return 0.0;
            }
            let err = (a[i] - b[i]).abs();
            ((UNRELIABLE_ERROR - err) / (UNRELIABLE_ERROR - RELIABLE_ERROR)).clamp(0.0, 1.0)
        })
        .collect()
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

/// Weight of a fresh scramble when crossfading around keyframes (0 = warped only).
fn keyframe_blend(tc: &TemporalCoherenceOptions, frame: usize) -> f32 {
    if tc.blend_frames == 0 || tc.keyframe_interval == 0 {
        return 0.0;
    }
    let since = frame % tc.keyframe_interval;
    let until = tc.keyframe_interval - since;
    if until <= tc.blend_frames {
        1.0 - until as f32 / (tc.blend_frames + 1) as f32
    } else if since <= tc.blend_frames {
        1.0 - since as f32 / (tc.blend_frames + 1) as f32
    } else {
        0.0
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

/// Follow-motion processing of one frame, which depends on the previous frame.
fn process_following(frame: &mut YuvFrame, options: &VideoProcessingOptions, scramble: &ScrambleOptions, base_seed: u64, session: &Mutex<onnx::Session>, state: &mut State) -> Result<()> {
    let tc = options.temporal_coherence.as_ref().expect("follow-motion processing needs temporal coherence options");
    let index = frame.index;
    let original = yuv_to_planes(frame);
    let image = to_rgba8(&original);
    let fresh = fresh_scramble(&original, scramble, frame_seed(options, base_seed, index), state.set_target.as_deref())?;

    let flow = match &state.prev_original {
        Some(prev) => Some(optical_flow::compute_optical_flow(&image, &to_rgba8(prev), session)?),
        None => None,
    };
    if let (Some(flow), true, Some(dir)) = (&flow, tc.export_flow, &tc.flow_output_dir) {
        std::fs::create_dir_all(dir)?;
        optical_flow::export_flo_file(flow, &dir.join(format!("frame_{index:06}.flo")))?;
    }

    let mask = if scramble.face_detection.is_some() {
        let detected = detect_mask(&image, scramble)?;
        Some(resolve_mask(detected, flow.as_ref(), &original, state))
    } else {
        None
    };

    // Motion-compensated previous result, trusted only where the motion estimate holds and,
    // with faces, only where the source pixel was itself scrambled.
    let keyframe = index == 0 || (tc.keyframe_interval > 0 && index.is_multiple_of(tc.keyframe_interval));
    let scrambled = match (&flow, &state.prev_scrambled, &state.prev_original) {
        (Some(flow), Some(prev_scrambled), Some(prev_original)) if !keyframe => {
            let warped = warp(prev_scrambled, flow);
            let mut weight = reliability(&original, &warp(prev_original, flow), flow);
            if let Some(prev_mask) = &state.prev_mask {
                let p = Planes { color: vec![prev_mask.weights.clone()], alpha: None, ..original.clone() };
                let source_scrambled = warp(&p, flow).color.remove(0);
                weight.iter_mut().zip(source_scrambled).for_each(|(w, s)| *w *= s);
            }
            let fresh_weight = keyframe_blend(tc, index);
            let replace: Vec<f32> = weight.iter().map(|w| 1.0 - w * (1.0 - fresh_weight)).collect();
            // The weights come from the original frames and follow their edges (eyes, mouth,
            // outlines). Smoothing them keeps those details from being drawn into the output.
            let sigma = WEIGHT_SMOOTHING * original.width.min(original.height) as f32;
            let replace = crate::scramble::blur::gaussian_blur(&replace, original.width, original.height, sigma);
            mix(&warped, &fresh, &replace)
        }
        _ => fresh,
    };

    let output = composite(&original, scrambled, mask.as_ref(), scramble);
    let written = with_aperture(output.clone(), scramble, mask.as_ref());
    planes_to_yuv(&written, frame);
    state.count([out_of_range(&written)]);
    state.prev_original = Some(original);
    state.prev_scrambled = Some(output);
    state.prev_mask = mask;
    state.frame += 1;
    Ok(())
}

/// Scrambles every frame of a video.
///
/// Frames are read as YUV and converted to RGB in floating point with the video's own matrix
/// and range, so a frame that is left unchanged is written back unchanged. Each frame is
/// scrambled as a whole with the frame's seed (`frame_seeds`); with face detection, the result
/// is shown only inside the faces detected in that frame. Independent frames are processed in
/// parallel batches. With `temporal_coherence`, frames are processed in order: between
/// keyframes, the previous result is moved along the optical flow wherever the motion estimate
/// is reliable, and a fresh scramble is used elsewhere. The settings, including the base seed,
/// are written to `<output>.json`.
/// Loads the optical flow model and returns where it runs, e.g. "CUDA" or "CPU (CUDA not available)".
pub fn optical_flow_device() -> Result<String> {
    optical_flow::load_optical_flow_model(None)?;
    Ok(onnx::device().unwrap_or_else(|| "CPU".to_string()))
}

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
    // Fourier scrambling of a frame needs about 160 bytes per pixel at its peak. A spatiotemporal
    // volume needs per pixel and frame: YUV (3), input and output RGB planes (24), one channel's
    // complex volume plus its temporal-transform copy (16) and the phase field (4), plus decoder
    // and encoder buffers.
    let batching = if flow_session.is_some() {
        Batching { max_frames: 1, bytes_per_pixel: 160, memory_budget: 2 << 30, balance: false, overlap: 0 }
    } else if options.spatiotemporal.is_some() {
        let gb = options.memory_budget_gb.unwrap_or(4.0).clamp(0.5, 1024.0) as f64;
        Batching { max_frames: usize::MAX, bytes_per_pixel: 80, memory_budget: (gb * (1u64 << 30) as f64) as usize, balance: true, overlap: SPATIOTEMPORAL_OVERLAP }
    } else {
        Batching { max_frames: rayon::current_num_threads().max(1) * 2, bytes_per_pixel: 160, memory_budget: 2 << 30, balance: false, overlap: 0 }
    };

    let state = Arc::new(Mutex::new(State {
        frame: 0,
        prev_original: None,
        prev_scrambled: None,
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
                (Some(session), _) => {
                    for frame in frames.iter_mut() {
                        process_following(frame, &frame_options, &scramble, base_seed, session, &mut state)?;
                    }
                    Ok(())
                }
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
    fn reliability_drops_where_motion_compensation_fails() {
        let current = planes(16, 8, |x, _| x as f32 / 16.0);
        let flow = uniform_flow(16, 8, 0.0, 0.0);
        assert!(reliability(&current, &current, &flow).iter().all(|&w| w == 1.0));
        let different = planes(16, 8, |x, _| 1.0 - x as f32 / 16.0);
        let w = reliability(&current, &different, &flow);
        assert_eq!(w[0], 0.0);
        // Motion pointing outside the frame is never trusted.
        let outside = reliability(&current, &current, &uniform_flow(16, 8, -3.0, 0.0));
        assert_eq!(outside[0], 0.0);
        assert_eq!(outside[10], 1.0);
    }
}
