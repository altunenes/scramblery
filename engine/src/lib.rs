pub mod scramble;
pub mod utils;
pub mod batch;

pub use scramble::*;
pub use utils::*;
pub use batch::*;

pub type Result<T> = std::result::Result<T, anyhow::Error>;