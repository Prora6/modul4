//! Blur plugin: box blur over an RGBA image.

use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

const BYTES_PER_PIXEL: usize = 4;

/// Parsed blur parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlurParams {
    pub radius: u32,
    pub iterations: u32,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self {
            radius: 1,
            iterations: 1,
        }
    }
}

/// Parses `key=value` pairs separated by commas and/or newlines.
///
/// Unknown keys are ignored. Invalid numeric values leave the field at default.
pub fn parse_params(input: &str) -> BlurParams {
    let mut params = BlurParams::default();

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
            "radius" => {
                if let Ok(v) = value.parse::<u32>() {
                    params.radius = v;
                }
            }
            "iterations" => {
                if let Ok(v) = value.parse::<u32>() {
                    params.iterations = v;
                }
            }
            _ => {}
        }
    }

    params
}

/// Applies box blur in place. Each channel (including alpha) is averaged.
pub fn apply_blur(width: u32, height: u32, rgba: &mut [u8], params: BlurParams) {
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

    if params.radius == 0 || params.iterations == 0 {
        return;
    }

    let radius = params.radius as isize;
    let mut src = rgba.to_vec();
    let mut dst = vec![0u8; expected];

    for _ in 0..params.iterations {
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0u32; 4];
                let mut count = 0u32;

                let y0 = (y as isize - radius).max(0) as usize;
                let y1 = (y as isize + radius).min(h as isize - 1) as usize;
                let x0 = (x as isize - radius).max(0) as usize;
                let x1 = (x as isize + radius).min(w as isize - 1) as usize;

                for yy in y0..=y1 {
                    for xx in x0..=x1 {
                        let idx = (yy * w + xx) * BYTES_PER_PIXEL;
                        for c in 0..BYTES_PER_PIXEL {
                            sum[c] += u32::from(src[idx + c]);
                        }
                        count += 1;
                    }
                }

                let out = (y * w + x) * BYTES_PER_PIXEL;
                for c in 0..BYTES_PER_PIXEL {
                    dst[out + c] = (sum[c] / count) as u8;
                }
            }
        }
        std::mem::swap(&mut src, &mut dst);
    }

    rgba.copy_from_slice(&src);
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
        apply_blur(width, height, &mut working, parsed);
        buffer.copy_from_slice(&working);
    }));

    if result.is_err() {
        eprintln!("blur plugin: panic caught inside process_image; buffer left unchanged");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_params() {
        let p = parse_params("radius=4,iterations=3");
        assert_eq!(
            p,
            BlurParams {
                radius: 4,
                iterations: 3
            }
        );
    }

    #[test]
    fn parse_empty_uses_defaults() {
        assert_eq!(parse_params(""), BlurParams::default());
    }

    #[test]
    fn parse_unknown_keys_ignored() {
        let p = parse_params("radius=2,foo=bar,iterations=5");
        assert_eq!(
            p,
            BlurParams {
                radius: 2,
                iterations: 5
            }
        );
    }

    #[test]
    fn parse_invalid_numbers_keep_defaults() {
        let p = parse_params("radius=abc,iterations=xyz");
        assert_eq!(p, BlurParams::default());
    }

    #[test]
    fn blur_monochrome_unchanged() {
        let mut buf = vec![100u8; 3 * 3 * 4];
        let original = buf.clone();
        apply_blur(
            3,
            3,
            &mut buf,
            BlurParams {
                radius: 1,
                iterations: 2,
            },
        );
        assert_eq!(buf, original);
    }

    #[test]
    fn blur_bright_center_spreads_to_neighbors() {
        // 3×3 all black except bright center pixel.
        let mut buf = vec![0u8; 3 * 3 * 4];
        let center = (1 * 3 + 1) * 4;
        buf[center] = 255;
        buf[center + 1] = 255;
        buf[center + 2] = 255;
        buf[center + 3] = 255;

        apply_blur(
            3,
            3,
            &mut buf,
            BlurParams {
                radius: 1,
                iterations: 1,
            },
        );

        // A neighbor (top-middle) must become non-zero after blur.
        let top = (0 * 3 + 1) * 4;
        assert!(
            buf[top] > 0 || buf[top + 1] > 0 || buf[top + 2] > 0,
            "neighbors should be non-zero after blurring a bright center"
        );
    }
}
