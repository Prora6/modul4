//! Dynamic loading of image-processing plugins via `libloading`.

use std::ffi::CString;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};
use plugin_interface::ProcessImageFn;

use crate::error::AppError;

/// Builds the platform-specific shared library file name from a bare plugin name.
///
/// Examples for `"mirror"`:
/// - Linux: `libmirror.so`
/// - macOS: `libmirror.dylib`
/// - Windows: `mirror.dll`
pub fn library_file_name(plugin: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{plugin}.dll")
    }
    #[cfg(target_os = "macos")]
    {
        format!("lib{plugin}.dylib")
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        format!("lib{plugin}.so")
    }
}

/// Full path: `{plugin_path}/{library_file_name}`.
pub fn resolve_plugin_path(plugin_dir: &Path, plugin: &str) -> PathBuf {
    plugin_dir.join(library_file_name(plugin))
}

/// Loads the plugin, resolves `process_image`, and runs it on the RGBA buffer.
///
/// The `Library` and `CString` stay alive for the entire call so symbols and
/// the params pointer remain valid (no dangling pointers / use-after-unload).
pub fn run_plugin(
    plugin_path: &Path,
    width: u32,
    height: u32,
    rgba_data: &mut [u8],
    params: &str,
) -> Result<(), AppError> {
    if !plugin_path.exists() {
        return Err(AppError::PluginNotFound(plugin_path.to_path_buf()));
    }

    let expected_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(AppError::InvalidBuffer { width, height })?;

    if rgba_data.len() != expected_len {
        return Err(AppError::InvalidBuffer { width, height });
    }

    // Keep the CString alive until after `process` returns.
    let params_c = CString::new(params)?;

    // SAFETY: `Library::new` runs the dynamic loader for a path we verified exists.
    // The library must outlive any `Symbol` obtained from it.
    let library = unsafe { Library::new(plugin_path)? };

    // SAFETY: We look up a C symbol by name; the type must match the plugin export.
    let process: Symbol<ProcessImageFn> = unsafe { library.get(b"process_image\0")? };

    // SAFETY: `rgba_data` is a valid mutable buffer of length width*height*4 owned
    // by the caller for the duration of this call. `params_c` stays alive here.
    // The plugin must not free this memory and must stay within buffer bounds.
    unsafe {
        process(
            width,
            height,
            rgba_data.as_mut_ptr(),
            params_c.as_ptr(),
        );
    }

    // Explicitly keep `library` until after the call (drop order would also work,
    // but this documents the lifetime requirement from the assignment).
    drop(process);
    drop(library);

    Ok(())
}
