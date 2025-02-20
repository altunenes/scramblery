use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::scramble::ScrambleOptions;

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchProcessingOptions {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub scramble_options: ScrambleOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchProgress {
    pub total_files: usize,
    pub processed_files: usize,
    pub current_file: Option<PathBuf>,
}

pub trait ProgressCallback: Fn(BatchProgress) + Send + Sync {}
impl<T> ProgressCallback for T where T: Fn(BatchProgress) + Send + Sync {}