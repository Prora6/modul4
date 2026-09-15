//! Application error types for the image processor CLI.

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("input image not found: {0}")]
    InputNotFound(PathBuf),

    #[error("params file not found: {0}")]
    ParamsNotFound(PathBuf),

    #[error("plugin library not found: {0}")]
    PluginNotFound(PathBuf),

    #[error("failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to load or decode image: {0}")]
    Image(#[from] image::ImageError),

    #[error("failed to load plugin: {0}")]
    PluginLoad(#[from] libloading::Error),

    #[error("params string contains interior null byte")]
    ParamsNul(#[from] std::ffi::NulError),

    #[error("invalid RGBA buffer size for {width}x{height} image")]
    InvalidBuffer { width: u32, height: u32 },
}
