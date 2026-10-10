use std::fmt;
use std::path::PathBuf;
use log::{info, debug};
use once_cell::sync::Lazy;
use anyhow::Result;
use ort::session::builder::GraphOptimizationLevel;
use ort::ep::CPU;

#[derive(Clone, Debug)]
pub enum ExecutionProvider {
    CPU,
    #[cfg(target_os = "macos")]
    CoreML,
    #[cfg(feature = "cuda")]
    CUDA,
    #[cfg(feature = "migraphx")]
    MIGraphX,
    #[cfg(feature = "webgpu")]
    WebGPU,
    #[cfg(all(feature = "directml", target_os = "windows"))]
    DirectML,
}

impl Default for ExecutionProvider {
    fn default() -> Self {
        // Priority: WebGPU (cross-platform), then platform-native GPU, then CUDA/ROCm, then CPU
        #[allow(unreachable_code)]
        {
            #[cfg(feature = "webgpu")]
            { return ExecutionProvider::WebGPU; }
            #[cfg(all(feature = "directml", target_os = "windows"))]
            { return ExecutionProvider::DirectML; }
            #[cfg(target_os = "macos")]
            { return ExecutionProvider::CoreML; }
            #[cfg(feature = "cuda")]
            { return ExecutionProvider::CUDA; }
            #[cfg(feature = "migraphx")]
            { return ExecutionProvider::MIGraphX; }
            ExecutionProvider::CPU
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

    #[cfg(feature = "cuda")]
    {
        providers.push(ort::ep::CUDA::default().build());
    }

    #[cfg(feature = "migraphx")]
    {
        providers.push(ort::ep::MIGraphX::default().build());
    }

    #[cfg(feature = "webgpu")]
    {
        providers.push(ort::ep::WebGPU::default().build());
    }

    #[cfg(all(feature = "directml", target_os = "windows"))]
    {
        use ort::ep::directml::{DeviceFilter, DirectML, PerformancePreference};
        providers.push(
            DirectML::default()
                .with_performance_preference(PerformancePreference::HighPerformance)
                .with_device_filter(DeviceFilter::Gpu)
                .build(),
        );
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
    info!("Creating new ONNX session from path: {:?} with config: {:?}", path, config);
    let session = session_builder(config)?.commit_from_file(path)?;
    info!("ONNX session created successfully");
    Ok(session)
}

/// Creates a session from in-memory model bytes.
pub fn new_session_from_memory(bytes: &[u8]) -> Result<Session> {
    let config = ModelConfig::default();
    info!("Creating new ONNX session from memory ({} bytes) with config: {:?}", bytes.len(), config);
    let session = session_builder(&config)?.commit_from_memory(bytes)?;
    info!("ONNX session created successfully");
    Ok(session)
}

fn session_builder(config: &ModelConfig) -> Result<ort::session::builder::SessionBuilder> {
    Lazy::force(&ENV);
    let mut builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3).map_err(|e| ort::Error::new(e.message().to_string()))?
        .with_intra_threads(config.intra_threads).map_err(|e| ort::Error::new(e.message().to_string()))?
        .with_inter_threads(config.inter_threads).map_err(|e| ort::Error::new(e.message().to_string()))?;

    if let Some(ref configure) = config.configure {
        builder = configure(builder)?;
    }
    Ok(builder)
}

/// Environment variable that overrides the directory bundled models are loaded from.
pub const MODELS_DIR_ENV: &str = "SCRAMBLERY_MODELS_DIR";

/// Locates a bundled model file by name.
///
/// Search order:
/// 1. `$SCRAMBLERY_MODELS_DIR/<name>`
/// 2. Next to the installed executable: `<exe dir>/models` (Windows, AppImage),
///    `<exe dir>/../Resources/models` (macOS .app), `<exe dir>/../lib/scramblery/models` (Linux .deb)
/// 3. The repository's `resources/models` (development builds)
pub fn find_model(name: &str) -> Result<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(dir) = std::env::var_os(MODELS_DIR_ENV) {
        candidates.push(PathBuf::from(dir).join(name));
    }

    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        candidates.push(exe_dir.join("models").join(name));
        candidates.push(exe_dir.join("../Resources/models").join(name));
        candidates.push(exe_dir.join("../lib/scramblery/models").join(name));
    }

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/models").join(name));

    match candidates.iter().find(|p| p.is_file()) {
        Some(path) => {
            info!("Found model {} at {:?}", name, path);
            Ok(path.clone())
        }
        None => anyhow::bail!("Model {} not found. Searched: {:?}", name, candidates),
    }
}

pub use ort::value::{Tensor, Value};
pub use ort::inputs;
pub use ort::session::Session;
