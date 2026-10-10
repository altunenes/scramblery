pub mod analysis;
pub mod io;
pub mod matching;
pub mod normalize;
pub mod planes;
pub mod scramble;
pub mod seed;
pub mod spectral;
pub mod batch;
pub mod video;

pub use scramble::*;
pub use batch::*;
pub use video::*;

/// Version written into reports and settings files.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Result<T> = std::result::Result<T, anyhow::Error>;