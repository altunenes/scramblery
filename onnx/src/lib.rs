use std::fmt;
use std::path::PathBuf;
use log::{info, debug};
use once_cell::sync::Lazy;
use anyhow::Result;
use ort::session::builder::GraphOptimizationLevel;
use ort::ep::CPU;

#[derive(Clone)]
pub enum ExecutionProvider {
    CPU,
    #[cfg(target_os = "macos")]
    CoreML,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    CUDA,
    #[cfg(target_os = "windows")]
    DirectML,
}

impl Default for ExecutionProvider {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        { ExecutionProvider::CoreML }
        #[cfg(all(not(target_os = "macos"), any(target_os = "windows", target_os = "linux")))]
        { ExecutionProvider::CUDA }
        #[cfg(all(not(target_os = "macos"), not(any(target_os = "windows", target_os = "linux"))))]
        { ExecutionProvider::CPU }
    }
}

impl fmt::Debug for ExecutionProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionProvider::CPU => write!(f, "CPU"),
            #[cfg(target_os = "macos")]
            ExecutionProvider::CoreML => write!(f, "CoreML"),
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            ExecutionProvider::CUDA => write!(f, "CUDA"),
            #[cfg(target_os = "windows")]
            ExecutionProvider::DirectML => write!(f, "DirectML"),
        }
    }
}

pub struct ModelConfig {
    pub execution_provider: ExecutionProvider,
    pub intra_threads: usize,
    pub inter_threads: usize,
    pub configure: Option<Box<dyn Fn(ort::session::builder::SessionBuilder) -> Result<ort::session::builder::SessionBuilder> + Send + Sync>>,
}

impl fmt::Debug for ModelConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelConfig")
            .field("execution_provider", &self.execution_provider)
            .field("intra_threads", &self.intra_threads)
            .field("inter_threads", &self.inter_threads)
            .field(
                "configure",
                &if self.configure.is_some() {
                    "<fn>"
                } else {
                    "None"
                },
            )
            .finish()
    }
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            execution_provider: ExecutionProvider::default(),
            intra_threads: 4,
            inter_threads: 1,
            configure: None,
        }
    }
}

/// Global ONNX Runtime environment initialization
pub static ENV: Lazy<()> = Lazy::new(|| {
    info!("Initializing ONNX Runtime environment");

    let mut providers: Vec<ort::ep::ExecutionProviderDispatch> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        use ort::ep::coreml::{ComputeUnits, CoreML};
        providers.push(
            CoreML::default()
                .with_compute_units(ComputeUnits::CPUAndGPU)
                .build(),
        );
    }

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        providers.push(ort::ep::CUDA::default().build());
    }

    #[cfg(target_os = "windows")]
    {
        providers.push(ort::ep::DirectML::default().build());
    }

    providers.push(CPU::default().build().error_on_failure());

    let success = ort::init()
        .with_execution_providers(providers)
        .with_name("onnx_runtime")
        .commit();
    if !success {
        debug!("Failed to initialize ONNX Runtime environment");
        panic!("ONNX Runtime initialization failed");
    }
    info!("ONNX Runtime environment initialized successfully");
});

pub fn new_session_from_path(path: PathBuf) -> Result<Session> {
    let config = ModelConfig::default();
    new_session_from_path_with_config(path, &config)
}

pub fn new_session_from_path_with_config(path: PathBuf, config: &ModelConfig) -> Result<Session> {
    Lazy::force(&ENV);
    info!("Creating new ONNX session from path: {:?} with config: {:?}", path, config);

    let mut builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(config.intra_threads)?
        .with_inter_threads(config.inter_threads)?;

    if let Some(ref configure) = config.configure {
        builder = configure(builder)?;
    }

    let session = builder.commit_from_file(path)?;

    info!("ONNX session created successfully");
    Ok(session)
}

pub use ort::value::{Tensor, Value};
pub use ort::inputs;
pub use ort::session::Session;
