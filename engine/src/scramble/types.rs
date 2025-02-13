use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ScrambleType {
    Pixel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScrambleOptions {
    pub intensity: f32,
    pub seed: Option<u64>,
}

impl Default for ScrambleOptions {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            seed: None,
        }
    }
}
