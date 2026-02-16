use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use anyhow::Result;
use video_processor::{VideoProcessor, VideoFrameExt};
use image::{DynamicImage, RgbaImage};
use serde::{Serialize, Deserialize};
use log::info;
use crate::scramble::{ScrambleType, ScrambleOptions, TemporalCoherenceOptions, FaceDetectionOptions};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoProcessingOptions {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub scramble_options: ScrambleOptions,
    pub temporal_coherence: Option<TemporalCoherenceOptions>,
}

/// Apply scrambling to a single frame based on the scramble options.
/// When face detection is enabled, only face regions are scrambled (background stays intact).
fn scramble_frame(image: &RgbaImage, scramble_options: &ScrambleOptions) -> Result<DynamicImage> {
    let (width, height) = image.dimensions();
    let dyn_image = DynamicImage::ImageRgba8(image.clone());

    match &scramble_options.scramble_type {
        ScrambleType::Pixel => {
            crate::scramble::scramble_pixels(&dyn_image, scramble_options)
        }
        ScrambleType::Fourier(fourier_opts) => {
            let mut scrambler = crate::scramble::FourierScrambler::new(
                width as usize,
                height as usize,
                fourier_opts.clone(),
                scramble_options.seed,
            );
            if let Some(face_opts) = &scramble_options.face_detection {
                scrambler.scramble_with_face_detection(&dyn_image, face_opts)
            } else {
                scrambler.scramble(&dyn_image)
            }
        }
        ScrambleType::Block(block_opts) => {
            let mut scrambler = crate::scramble::BlockScrambler::new(
                block_opts.clone(),
                scramble_options.seed,
            );
            if let Some(face_opts) = &scramble_options.face_detection {
                scrambler.scramble_with_face_detection(&dyn_image, face_opts)
            } else {
                scrambler.scramble(&dyn_image)
            }
        }
        ScrambleType::Blur(blur_opts) => {
            let scrambler = crate::scramble::BlurScrambler::new(blur_opts.clone());
            if let Some(face_opts) = &scramble_options.face_detection {
                scrambler.scramble_with_face_detection(&dyn_image, face_opts)
            } else {
                scrambler.scramble(&dyn_image)
            }
        }
    }
}

/// Detect face regions on the given image. Returns bounding boxes.
fn detect_faces(image: &RgbaImage, face_opts: &FaceDetectionOptions) -> Result<Vec<face_detection::FaceRegion>> {
    let dyn_image = DynamicImage::ImageRgba8(image.clone());
    let session = face_detection::load_face_detector(None)?;
    face_detection::detect_face_regions(
        &dyn_image,
        session,
        face_opts.confidence_threshold,
        Some(face_opts.expansion_factor),
    )
}

/// Build a boolean mask from face regions.
fn regions_to_mask(regions: &[face_detection::FaceRegion], width: u32, height: u32) -> Vec<bool> {
    let mut mask = vec![false; (width * height) as usize];
    for region in regions {
        for y in region.y1..region.y2.min(height) {
            for x in region.x1..region.x2.min(width) {
                mask[(y * width + x) as usize] = true;
            }
        }
    }
    mask
}

/// Composite two frames using a mask.
/// Pixels where mask=true come from `foreground`, pixels where mask=false come from `background`.
fn composite_with_mask(background: &RgbaImage, foreground: &RgbaImage, mask: &[bool]) -> RgbaImage {
    let (w, h) = background.dimensions();
    let mut output = background.clone();
    for y in 0..h {
        for x in 0..w {
            if mask[(y * w + x) as usize] {
                output.put_pixel(x, y, *foreground.get_pixel(x, y));
            }
        }
    }
    output
}

/// Alpha-blend two RGBA images: output = a * (1 - alpha) + b * alpha
fn blend_frames(a: &RgbaImage, b: &RgbaImage, alpha: f32) -> RgbaImage {
    let (w, h) = a.dimensions();
    let mut output = RgbaImage::new(w, h);
    let inv_alpha = 1.0 - alpha;
    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y).0;
            let pb = b.get_pixel(x, y).0;
            let pixel = image::Rgba([
                (pa[0] as f32 * inv_alpha + pb[0] as f32 * alpha).round().clamp(0.0, 255.0) as u8,
                (pa[1] as f32 * inv_alpha + pb[1] as f32 * alpha).round().clamp(0.0, 255.0) as u8,
                (pa[2] as f32 * inv_alpha + pb[2] as f32 * alpha).round().clamp(0.0, 255.0) as u8,
                (pa[3] as f32 * inv_alpha + pb[3] as f32 * alpha).round().clamp(0.0, 255.0) as u8,
            ]);
            output.put_pixel(x, y, pixel);
        }
    }
    output
}

/// State held across frames for temporal coherence
struct FrameState {
    frame_index: usize,
    prev_original: Option<RgbaImage>,
    prev_scrambled: Option<RgbaImage>,
    /// Face mask from the last keyframe, reused for inter-frames
    /// to avoid running face detection every frame.
    keyframe_face_mask: Option<Vec<bool>>,
}

pub fn process_video(options: &VideoProcessingOptions, progress_callback: impl Fn(f32) + Send + Sync + 'static) -> Result<()> {
    let processor = VideoProcessor::new()?;
    let scramble_options = options.scramble_options.clone();
    let temporal_coherence = options.temporal_coherence.clone();

    // Load optical flow model if temporal coherence is enabled
    let flow_session = if temporal_coherence.is_some() {
        info!("Temporal coherence enabled, loading optical flow model");
        Some(optical_flow::load_optical_flow_model(None)?)
    } else {
        None
    };

    let state = Arc::new(Mutex::new(FrameState {
        frame_index: 0,
        prev_original: None,
        prev_scrambled: None,
        keyframe_face_mask: None,
    }));

    let has_face_detection = scramble_options.face_detection.is_some();

    processor.process_video(
        &options.input_path,
        &options.output_path,
        move |frame| {
            let width = frame.width() as u32;
            let height = frame.height() as u32;
            let stride = frame.plane_stride()[0] as usize;
            let data = frame.plane_data(0).unwrap();

            let mut image = RgbaImage::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let offset = y as usize * stride + x as usize * 4;
                    let pixel = image::Rgba([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]);
                    image.put_pixel(x, y, pixel);
                }
            }

            let processed = if let (Some(ref tc_opts), Some(session)) = (&temporal_coherence, flow_session) {
                let mut state = state.lock().map_err(|e| anyhow::anyhow!("State mutex poisoned: {}", e))?;
                let frame_idx = state.frame_index;

                let is_keyframe = frame_idx == 0
                    || (tc_opts.keyframe_interval > 0 && frame_idx % tc_opts.keyframe_interval == 0);

                // Compute how close we are to the next keyframe for blending
                let blend_n = tc_opts.blend_frames;
                let frames_since_keyframe = if tc_opts.keyframe_interval > 0 {
                    frame_idx % tc_opts.keyframe_interval
                } else {
                    frame_idx
                };

                let result = if is_keyframe {
                    info!("Frame {}: keyframe, scrambling fresh", frame_idx);

                    // On keyframes, detect faces and cache the mask for inter-frames
                    if has_face_detection {
                        let face_opts = scramble_options.face_detection.as_ref().unwrap();
                        let regions = detect_faces(&image, face_opts)?;
                        if regions.is_empty() {
                            info!("Frame {}: no faces detected on keyframe", frame_idx);
                            state.keyframe_face_mask = None;
                        } else {
                            info!("Frame {}: detected {} face(s), caching mask", frame_idx, regions.len());
                            state.keyframe_face_mask = Some(regions_to_mask(&regions, width, height));
                        }
                    }

                    scramble_frame(&image, &scramble_options)?.to_rgba8()
                } else {
                    // Inter-frame: compute backward flow (current→prev)
                    let prev_orig = state.prev_original.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Missing previous original frame"))?;

                    let flow = optical_flow::compute_optical_flow(&image, prev_orig, session)?;

                    if tc_opts.export_flow {
                        if let Some(ref dir) = tc_opts.flow_output_dir {
                            std::fs::create_dir_all(dir)?;
                            let flo_path = dir.join(format!("frame_{:06}.flo", frame_idx));
                            optical_flow::export_flo_file(&flow, &flo_path)?;
                        }
                    }

                    let prev_scrambled = state.prev_scrambled.as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Missing previous scrambled frame"))?;

                    let warped = optical_flow::warp_image(prev_scrambled, &flow)?;

                    // Apply face mask compositing if active
                    let warped = if let Some(ref mask) = state.keyframe_face_mask {
                        composite_with_mask(&image, &warped, mask)
                    } else {
                        warped
                    };

                    // Keyframe blending: crossfade warped→fresh near keyframe boundaries
                    if blend_n > 0 && tc_opts.keyframe_interval > 0 {
                        let frames_until_keyframe = tc_opts.keyframe_interval - frames_since_keyframe;
                        if frames_until_keyframe <= blend_n {
                            // Approaching next keyframe: blend warped with fresh scramble
                            // alpha goes from 0 (pure warped) to ~1 (mostly fresh) as we approach
                            let alpha = 1.0 - (frames_until_keyframe as f32 / (blend_n + 1) as f32);
                            info!("Frame {}: blending toward keyframe, alpha={:.2}", frame_idx, alpha);
                            let fresh = scramble_frame(&image, &scramble_options)?.to_rgba8();
                            blend_frames(&warped, &fresh, alpha)
                        } else if frames_since_keyframe > 0 && frames_since_keyframe <= blend_n {
                            // Just after keyframe: blend fresh with warped for smooth entry
                            let alpha = frames_since_keyframe as f32 / (blend_n + 1) as f32;
                            info!("Frame {}: blending after keyframe, alpha={:.2}", frame_idx, alpha);
                            let fresh = scramble_frame(&image, &scramble_options)?.to_rgba8();
                            blend_frames(&fresh, &warped, alpha)
                        } else {
                            warped
                        }
                    } else {
                        warped
                    }
                };

                state.prev_original = Some(image);
                state.prev_scrambled = Some(result.clone());
                state.frame_index += 1;

                result
            } else {
                // No temporal coherence: scramble each frame independently
                scramble_frame(&image, &scramble_options)?.to_rgba8()
            };

            let data = frame.plane_data_mut(0).unwrap();
            for y in 0..height {
                for x in 0..width {
                    let pixel = processed.get_pixel(x, y);
                    let offset = y as usize * stride + x as usize * 4;
                    data[offset] = pixel[0];
                    data[offset + 1] = pixel[1];
                    data[offset + 2] = pixel[2];
                    data[offset + 3] = pixel[3];
                }
            }

            Ok(())
        },
        move |progress| {
            progress_callback(progress);
        },
    )?;

    Ok(())
}
