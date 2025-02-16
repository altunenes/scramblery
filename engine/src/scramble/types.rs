use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ScrambleType {
    Pixel,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BackgroundMode {
    Include,    // Keep background as is, only scramble faces
    Exclude,    // Output only the scrambled face regions
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScrambleOptions {
    pub intensity: f32,
    pub seed: Option<u64>,
    pub face_detection: Option<FaceDetectionOptions>,
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