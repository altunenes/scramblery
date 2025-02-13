pub mod scramble;
pub mod utils;

pub use scramble::*;

// Basic error type for the library
pub type Result<T> = std::result::Result<T, anyhow::Error>;