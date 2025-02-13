pub mod scramble;
pub mod utils;

pub use scramble::*;
pub use utils::*;

pub type Result<T> = std::result::Result<T, anyhow::Error>;