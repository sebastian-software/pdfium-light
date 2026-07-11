// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Rust-owned bitmap, color, and compositing primitives.
//!
//! The initial boundary batches separable blend operations so production row
//! compositors can cross the native boundary once per row instead of once per
//! channel. The C++ implementation remains the differential oracle until that
//! row boundary is activated.

use std::slice;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum BlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
    Overlay = 3,
    Darken = 4,
    Lighten = 5,
    ColorDodge = 6,
    ColorBurn = 7,
    HardLight = 8,
    SoftLight = 9,
    Difference = 10,
    Exclusion = 11,
}

impl TryFrom<u8> for BlendMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Multiply),
            2 => Ok(Self::Screen),
            3 => Ok(Self::Overlay),
            4 => Ok(Self::Darken),
            5 => Ok(Self::Lighten),
            6 => Ok(Self::ColorDodge),
            7 => Ok(Self::ColorBurn),
            8 => Ok(Self::HardLight),
            9 => Ok(Self::SoftLight),
            10 => Ok(Self::Difference),
            11 => Ok(Self::Exclusion),
            _ => Err(()),
        }
    }
}

// Byte-exact PDFium soft-light transfer table. This preserves the historical
// integer results across platforms instead of recomputing them with floating
// point at runtime.
const COLOR_SQRT: [u8; 256] = [
    0x00, 0x03, 0x07, 0x0B, 0x0F, 0x12, 0x16, 0x19, 0x1D, 0x20, 0x23, 0x26, 0x29, 0x2C, 0x2F, 0x32,
    0x35, 0x37, 0x3A, 0x3C, 0x3F, 0x41, 0x43, 0x46, 0x48, 0x4A, 0x4C, 0x4E, 0x50, 0x52, 0x54, 0x56,
    0x57, 0x59, 0x5B, 0x5C, 0x5E, 0x60, 0x61, 0x63, 0x64, 0x65, 0x67, 0x68, 0x69, 0x6B, 0x6C, 0x6D,
    0x6E, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8C, 0x8D, 0x8E,
    0x8F, 0x90, 0x91, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x97, 0x98, 0x99, 0x9A, 0x9B, 0x9C,
    0x9C, 0x9D, 0x9E, 0x9F, 0xA0, 0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA4, 0xA5, 0xA6, 0xA7, 0xA7, 0xA8,
    0xA9, 0xAA, 0xAA, 0xAB, 0xAC, 0xAD, 0xAD, 0xAE, 0xAF, 0xB0, 0xB0, 0xB1, 0xB2, 0xB3, 0xB3, 0xB4,
    0xB5, 0xB5, 0xB6, 0xB7, 0xB7, 0xB8, 0xB9, 0xBA, 0xBA, 0xBB, 0xBC, 0xBC, 0xBD, 0xBE, 0xBE, 0xBF,
    0xC0, 0xC0, 0xC1, 0xC2, 0xC2, 0xC3, 0xC4, 0xC4, 0xC5, 0xC6, 0xC6, 0xC7, 0xC7, 0xC8, 0xC9, 0xC9,
    0xCA, 0xCB, 0xCB, 0xCC, 0xCC, 0xCD, 0xCE, 0xCE, 0xCF, 0xD0, 0xD0, 0xD1, 0xD1, 0xD2, 0xD3, 0xD3,
    0xD4, 0xD4, 0xD5, 0xD6, 0xD6, 0xD7, 0xD7, 0xD8, 0xD9, 0xD9, 0xDA, 0xDA, 0xDB, 0xDC, 0xDC, 0xDD,
    0xDD, 0xDE, 0xDE, 0xDF, 0xE0, 0xE0, 0xE1, 0xE1, 0xE2, 0xE2, 0xE3, 0xE4, 0xE4, 0xE5, 0xE5, 0xE6,
    0xE6, 0xE7, 0xE7, 0xE8, 0xE9, 0xE9, 0xEA, 0xEA, 0xEB, 0xEB, 0xEC, 0xEC, 0xED, 0xED, 0xEE, 0xEE,
    0xEF, 0xF0, 0xF0, 0xF1, 0xF1, 0xF2, 0xF2, 0xF3, 0xF3, 0xF4, 0xF4, 0xF5, 0xF5, 0xF6, 0xF6, 0xF7,
    0xF7, 0xF8, 0xF8, 0xF9, 0xF9, 0xFA, 0xFA, 0xFB, 0xFB, 0xFC, 0xFC, 0xFD, 0xFD, 0xFE, 0xFE, 0xFF,
];

fn blend(mode: BlendMode, backdrop: u8, source: u8) -> u8 {
    let backdrop = i32::from(backdrop);
    let source = i32::from(source);
    let result = match mode {
        BlendMode::Normal => source,
        BlendMode::Multiply => source * backdrop / 255,
        BlendMode::Screen => source + backdrop - source * backdrop / 255,
        BlendMode::Overlay => return blend(BlendMode::HardLight, source as u8, backdrop as u8),
        BlendMode::Darken => source.min(backdrop),
        BlendMode::Lighten => source.max(backdrop),
        BlendMode::ColorDodge if source == 255 => 255,
        BlendMode::ColorDodge => (backdrop * 255 / (255 - source)).min(255),
        BlendMode::ColorBurn if source == 0 => 0,
        BlendMode::ColorBurn => 255 - ((255 - backdrop) * 255 / source).min(255),
        BlendMode::HardLight if source < 128 => source * backdrop * 2 / 255,
        BlendMode::HardLight => {
            return blend(BlendMode::Screen, backdrop as u8, (2 * source - 255) as u8)
        }
        BlendMode::SoftLight if source < 128 => {
            backdrop - (255 - 2 * source) * backdrop * (255 - backdrop) / 255 / 255
        }
        BlendMode::SoftLight => {
            backdrop
                + (2 * source - 255) * (i32::from(COLOR_SQRT[backdrop as usize]) - backdrop) / 255
        }
        BlendMode::Difference => (backdrop - source).abs(),
        BlendMode::Exclusion => backdrop + source - 2 * backdrop * source / 255,
    };
    result as u8
}

fn alpha_merge(backdrop: u8, source: u8, source_alpha: u8) -> u8 {
    let backdrop = u32::from(backdrop);
    let source = u32::from(source);
    let source_alpha = u32::from(source_alpha);
    ((backdrop * (255 - source_alpha) + source * source_alpha) / 255) as u8
}

fn composite_bgra_pixel(mode: BlendMode, input: [u8; 4], clip: u8, output: &mut [u8; 4]) {
    let source_alpha = (u16::from(input[3]) * u16::from(clip) / 255) as u8;
    if output[3] == 0 {
        output[..3].copy_from_slice(&input[..3]);
        output[3] = source_alpha;
        return;
    }
    if source_alpha == 0 {
        return;
    }

    let destination_alpha = (u16::from(output[3]) + u16::from(source_alpha)
        - u16::from(output[3]) * u16::from(source_alpha) / 255) as u8;
    let alpha_ratio = (u16::from(source_alpha) * 255 / u16::from(destination_alpha)) as u8;
    for channel in 0..3 {
        let candidate = if mode == BlendMode::Normal {
            input[channel]
        } else {
            let blended = blend(mode, output[channel], input[channel]);
            alpha_merge(input[channel], blended, output[3])
        };
        output[channel] = alpha_merge(output[channel], candidate, alpha_ratio);
    }
    output[3] = destination_alpha;
}

// RUST_PORT_METRICS_BEGIN abi_thunk
/// Applies one separable blend mode to equally sized channel arrays.
///
/// # Safety
///
/// For non-zero `len`, `backdrop`, `source`, and `output` must be valid for
/// `len` bytes. The output region must be writable and must not overlap either
/// input region.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_blend_channels(
    mode: u8,
    backdrop: *const u8,
    source: *const u8,
    output: *mut u8,
    len: usize,
) -> bool {
    let Ok(mode) = BlendMode::try_from(mode) else {
        return false;
    };
    if len == 0 {
        return true;
    }

    // SAFETY: The caller contract above requires three valid, non-overlapping
    // regions with this exact length.
    let (backdrop, source, output) = unsafe {
        (
            slice::from_raw_parts(backdrop, len),
            slice::from_raw_parts(source, len),
            slice::from_raw_parts_mut(output, len),
        )
    };
    for ((&backdrop, &source), output) in backdrop.iter().zip(source).zip(output) {
        *output = blend(mode, backdrop, source);
    }
    true
}

/// Composites a packed BGRA source row into a packed BGRA destination row.
///
/// # Safety
///
/// `source` and `output` must each be valid for `pixel_count * 4` bytes.
/// `clip` must be null or valid for `pixel_count` bytes. Source and output may
/// alias exactly, but otherwise their regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_bgra_row(
    mode: u8,
    source: *const u8,
    clip: *const u8,
    output: *mut u8,
    pixel_count: usize,
) -> bool {
    let Ok(mode) = BlendMode::try_from(mode) else {
        return false;
    };
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees complete packed pixels at
        // each computed offset. Reading both values before writing supports
        // exact in-place operation without creating aliased references.
        unsafe {
            let offset = pixel * 4;
            let input = (source.add(offset) as *const [u8; 4]).read_unaligned();
            let mut destination = (output.add(offset) as *const [u8; 4]).read_unaligned();
            let clip = if clip.is_null() { 255 } else { *clip.add(pixel) };
            composite_bgra_pixel(mode, input, clip, &mut destination);
            (output.add(offset) as *mut [u8; 4]).write_unaligned(destination);
        }
    }
    true
}
// RUST_PORT_METRICS_END abi_thunk

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_should_preserve_reference_edge_cases() {
        let cases = [
            (BlendMode::Multiply, 99, 99, 38),
            (BlendMode::Screen, 99, 99, 160),
            (BlendMode::ColorDodge, 99, 99, 161),
            (BlendMode::ColorBurn, 99, 99, 0),
            (BlendMode::SoftLight, 47, 199, 81),
            (BlendMode::Exclusion, 99, 99, 122),
        ];

        for (mode, backdrop, source, expected) in cases {
            assert_eq!(expected, blend(mode, backdrop, source));
        }
    }

    #[test]
    fn blend_should_cover_every_channel_value_without_overflow() {
        for mode in 0..=BlendMode::Exclusion as u8 {
            let mode = BlendMode::try_from(mode).expect("test mode is separable");
            for backdrop in u8::MIN..=u8::MAX {
                for source in u8::MIN..=u8::MAX {
                    let _ = blend(mode, backdrop, source);
                }
            }
        }
    }

    #[test]
    fn composite_bgra_pixel_should_preserve_alpha_boundaries() {
        let mut transparent_destination = [255, 100, 0, 0];
        composite_bgra_pixel(
            BlendMode::Normal,
            [100, 0, 255, 100],
            255,
            &mut transparent_destination,
        );
        assert_eq!([100, 0, 255, 100], transparent_destination);

        let mut clipped_destination = [255, 100, 0, 200];
        composite_bgra_pixel(BlendMode::Darken, [100, 0, 255, 200], 0, &mut clipped_destination);
        assert_eq!([255, 100, 0, 200], clipped_destination);
    }
}
