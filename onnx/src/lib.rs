use std::fmt;
use std::path::PathBuf;
use log::info;
use once_cell::sync::Lazy;
use std::sync::Mutex;
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
    #[cfg(feature = "openvino")]
    OpenVINO,
}

impl Default for ExecutionProvider {
    fn default() -> Self {
        // WebGPU, then the platform's native provider, then CUDA or MIGraphX, then CPU.
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
            #[cfg(feature = "openvino")]
            { return ExecutionProvider::OpenVINO; }
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

/// Where CoreML keeps compiled models, so that only the first run pays the compilation time.
#[cfg(target_os = "macos")]
fn coreml_cache_dir() -> String {
    let base = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Caches"))
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("scramblery/coreml");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Cannot create CoreML cache folder {}: {e}", dir.display());
    }
    info!("CoreML model cache: {}", dir.display());
    dir.to_string_lossy().into_owned()
}

/// The GPU execution providers compiled into this build, in order of preference.
fn gpu_providers() -> Vec<(&'static str, ort::ep::ExecutionProviderDispatch)> {
    #[allow(unused_mut)]
    let mut providers: Vec<(&'static str, ort::ep::ExecutionProviderDispatch)> = Vec::new();

    #[cfg(target_os = "macos")]
    {
        use ort::ep::coreml::{ComputeUnits, CoreML, ModelFormat};
        providers.push((
            "CoreML",
            CoreML::default()
                .with_compute_units(ComputeUnits::CPUAndGPU)
                .with_model_format(ModelFormat::MLProgram)
                .with_static_input_shapes(true)
                .with_model_cache_dir(coreml_cache_dir())
                .build(),
        ));
    }

    #[cfg(feature = "cuda")]
    providers.push(("CUDA", ort::ep::CUDA::default().build()));

    #[cfg(feature = "migraphx")]
    providers.push(("MIGraphX", ort::ep::MIGraphX::default().build()));

    #[cfg(feature = "webgpu")]
    providers.push(("WebGPU", ort::ep::WebGPU::default().build()));

    // Intel GPUs; OpenVINO's own CPU path is left to the default CPU provider.
    #[cfg(feature = "openvino")]
    providers.push(("OpenVINO", ort::ep::OpenVINO::default().with_device_type("GPU").build()));

    #[cfg(all(feature = "directml", target_os = "windows"))]
    {
        use ort::ep::directml::{DeviceFilter, DirectML, PerformancePreference};
        providers.push((
            "DirectML",
            DirectML::default()
                .with_performance_preference(PerformancePreference::HighPerformance)
                .with_device_filter(DeviceFilter::Gpu)
                .build(),
        ));
    }

    providers
}

/// see ort rc14: release notes.
static ENV: Lazy<std::result::Result<ort::environment::Environment, String>> = Lazy::new(|| {
    info!("Initializing ONNX Runtime environment");
    ort::init().with_name("scramblery").build().map_err(|e| e.to_string())
});

static DEVICE: Mutex<Option<String>> = Mutex::new(None);

/// Where the most recently created GPU-capable session runs, e.g. "CUDA" or
/// "CPU (CUDA not available)". None before any such session exists.
pub fn device() -> Option<String> {
    DEVICE.lock().ok()?.clone()
}

fn set_device(device: String) {
    info!("ONNX session runs on {device}");
    if let Ok(mut slot) = DEVICE.lock() {
        *slot = Some(device);
    }
}

fn builder_error(e: ort::Error<ort::session::builder::SessionBuilder>) -> ort::Error {
    ort::Error::new(e.message().to_string())
}

fn cpu_builder(config: &ModelConfig) -> Result<ort::session::builder::SessionBuilder> {
    Ok(session_builder(config)?
        .with_no_environment_execution_providers()
        .map_err(builder_error)?
        .with_execution_providers([CPU::default().build()])
        .map_err(builder_error)?)
}

/// Commits a session on the first GPU provider that registers and loads the model, else on the CPU.
fn commit_preferring_gpu(
    config: &ModelConfig,
    commit: impl Fn(&mut ort::session::builder::SessionBuilder) -> ort::Result<Session>,
) -> Result<Session> {
    let mut failed = Vec::new();
    for (name, provider) in gpu_providers() {
        let builder = session_builder(config)?
            .with_no_environment_execution_providers()
            .map_err(builder_error)?;
        match builder.with_execution_providers([provider.error_on_failure()]) {
            Ok(mut builder) => match commit(&mut builder) {
                Ok(session) => {
                    set_device(name.to_string());
                    return Ok(session);
                }
                Err(e) => log::warn!("{name} could not load the model: {e}"),
            },
            Err(e) => log::warn!("{name} is not available: {}", e.message()),
        }
        failed.push(name);
    }
    let session = commit(&mut cpu_builder(config)?)?;
    set_device(if failed.is_empty() {
        "CPU".to_string()
    } else {
        format!("CPU ({} not available)", failed.join(", "))
    });
    Ok(session)
}

pub fn new_session_from_path(path: PathBuf) -> Result<Session> {
    let config = ModelConfig::default();
    new_session_from_path_with_config(path, &config)
}

pub fn new_session_from_path_with_config(path: PathBuf, config: &ModelConfig) -> Result<Session> {
    info!("Creating new ONNX session from path: {:?} with config: {:?}", path, config);
    let session = commit_preferring_gpu(config, |builder| builder.commit_from_file(&path))?;
    info!("ONNX session created successfully");
    Ok(session)
}

/// Creates a session from in-memory model bytes.
pub fn new_session_from_memory(bytes: &[u8]) -> Result<Session> {
    let config = ModelConfig::default();
    info!("Creating new ONNX session from memory ({} bytes) with config: {:?}", bytes.len(), config);
    let session = commit_preferring_gpu(&config, |builder| builder.commit_from_memory(bytes))?;
    info!("ONNX session created successfully");
    Ok(session)
}

/// Creates a session from in-memory model bytes that runs on one CPU thread only, ignoring
/// GPU execution providers. Results are then identical across runs, processes and machines.
pub fn new_deterministic_session_from_memory(bytes: &[u8]) -> Result<Session> {
    let config = ModelConfig { intra_threads: 1, ..ModelConfig::default() };
    info!("Creating CPU-only ONNX session from memory ({} bytes)", bytes.len());
    Ok(cpu_builder(&config)?.commit_from_memory(bytes)?)
}

fn session_builder(config: &ModelConfig) -> Result<ort::session::builder::SessionBuilder> {
    let env = ENV
        .as_ref()
        .map_err(|e| anyhow::anyhow!("ONNX Runtime initialization failed: {e}"))?;
    let mut builder = Session::builder(env)?
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
