//! Core pipeline: load image → run plugin → save PNG.

mod error;
mod plugin_loader;

pub use error::AppError;
pub use plugin_loader::{library_file_name, resolve_plugin_path, run_plugin};

use std::fs;
use std::path::Path;

use image::RgbaImage;

/// Runs the full processing pipeline for the given paths.
pub fn process(
    input: &Path,
    output: &Path,
    plugin: &str,
    params_path: &Path,
    plugin_dir: &Path,
) -> Result<(), AppError> {
    if !input.exists() {
        return Err(AppError::InputNotFound(input.to_path_buf()));
    }
    if !params_path.exists() {
        return Err(AppError::ParamsNotFound(params_path.to_path_buf()));
    }

    let params = fs::read_to_string(params_path)?;

    let img = image::open(input)?;
    let rgba = img.to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let mut buffer = rgba.into_raw();

    let plugin_path = resolve_plugin_path(plugin_dir, plugin);
    run_plugin(&plugin_path, width, height, &mut buffer, &params)?;

    let out = RgbaImage::from_raw(width, height, buffer)
        .ok_or(AppError::InvalidBuffer { width, height })?;
    out.save(output)?;

    Ok(())
}
