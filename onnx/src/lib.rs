use std::path::PathBuf;
use log::{info, debug};
use once_cell::sync::Lazy;
use anyhow::Result;
use ort::session::builder::GraphOptimizationLevel;
use ort::ep::CPU;

/// Global ONNX Runtime environment initialization
pub static ENV: Lazy<()> = Lazy::new(|| {
    info!("Initializing ONNX Runtime environment");
    let success = ort::init()
        .with_execution_providers([
            CPU::default().build().error_on_failure(),
        ])
        .with_name("onnx_runtime")
        .commit();
    if !success {
        debug!("Failed to initialize ONNX Runtime environment");
        panic!("ONNX Runtime initialization failed");
    }
    info!("ONNX Runtime environment initialized successfully");
});

pub fn new_session_from_path(path: PathBuf) -> Result<Session> {
    Lazy::force(&ENV);
    info!("Creating new ONNX session from path: {:?}", path);

    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .commit_from_file(path)?;

    info!("ONNX session created successfully");
    Ok(session)
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub input_name: String,
    pub input_dims: Vec<i64>,
    pub output_names: Vec<String>,
}

pub use ort::value::{Tensor, Value};
pub use ort::inputs;
pub use ort::session::Session;
