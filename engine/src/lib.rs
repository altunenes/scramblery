pub mod scramble;
pub mod batch;
pub mod video;

pub use scramble::*;
pub use batch::*;
pub use video::*;

pub type Result<T> = std::result::Result<T, anyhow::Error>;