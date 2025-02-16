use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ScrambleType {
    Pixel,
    Fourier(FourierOptions),
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
        }
    }
}