//! Mirror plugin: horizontal and/or vertical flip of an RGBA image.

use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

const BYTES_PER_PIXEL: usize = 4;

/// Parsed mirror parameters. Missing or invalid keys keep defaults (`false`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirrorParams {
    pub horizontal: bool,
    pub vertical: bool,
}

impl Default for MirrorParams {
    fn default() -> Self {
        Self {
            horizontal: false,
            vertical: false,
        }
    }
}

/// Parses `key=value` pairs separated by commas and/or newlines.
///
/// Unknown keys are ignored. Invalid boolean values leave the field at default.
pub fn parse_params(input: &str) -> MirrorParams {
    let mut params = MirrorParams::default();

    for token in input.split(|c: char| c == ',' || c.is_whitespace()) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "horizontal" => {
                if let Some(v) = parse_bool(value) {
                    params.horizontal = v;
                }
            }
            "vertical" => {
                if let Some(v) = parse_bool(value) {
                    params.vertical = v;
                }
            }
            _ => {}
        }
    }

    params
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

/// Mirrors the RGBA buffer in place according to `params`.
pub fn apply_mirror(width: u32, height: u32, rgba: &mut [u8], params: MirrorParams) {
    let w = width as usize;
    let h = height as usize;
    let expected = w
        .checked_mul(h)
        .and_then(|n| n.checked_mul(BYTES_PER_PIXEL));
    let Some(expected) = expected else {
        return;
    };
    if rgba.len() != expected || w == 0 || h == 0 {
        return;
    }

    if params.horizontal {
        for y in 0..h {
            let row_start = y * w * BYTES_PER_PIXEL;
            for x in 0..(w / 2) {
                let left = row_start + x * BYTES_PER_PIXEL;
                let right = row_start + (w - 1 - x) * BYTES_PER_PIXEL;
                for i in 0..BYTES_PER_PIXEL {
                    rgba.swap(left + i, right + i);
                }
            }
        }
    }

    if params.vertical {
        for y in 0..(h / 2) {
            let top = y * w * BYTES_PER_PIXEL;
            let bottom = (h - 1 - y) * w * BYTES_PER_PIXEL;
            let row_bytes = w * BYTES_PER_PIXEL;
            for i in 0..row_bytes {
                rgba.swap(top + i, bottom + i);
            }
        }
    }
}

/// FFI entry point. Panics are caught so they never cross the FFI boundary.
#[unsafe(no_mangle)]
pub extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if rgba_data.is_null() {
            return;
        }
        let len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(BYTES_PER_PIXEL));
        let Some(len) = len else {
            return;
        };

        // SAFETY: Host guarantees a valid buffer of length width*height*4 for this call.
        let buffer = unsafe { slice::from_raw_parts_mut(rgba_data, len) };

        let params_str = if params.is_null() {
            ""
        } else {
            // SAFETY: Host passes a valid NUL-terminated C string that lives for this call.
            unsafe { std::ffi::CStr::from_ptr(params) }
                .to_str()
                .unwrap_or("")
        };

        let parsed = parse_params(params_str);
        // Work on a copy so a panic leaves the host buffer unchanged.
        let mut working = buffer.to_vec();
        apply_mirror(width, height, &mut working, parsed);
        buffer.copy_from_slice(&working);
    }));

    if result.is_err() {
        eprintln!("mirror plugin: panic caught inside process_image; buffer left unchanged");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_params() {
        let p = parse_params("horizontal=true,vertical=false");
        assert_eq!(
            p,
            MirrorParams {
                horizontal: true,
                vertical: false
            }
        );
    }

    #[test]
    fn parse_empty_uses_defaults() {
        assert_eq!(parse_params(""), MirrorParams::default());
        assert_eq!(parse_params("   "), MirrorParams::default());
    }

    #[test]
    fn parse_unknown_keys_ignored() {
        let p = parse_params("horizontal=true,foo=bar,vertical=true");
        assert_eq!(
            p,
            MirrorParams {
                horizontal: true,
                vertical: true
            }
        );
    }

    #[test]
    fn parse_invalid_bool_keeps_default() {
        let p = parse_params("horizontal=maybe,vertical=nope");
        assert_eq!(p, MirrorParams::default());
    }

    fn pixel(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
        [r, g, b, a]
    }

    /// 2×2 layout: A B / C D  as flat RGBA.
    fn buffer_2x2() -> Vec<u8> {
        let mut buf = Vec::with_capacity(16);
        buf.extend_from_slice(&pixel(1, 0, 0, 255)); // A
        buf.extend_from_slice(&pixel(2, 0, 0, 255)); // B
        buf.extend_from_slice(&pixel(3, 0, 0, 255)); // C
        buf.extend_from_slice(&pixel(4, 0, 0, 255)); // D
        buf
    }

    #[test]
    fn mirror_horizontal_2x2() {
        let mut buf = buffer_2x2();
        apply_mirror(
            2,
            2,
            &mut buf,
            MirrorParams {
                horizontal: true,
                vertical: false,
            },
        );
        // B A / D C
        let mut expected = Vec::new();
        expected.extend_from_slice(&pixel(2, 0, 0, 255));
        expected.extend_from_slice(&pixel(1, 0, 0, 255));
        expected.extend_from_slice(&pixel(4, 0, 0, 255));
        expected.extend_from_slice(&pixel(3, 0, 0, 255));
        assert_eq!(buf, expected);
    }

    #[test]
    fn mirror_vertical_2x2() {
        let mut buf = buffer_2x2();
        apply_mirror(
            2,
            2,
            &mut buf,
            MirrorParams {
                horizontal: false,
                vertical: true,
            },
        );
        // C D / A B
        let mut expected = Vec::new();
        expected.extend_from_slice(&pixel(3, 0, 0, 255));
        expected.extend_from_slice(&pixel(4, 0, 0, 255));
        expected.extend_from_slice(&pixel(1, 0, 0, 255));
        expected.extend_from_slice(&pixel(2, 0, 0, 255));
        assert_eq!(buf, expected);
    }

    #[test]
    fn mirror_both_2x2() {
        let mut buf = buffer_2x2();
        apply_mirror(
            2,
            2,
            &mut buf,
            MirrorParams {
                horizontal: true,
                vertical: true,
            },
        );
        // D C / B A
        let mut expected = Vec::new();
        expected.extend_from_slice(&pixel(4, 0, 0, 255));
        expected.extend_from_slice(&pixel(3, 0, 0, 255));
        expected.extend_from_slice(&pixel(2, 0, 0, 255));
        expected.extend_from_slice(&pixel(1, 0, 0, 255));
        assert_eq!(buf, expected);
    }
}
