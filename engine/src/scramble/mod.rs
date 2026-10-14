mod types;
pub mod aperture;
pub mod block;
pub mod blur;
pub mod diffeomorphic;
pub mod fourier;
pub mod local_phase;
pub mod mask;
pub mod pixel;

pub use types::*;

use image::DynamicImage;
use serde::{Deserialize, Serialize};

use crate::matching::{self, MatchProperty, MatchTarget, Target};
use crate::planes::Planes;
use crate::seed;

/// The result of scrambling one image.
#[derive(Debug, Clone)]
pub struct ScrambleOutput {
    pub image: DynamicImage,
    pub report: ScrambleReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScrambleReport {
    /// The seed that was used; passing it back in reproduces the output exactly.
    pub seed: u64,
    /// Fraction of colour samples that fell outside the valid range and were clipped.
    pub clipped_fraction: f64,
    /// Number of faces found, when face detection was enabled.
    pub faces_detected: Option<usize>,
    /// Factor by which contrast was scaled to fit the range (1 when nothing was changed).
    #[serde(default = "one")]
    pub contrast_scale: f64,
}

fn one() -> f64 {
    1.0
}

/// Scrambles an image. With face detection enabled, only the detected face regions are scrambled.
pub fn scramble_image(image: &DynamicImage, options: &ScrambleOptions) -> crate::Result<ScrambleOutput> {
    scramble_image_with(image, options, None)
}

/// Scrambles an image, using `set_target` when the options match to a set average.
pub fn scramble_image_with(
    image: &DynamicImage,
    options: &ScrambleOptions,
    set_target: Option<&Target>,
) -> crate::Result<ScrambleOutput> {
    let seed = seed::resolve(options.seed);
    let encoded = Planes::from_image(image);
    let planes = if options.linear_light { encoded.to_linear() } else { encoded };

    let (mut result, faces) = match &options.face_detection {
        None => (scramble_planes(&planes, options, seed), None),
        Some(face_opts) => {
            let (result, faces) = scramble_faces(image, &planes, options, face_opts, seed)?;
            (result, Some(faces))
        }
    };
    let contrast_scale = postprocess(&mut result, &planes, options, set_target)?;
    if options.linear_light {
        result = result.to_encoded();
    }
    if let Some(aperture) = &options.aperture {
        aperture::apply(&mut result, aperture, faces.as_deref());
    }
    let clipped_fraction = result.clip();

    Ok(ScrambleOutput {
        image: result.to_image(),
        report: ScrambleReport { seed, clipped_fraction, faces_detected: faces.map(|f| f.len()), contrast_scale },
    })
}

/// Applies the matching and out-of-range options to a scrambled result and returns the
/// contrast scale. `original` is the image before scrambling. Values are not clipped.
pub fn postprocess(
    result: &mut Planes,
    original: &Planes,
    options: &ScrambleOptions,
    set_target: Option<&Target>,
) -> crate::Result<f64> {
    if let Some(m) = &options.matching {
        let target = match m.target {
            MatchTarget::Original => {
                let original = if result.is_grayscale() && !original.is_grayscale() { original.to_grayscale() } else { original.clone() };
                Target::of_image(&original, m.property)
            }
            MatchTarget::Fixed { mean, rms_contrast } => {
                if m.property != MatchProperty::Luminance {
                    anyhow::bail!("Fixed targets are only available for luminance matching");
                }
                Target::Luminance { mean, rms_contrast }
            }
            MatchTarget::SetAverage => match set_target {
                Some(t) if t.property() == m.property => t.clone(),
                Some(_) => anyhow::bail!("The reference set target is for a different property"),
                None => anyhow::bail!("Matching to a set average needs a reference set"),
            },
        };
        matching::apply(result, &target)?;
    }

    Ok(match options.out_of_range {
        RangeHandling::Clip => 1.0,
        RangeHandling::CompressContrast => result.compress_to_range(),
    })
}

/// Scrambles float planes without clipping. Values may fall outside 0..1.
pub fn scramble_planes(planes: &Planes, options: &ScrambleOptions, seed: u64) -> Planes {
    let mut rng = seed::rng(seed);
    match &options.scramble_type {
        ScrambleType::Pixel => pixel::scramble(planes, options.intensity, &mut rng),
        ScrambleType::Fourier(o) => fourier::scramble(planes, o, &mut rng),
        ScrambleType::Block(o) => block::scramble(planes, o, &mut rng),
        ScrambleType::Blur(o) => blur::scramble(planes, o.sigma),
        ScrambleType::Diffeomorphic(o) => diffeomorphic::scramble(planes, o, &mut rng),
        ScrambleType::LocalPhase(o) => local_phase::scramble(planes, o, &mut rng),
    }
}

fn scramble_faces(
    image: &DynamicImage,
    planes: &Planes,
    options: &ScrambleOptions,
    face_opts: &FaceDetectionOptions,
    seed: u64,
) -> crate::Result<(Planes, Vec<aperture::FaceBox>)> {
    let session = face_detection::load_face_detector(None)?;
    let regions = face_detection::detect_face_regions(
        image,
        session,
        face_opts.confidence_threshold,
        Some(face_opts.expansion_factor),
    )?;

    // A grayscale Fourier result cannot be pasted into a colour image, so convert first.
    let grayscale_output = matches!(&options.scramble_type, ScrambleType::Fourier(o) if o.grayscale);
    let base = if grayscale_output { planes.to_grayscale() } else { planes.clone() };

    let mut result = match face_opts.background_mode {
        BackgroundMode::Include => base.clone(),
        BackgroundMode::Exclude => Planes {
            color: base.color.iter().map(|p| vec![0.0; p.len()]).collect(),
            alpha: Some(vec![0.0; base.len()]),
            ..base.clone()
        },
    };

    for (index, region) in regions.iter().enumerate() {
        let Some(r) = mask::face_region(image, region, face_opts)? else { continue };
        let patch = scramble_planes(&base.crop(r.x, r.y, r.width, r.height), options, seed::derive(seed, index as u64));
        blend_patch(&mut result, &patch, &r.weights, r.x, r.y, face_opts.background_mode);
    }
    Ok((result, regions.iter().map(aperture::FaceBox::from).collect()))
}

/// Blends a scrambled patch into `result` at (x, y) with per-pixel `weights`. With a removed
/// background the weight becomes the alpha of the output.
fn blend_patch(result: &mut Planes, patch: &Planes, weights: &[f32], x: usize, y: usize, mode: BackgroundMode) {
    let width = result.width;
    for py in 0..patch.height {
        for px in 0..patch.width {
            let m = weights[py * patch.width + px];
            if m <= 0.0 {
                continue;
            }
            let (i, j) = (py * patch.width + px, (y + py) * width + x + px);
            match mode {
                BackgroundMode::Include => {
                    for (dst, src) in result.color.iter_mut().zip(&patch.color) {
                        dst[j] = m * src[i] + (1.0 - m) * dst[j];
                    }
                }
                BackgroundMode::Exclude => {
                    let alpha = result.alpha.as_mut().expect("removed background has alpha");
                    if m >= alpha[j] {
                        for (dst, src) in result.color.iter_mut().zip(&patch.color) {
                            dst[j] = src[i];
                        }
                        alpha[j] = m * patch.alpha.as_ref().map_or(1.0, |a| a[i]);
                    }
                }
            }
        }
    }
}

/// Unit of the strength parameter varied by an intensity series, or `None` when the method
/// has none (block scrambling).
pub fn series_unit(scramble_type: &ScrambleType) -> Option<&'static str> {
    match scramble_type {
        ScrambleType::Pixel | ScrambleType::Fourier(_) | ScrambleType::LocalPhase(_) => Some("%"),
        ScrambleType::Blur(_) | ScrambleType::Diffeomorphic(_) => Some("px"),
        ScrambleType::Block(_) => None,
    }
}

/// Options with the method's strength set to `level`: intensity in percent for pixel, Fourier
/// and local phase scrambling, sigma in pixels for blur, maximum distortion in pixels for
/// diffeomorphic scrambling.
pub fn with_level(options: &ScrambleOptions, level: f32) -> crate::Result<ScrambleOptions> {
    let mut options = options.clone();
    match &mut options.scramble_type {
        ScrambleType::Pixel => options.intensity = level / 100.0,
        ScrambleType::Fourier(o) => o.intensity = level / 100.0,
        ScrambleType::LocalPhase(o) => o.intensity = level / 100.0,
        ScrambleType::Blur(o) => o.sigma = level,
        ScrambleType::Diffeomorphic(o) => o.max_distortion = level,
        ScrambleType::Block(_) => anyhow::bail!("Block scrambling has no strength to vary"),
    }
    Ok(options)
}
