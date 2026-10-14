use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::matching::MatchOptions;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ScrambleType {
    Pixel,
    Fourier(FourierOptions),
    Block(BlockOptions),
    Blur(BlurOptions),
    Diffeomorphic(DiffeomorphicOptions),
    LocalPhase(LocalPhaseOptions),
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum BackgroundMode {
    /// Scramble the faces and keep the rest of the image.
    Include,
    /// Output only the scrambled faces on a transparent background.
    Exclude,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ScrambleOptions {
    pub scramble_type: ScrambleType,
    /// Pixel scrambling: fraction of pixels that are shuffled (0..1).
    pub intensity: f32,
    /// Seed for all random choices. `None` picks a random seed, which is reported back.
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub face_detection: Option<FaceDetectionOptions>,
    /// Match a property of the scrambled image to a target after scrambling.
    #[serde(default)]
    pub matching: Option<MatchOptions>,
    /// Show the result through a window with a uniform surround, as the last step.
    #[serde(default)]
    pub aperture: Option<super::aperture::ApertureOptions>,
    /// Scramble and match in linear light: colour values are converted from sRGB-encoded to
    /// linear before processing and back afterwards.
    #[serde(default)]
    pub linear_light: bool,
    #[serde(default)]
    pub out_of_range: RangeHandling,
}

/// What to do with values that fall outside 0..1 (mostly after phase scrambling).
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum RangeHandling {
    /// Clamp to 0..1. The mean and contrast of the clipped areas change.
    #[default]
    Clip,
    /// Scale every channel around its own mean by one common factor so that all values fit.
    /// Channel means are kept exactly; contrast is reduced by the reported factor.
    CompressContrast,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FourierOptions {
    /// Scale of the random phase added to each frequency (0..1). At 1 the phase is uniformly random.
    pub intensity: f32,
    /// Frequencies whose phase is randomised. Cutoffs are given in `unit`.
    pub frequency_range: FrequencyRange,
    #[serde(default)]
    pub unit: FrequencyUnit,
    /// Set frequencies outside `frequency_range` to zero instead of leaving them intact.
    /// The mean (DC term) is always kept.
    #[serde(default)]
    pub remove_outside_range: bool,
    pub phase_scramble: bool,
    pub padding_mode: PaddingMode,
    /// Process the Rec. 709 luminance as a single channel and output a grayscale image.
    pub grayscale: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum FrequencyRange {
    All,
    LowPass(f32),
    HighPass(f32),
    BandPass { low: f32, high: f32 },
}

/// Unit of spatial-frequency cutoffs.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum FrequencyUnit {
    /// Cycles per pixel; Nyquist is 0.5.
    #[default]
    CyclesPerPixel,
    /// Cycles per image width; Nyquist is width / 2.
    CyclesPerImage,
    /// Cycles per degree of visual angle, given the display's pixels per degree.
    CyclesPerDegree { pixels_per_degree: f32 },
}

impl FrequencyUnit {
    /// Converts a frequency in this unit to cycles per pixel for an image `width` pixels wide.
    pub fn to_cycles_per_pixel(self, value: f32, width: usize) -> f64 {
        match self {
            FrequencyUnit::CyclesPerPixel => value as f64,
            FrequencyUnit::CyclesPerImage => value as f64 / width.max(1) as f64,
            FrequencyUnit::CyclesPerDegree { pixels_per_degree } => value as f64 / pixels_per_degree.max(f32::EPSILON) as f64,
        }
    }

    /// Converts cycles per pixel to this unit for an image `width` pixels wide.
    pub fn from_cycles_per_pixel(self, cpp: f64, width: usize) -> f64 {
        match self {
            FrequencyUnit::CyclesPerPixel => cpp,
            FrequencyUnit::CyclesPerImage => cpp * width as f64,
            FrequencyUnit::CyclesPerDegree { pixels_per_degree } => cpp * pixels_per_degree as f64,
        }
    }
}

/// How an image is extended before the Fourier transform.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum PaddingMode {
    /// No padding: the image is treated as periodic and its amplitude spectrum is preserved exactly.
    #[default]
    Wrap,
    /// Mirror the image to twice its size, which removes the wrap-around edge discontinuity.
    Reflect,
    /// Pad to twice the size with the image's mean value.
    #[serde(alias = "Zero")]
    Mean,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BlockOptions {
    /// Block width and height in pixels.
    pub block_size: (u32, u32),
    /// Average the two pixels on each side of every block boundary.
    pub interpolate_edges: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct FaceDetectionOptions {
    pub confidence_threshold: f32,
    /// Factor by which detected face boxes are enlarged.
    pub expansion_factor: f32,
    pub background_mode: BackgroundMode,
    /// Shape of the processed region within each face box.
    #[serde(default)]
    pub shape: MaskShape,
    /// Width of the soft edge as a fraction of the region's half-size (0 = hard edge, 1 = the
    /// weight falls off from the centre). See `scramble::mask`.
    #[serde(default)]
    pub feather: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Default)]
pub enum MaskShape {
    /// The whole face box.
    #[default]
    Rectangle,
    /// The ellipse inscribed in the face box.
    Ellipse,
    /// The face oval traced by facial landmarks (MediaPipe Face Mesh).
    Outline,
}

impl Default for ScrambleOptions {
    fn default() -> Self {
        Self {
            scramble_type: ScrambleType::Pixel,
            intensity: 0.5,
            seed: None,
            face_detection: None,
            matching: None,
            aperture: None,
            linear_light: false,
            out_of_range: RangeHandling::Clip,
        }
    }
}

impl Default for FaceDetectionOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            expansion_factor: 1.0,
            background_mode: BackgroundMode::Include,
            shape: MaskShape::Rectangle,
            feather: 0.0,
        }
    }
}

impl Default for FourierOptions {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            frequency_range: FrequencyRange::All,
            unit: FrequencyUnit::CyclesPerPixel,
            remove_outside_range: false,
            phase_scramble: true,
            padding_mode: PaddingMode::Wrap,
            grayscale: false,
        }
    }
}

/// Local phase scrambling with a complex steerable filter bank (see `scramble::local_phase`).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LocalPhaseOptions {
    /// Scale of the random phase added to each band coefficient (0..1). At 1 the local phase is
    /// uniformly random.
    pub intensity: f32,
    /// Number of octave-spaced scales, the finest centred on 1/4 cycle per pixel. Structure
    /// coarser than the coarsest scale is kept.
    pub scales: u32,
    /// Number of orientations per scale.
    pub orientations: u32,
    pub padding_mode: PaddingMode,
}

impl Default for LocalPhaseOptions {
    fn default() -> Self {
        Self { intensity: 1.0, scales: 5, orientations: 4, padding_mode: PaddingMode::Reflect }
    }
}

impl Default for BlockOptions {
    fn default() -> Self {
        Self {
            block_size: (32, 32),
            interpolate_edges: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BlurOptions {
    /// Standard deviation of the Gaussian kernel in pixels.
    pub sigma: f32,
}

impl Default for BlurOptions {
    fn default() -> Self {
        Self { sigma: 5.0 }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DiffeomorphicOptions {
    /// RMS displacement of the final warp in pixels.
    pub max_distortion: f32,
    /// Number of small warp steps composed into the final warp.
    pub n_steps: u32,
    /// Number of DCT components per axis (spatial frequency of the warp field).
    pub n_comp: u32,
}

impl Default for DiffeomorphicOptions {
    fn default() -> Self {
        Self {
            max_distortion: 5.0,
            n_steps: 20,
            n_comp: 5,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TemporalCoherenceOptions {
    pub export_flow: bool,
    pub flow_output_dir: Option<PathBuf>,
    /// Frames after which each of the two layers that carry the scramble along the motion is
    /// renewed (at least 2). Shorter periods limit how far moved texture stretches; longer
    /// periods keep it with the content for longer.
    pub keyframe_interval: usize,
}
