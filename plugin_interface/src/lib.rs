//! Shared FFI types for image-processing plugins.

use std::os::raw::c_char;

/// C-compatible signature exported by every plugin as `process_image`.
pub type ProcessImageFn =
    unsafe extern "C" fn(width: u32, height: u32, rgba_data: *mut u8, params: *const c_char);
