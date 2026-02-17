use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScrambleType {
    Pixel,
    Fourier(FourierOptions),
    Block(BlockOptions),
    Blur(BlurOptions),
    Diffeomorphic(DiffeomorphicOptions),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BackgroundMode {
    Include,    // Keep background as is, only scramble faces
    Exclude,    // Output only the scrambled face regions
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrambleOptions {
    pub scramble_type: ScrambleType,
    pub intensity: f32,
    pub seed: Option<u64>,
    pub face_detection: Option<FaceDetectionOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FourierOptions {
    pub frequency_range: FrequencyRange,
    pub phase_scramble: bool,
    pub magnitude_scramble: bool,
    pub padding_mode: PaddingMode,
    pub intensity: f32,
    pub grayscale: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FrequencyRange {
    All,
    HighPass(f32),  // cutoff frequency (0.0 - 1.0)
    LowPass(f32),   // cutoff frequency (0.0 - 1.0)
    BandPass {
        low: f32,
        high: f32,
    },
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockOptions {
    pub block_size: (u32, u32),    // Width and height of blocks
    pub interpolate_edges: bool,    // Whether to smooth transitions between blocks
    pub padding_mode: PaddingMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PaddingMode {
    Zero,
    Reflect,
    Wrap,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FaceDetectionOptions {
    pub confidence_threshold: f32,
    pub expansion_factor: f32,
    pub background_mode: BackgroundMode,
}

impl Default for ScrambleOptions {
    fn default() -> Self {
        Self {
            scramble_type: ScrambleType::Pixel,
            intensity: 0.5,
            seed: None,
            face_detection: None,
        }
    }
}
//expansion_factor: How much to expand detected regions?
impl Default for FaceDetectionOptions {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.7,
            expansion_factor: 1.0,
            background_mode: BackgroundMode::Include,
        }
    }
}
impl Default for FourierOptions {
    fn default() -> Self {
        Self {
            frequency_range: FrequencyRange::All,
            phase_scramble: true,
            magnitude_scramble: false,
            padding_mode: PaddingMode::Reflect,
            intensity: 1.0,
            grayscale: false,
        }
    }
}
impl Default for BlockOptions {
    fn default() -> Self {
        Self {
            block_size: (32, 32),
            interpolate_edges: true,
            padding_mode: PaddingMode::Reflect,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlurOptions {
    pub sigma: f32,
}

impl Default for BlurOptions {
    fn default() -> Self {
        Self {
            sigma: 5.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiffeomorphicOptions {
    /// Maximum distortion in pixels (RMS displacement)
    pub max_distortion: f32,
    /// Number of iterative warp steps (higher = smoother, more invertible)
    pub n_steps: u32,
    /// Number of DCT components per axis (controls spatial frequency of warp)
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemporalCoherenceOptions {
    pub export_flow: bool,
    pub flow_output_dir: Option<PathBuf>,
    /// Insert a freshly scrambled keyframe every N frames to prevent error accumulation.
    /// Frame 0 is always a keyframe. Frames between keyframes are warped from the previous frame.
    /// E.g. keyframe_interval=30 means frames 0, 30, 60, ... are freshly scrambled.
    pub keyframe_interval: usize,
    /// Number of frames over which to crossfade between the warped result and a fresh scramble
    /// at keyframe boundaries. 0 = disabled (abrupt switch). E.g. blend_frames=3 means frames
    /// K-1, K, K+1 around each keyframe K are blended for a smooth transition.
    pub blend_frames: usize,
}