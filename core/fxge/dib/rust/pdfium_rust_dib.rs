// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Rust-owned bitmap, color, and compositing primitives.
//!
//! The initial boundary batches separable blend operations so production row
//! compositors can cross the native boundary once per row instead of once per
//! channel. The C++ implementation remains the differential oracle until that
//! row boundary is activated.

#[path = "pdfium_rust_cmyk_table.rs"]
mod cmyk_table;

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
    Hue = 12,
    Saturation = 13,
    Color = 14,
    Luminosity = 15,
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
            12 => Ok(Self::Hue),
            13 => Ok(Self::Saturation),
            14 => Ok(Self::Color),
            15 => Ok(Self::Luminosity),
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
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity => source,
    };
    result as u8
}

#[derive(Clone, Copy)]
struct Rgb {
    red: i32,
    green: i32,
    blue: i32,
}

fn lum(color: Rgb) -> i32 {
    (color.red * 30 + color.green * 59 + color.blue * 11) / 100
}

fn clip_color(mut color: Rgb) -> Rgb {
    let luminosity = lum(color);
    let minimum = color.red.min(color.green).min(color.blue);
    let maximum = color.red.max(color.green).max(color.blue);
    if minimum < 0 {
        color.red = luminosity + (color.red - luminosity) * luminosity / (luminosity - minimum);
        color.green = luminosity + (color.green - luminosity) * luminosity / (luminosity - minimum);
        color.blue = luminosity + (color.blue - luminosity) * luminosity / (luminosity - minimum);
    }
    if maximum > 255 {
        color.red =
            luminosity + (color.red - luminosity) * (255 - luminosity) / (maximum - luminosity);
        color.green =
            luminosity + (color.green - luminosity) * (255 - luminosity) / (maximum - luminosity);
        color.blue =
            luminosity + (color.blue - luminosity) * (255 - luminosity) / (maximum - luminosity);
    }
    color
}

fn set_lum(mut color: Rgb, luminosity: i32) -> Rgb {
    let delta = luminosity - lum(color);
    color.red += delta;
    color.green += delta;
    color.blue += delta;
    clip_color(color)
}

fn saturation(color: Rgb) -> i32 {
    color.red.max(color.green).max(color.blue) - color.red.min(color.green).min(color.blue)
}

fn set_saturation(mut color: Rgb, saturation: i32) -> Rgb {
    let minimum = color.red.min(color.green).min(color.blue);
    let maximum = color.red.max(color.green).max(color.blue);
    if minimum == maximum {
        return Rgb { red: 0, green: 0, blue: 0 };
    }
    color.red = (color.red - minimum) * saturation / (maximum - minimum);
    color.green = (color.green - minimum) * saturation / (maximum - minimum);
    color.blue = (color.blue - minimum) * saturation / (maximum - minimum);
    color
}

fn rgb_blend(mode: BlendMode, source: [u8; 4], backdrop: [u8; 4]) -> [u8; 3] {
    let source =
        Rgb { red: i32::from(source[2]), green: i32::from(source[1]), blue: i32::from(source[0]) };
    let backdrop = Rgb {
        red: i32::from(backdrop[2]),
        green: i32::from(backdrop[1]),
        blue: i32::from(backdrop[0]),
    };
    let result = match mode {
        BlendMode::Hue => set_lum(set_saturation(source, saturation(backdrop)), lum(backdrop)),
        BlendMode::Saturation => {
            set_lum(set_saturation(backdrop, saturation(source)), lum(backdrop))
        }
        BlendMode::Color => set_lum(source, lum(backdrop)),
        BlendMode::Luminosity => set_lum(backdrop, lum(source)),
        _ => source,
    };
    [result.blue as u8, result.green as u8, result.red as u8]
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
    let non_separable = matches!(
        mode,
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity
    );
    let blended_channels = non_separable.then(|| rgb_blend(mode, input, *output));
    for channel in 0..3 {
        let candidate = if mode == BlendMode::Normal {
            input[channel]
        } else if let Some(blended) = blended_channels {
            alpha_merge(input[channel], blended[channel], output[3])
        } else {
            let blended = blend(mode, output[channel], input[channel]);
            alpha_merge(input[channel], blended, output[3])
        };
        output[channel] = alpha_merge(output[channel], candidate, alpha_ratio);
    }
    output[3] = destination_alpha;
}

fn composite_mask_bgra_pixel(
    mode: BlendMode,
    input: [u8; 4],
    source_alpha: u8,
    output: &mut [u8; 4],
) {
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
    let blended_channels = rgb_blend(mode, input, *output);
    for channel in 0..3 {
        output[channel] = alpha_merge(output[channel], blended_channels[channel], alpha_ratio);
    }
    output[3] = destination_alpha;
}

fn composite_bgra_to_bgr_pixel(
    mode: BlendMode,
    input: [u8; 4],
    clip: u8,
    rgb_byte_order: bool,
    output: &mut [u8],
) {
    let source_alpha = (u16::from(input[3]) * u16::from(clip) / 255) as u8;
    let non_separable = matches!(
        mode,
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity
    );
    let backdrop = if rgb_byte_order {
        [output[2], output[1], output[0], 255]
    } else {
        [output[0], output[1], output[2], 255]
    };
    let blended_channels = non_separable.then(|| rgb_blend(mode, input, backdrop));
    for (channel, destination) in output[..3].iter_mut().enumerate() {
        let source_channel = if rgb_byte_order { 2 - channel } else { channel };
        let source = if mode == BlendMode::Normal {
            input[source_channel]
        } else if let Some(blended) = blended_channels {
            blended[source_channel]
        } else {
            blend(mode, *destination, input[source_channel])
        };
        *destination = alpha_merge(*destination, source, source_alpha);
    }
}

fn gray(input: [u8; 4]) -> u8 {
    ((u16::from(input[0]) * 11 + u16::from(input[1]) * 59 + u16::from(input[2]) * 30) / 100) as u8
}

fn composite_bgra_to_byte_pixel(
    mode: u8,
    input: [u8; 4],
    clip: u8,
    is_mask: bool,
    output: &mut u8,
) -> bool {
    let source_alpha = (u16::from(input[3]) * u16::from(clip) / 255) as u8;
    if is_mask {
        *output = (u16::from(*output) + u16::from(source_alpha)
            - u16::from(*output) * u16::from(source_alpha) / 255) as u8;
        return true;
    }
    if source_alpha == 0 {
        return mode <= 15;
    }

    let source_gray = gray(input);
    let candidate = match mode {
        0 => source_gray,
        1..=11 => {
            let Ok(mode) = BlendMode::try_from(mode) else {
                return false;
            };
            blend(mode, *output, source_gray)
        }
        12..=14 => *output,
        15 => source_gray,
        _ => return false,
    };
    *output = alpha_merge(*output, candidate, source_alpha);
    true
}

fn composite_opaque_pixel(
    mode: u8,
    input: [u8; 4],
    clip: u8,
    target: u8,
    rgb_byte_order: bool,
    output: &mut [u8],
) -> bool {
    match target {
        0 => composite_bgra_to_byte_pixel(mode, input, clip, false, &mut output[0]),
        1 => composite_bgra_to_byte_pixel(mode, input, clip, true, &mut output[0]),
        2 => {
            let Ok(mode) = BlendMode::try_from(mode) else {
                return false;
            };
            composite_bgra_to_bgr_pixel(mode, input, clip, rgb_byte_order, output);
            true
        }
        3 => {
            let Ok(mode) = BlendMode::try_from(mode) else {
                return false;
            };
            let mut destination = if rgb_byte_order {
                [output[2], output[1], output[0], output[3]]
            } else {
                [output[0], output[1], output[2], output[3]]
            };
            composite_bgra_pixel(mode, input, clip, &mut destination);
            if rgb_byte_order {
                output[..3].copy_from_slice(&[destination[2], destination[1], destination[0]]);
            } else {
                output[..3].copy_from_slice(&destination[..3]);
            }
            output[3] = destination[3];
            true
        }
        _ => false,
    }
}

fn adobe_cmyk_to_standard_rgb(cyan: u8, magenta: u8, yellow: u8, key: u8) -> [u8; 3] {
    let fixed = [cyan, magenta, yellow, key].map(|channel| i32::from(channel) << 8);
    let index = fixed.map(|channel| (channel + 4096) >> 13);
    let start = cmyk_table::CMYK
        [((index[0] * 9 + index[1]) * 9 + index[2]) as usize * 9 + index[3] as usize];
    let mut rgb = start.map(|channel| i32::from(channel) << 8);

    for dimension in 0..4 {
        let mut adjacent_index = fixed[dimension] >> 13;
        if adjacent_index == index[dimension] {
            adjacent_index = if adjacent_index == 8 { 7 } else { adjacent_index + 1 };
        }
        let mut adjacent = index;
        adjacent[dimension] = adjacent_index;
        let adjacent_rgb =
            cmyk_table::CMYK[((adjacent[0] * 9 + adjacent[1]) * 9 + adjacent[2]) as usize * 9
                + adjacent[3] as usize];
        let rate =
            (fixed[dimension] - (index[dimension] << 13)) * (index[dimension] - adjacent_index);
        for channel in 0..3 {
            rgb[channel] +=
                (i32::from(start[channel]) - i32::from(adjacent_rgb[channel])) * rate / 32;
        }
    }

    rgb.map(|channel| (channel.max(0) >> 8) as u8)
}

fn packed_rows_are_valid(width: usize, height: usize, pitch: usize, components: usize) -> bool {
    width.checked_mul(components).is_some_and(|row_bytes| row_bytes <= pitch)
        && pitch.checked_mul(height).is_some()
}

/// Clears a packed bitmap with one pixel value.
///
/// When `fill_padding` is set, the first pixel byte is written across the
/// entire buffer to preserve PDFium's 1-/8-bpp and gray-BGR padding behavior.
///
/// # Safety
///
/// `buffer` must be valid for `buffer_len` writable bytes. For packed-pixel
/// operation it must contain `pitch * height` bytes with at least
/// `width * components` bytes per row. `components` must be in `1..=4`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_clear_bitmap(
    buffer: *mut u8,
    buffer_len: usize,
    width: usize,
    height: usize,
    pitch: usize,
    components: usize,
    blue: u8,
    green: u8,
    red: u8,
    alpha: u8,
    fill_padding: bool,
) -> bool {
    if buffer.is_null() || !(1..=4).contains(&components) {
        return false;
    }
    if fill_padding {
        // SAFETY: The caller contract guarantees `buffer_len` writable bytes.
        unsafe {
            std::ptr::write_bytes(buffer, blue, buffer_len);
        }
        return true;
    }
    let Some(required_len) = pitch.checked_mul(height) else {
        return false;
    };
    if !packed_rows_are_valid(width, height, pitch, components) || required_len > buffer_len {
        return false;
    }
    let pixel = [blue, green, red, alpha];
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee a
            // complete packed pixel at every computed offset.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    pixel.as_ptr(),
                    buffer.add(row * pitch + column * components),
                    components,
                );
            }
        }
    }
    true
}

/// Converts packed BGR/BGRx/BGRA pixels to PDFium's integer gray value.
///
/// # Safety
///
/// `buffer` must cover `pitch * height` writable bytes, with at least
/// `width * components` bytes per row. `components` must be 3 or 4.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_convert_bgr_color_scale(
    buffer: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    components: usize,
) -> bool {
    if buffer.is_null()
        || !matches!(components, 3 | 4)
        || !packed_rows_are_valid(width, height, pitch, components)
    {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee a
            // complete BGR/BGRx/BGRA pixel at every computed offset.
            unsafe {
                let pixel = buffer.add(row * pitch + column * components);
                let gray = (u16::from(*pixel) * 11
                    + u16::from(*pixel.add(1)) * 59
                    + u16::from(*pixel.add(2)) * 30)
                    / 100;
                *pixel = gray as u8;
                *pixel.add(1) = gray as u8;
                *pixel.add(2) = gray as u8;
            }
        }
    }
    true
}

fn calculate_pitch_and_size(
    width: i32,
    height: i32,
    format: u16,
    requested_pitch: u32,
) -> Option<(u32, u32)> {
    let width = u32::try_from(width).ok().filter(|width| *width > 0)?;
    let height = u32::try_from(height).ok().filter(|height| *height > 0)?;
    let bits_per_pixel = u32::from(format & 0xff);
    if bits_per_pixel == 0 {
        return None;
    }

    let unaligned_bits = bits_per_pixel.checked_mul(width)?;
    let pitch = if requested_pitch == 0 {
        unaligned_bits.checked_add(31)?.checked_div(32)?.checked_mul(4)?
    } else {
        let minimum_pitch = unaligned_bits.checked_add(7)?.checked_div(8)?;
        if requested_pitch < minimum_pitch {
            return None;
        }
        requested_pitch
    };
    Some((pitch, pitch.checked_mul(height)?))
}

/// Calculates PDFium's bitmap pitch and allocation size with checked u32 math.
///
/// # Safety
///
/// `output_pitch` and `output_size` must be valid, non-overlapping writable
/// pointers to one `u32` each.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_calculate_pitch_and_size(
    width: i32,
    height: i32,
    format: u16,
    requested_pitch: u32,
    output_pitch: *mut u32,
    output_size: *mut u32,
) -> bool {
    if output_pitch.is_null() || output_size.is_null() {
        return false;
    }
    let Some((pitch, size)) = calculate_pitch_and_size(width, height, format, requested_pitch)
    else {
        return false;
    };
    // SAFETY: The caller contract guarantees two valid, non-overlapping
    // output locations.
    unsafe {
        *output_pitch = pitch;
        *output_size = size;
    }
    true
}

/// Expands packed 1-bpp source rows into an 8-bpp mask bitmap.
///
/// # Safety
///
/// Source and destination must cover `pitch * height` bytes for their
/// respective pitches and must not overlap. Source rows must contain at least
/// `ceil(width / 8)` bytes; destination rows at least `width` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_expand_1bpp_mask(
    source: *const u8,
    source_len: usize,
    source_pitch: usize,
    destination: *mut u8,
    destination_len: usize,
    destination_pitch: usize,
    width: usize,
    height: usize,
) -> bool {
    let Some(source_row_bytes) = width.checked_add(7).map(|value| value / 8) else {
        return false;
    };
    let Some(required_source_len) = source_pitch.checked_mul(height) else {
        return false;
    };
    let Some(required_destination_len) = destination_pitch.checked_mul(height) else {
        return false;
    };
    if source.is_null()
        || destination.is_null()
        || source_pitch < source_row_bytes
        || destination_pitch < width
        || required_source_len > source_len
        || required_destination_len > destination_len
    {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The validated pitches and lengths cover every computed
            // source bit and destination byte, and the regions do not overlap.
            unsafe {
                let source_byte = *source.add(row * source_pitch + column / 8);
                *destination.add(row * destination_pitch + column) =
                    if source_byte & (0x80 >> (column % 8)) == 0 { 0 } else { 255 };
            }
        }
    }
    true
}

/// Zeroes a destination bitmap and copies the shared prefix of each source row.
///
/// # Safety
///
/// Source and destination must cover `pitch * height` bytes for their
/// respective pitches and must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_populate_bitmap(
    source: *const u8,
    source_len: usize,
    source_pitch: usize,
    destination: *mut u8,
    destination_len: usize,
    destination_pitch: usize,
    height: usize,
) -> bool {
    let Some(required_source_len) = source_pitch.checked_mul(height) else {
        return false;
    };
    let Some(required_destination_len) = destination_pitch.checked_mul(height) else {
        return false;
    };
    if source.is_null()
        || destination.is_null()
        || required_source_len > source_len
        || required_destination_len > destination_len
    {
        return false;
    }
    // SAFETY: The caller contract guarantees the complete writable region.
    unsafe {
        std::ptr::write_bytes(destination, 0, destination_len);
    }
    let row_bytes = source_pitch.min(destination_pitch);
    for row in 0..height {
        // SAFETY: The validated lengths cover both row prefixes and the
        // source/destination regions do not overlap.
        unsafe {
            std::ptr::copy_nonoverlapping(
                source.add(row * source_pitch),
                destination.add(row * destination_pitch),
                row_bytes,
            );
        }
    }
    true
}

/// Copies one equal-format multi-byte bitmap row.
///
/// # Safety
///
/// Source and destination must cover their supplied lengths and must not
/// overlap. Offsets and width are expressed in pixels.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_transfer_bitmap_row(
    source: *const u8,
    source_len: usize,
    source_left: usize,
    destination: *mut u8,
    destination_len: usize,
    destination_left: usize,
    width: usize,
    components: usize,
) -> bool {
    if source.is_null() || destination.is_null() || components == 0 {
        return false;
    }
    let Some(source_offset) = source_left.checked_mul(components) else {
        return false;
    };
    let Some(destination_offset) = destination_left.checked_mul(components) else {
        return false;
    };
    let Some(copy_len) = width.checked_mul(components) else {
        return false;
    };
    let Some(source_end) = source_offset.checked_add(copy_len) else {
        return false;
    };
    let Some(destination_end) = destination_offset.checked_add(copy_len) else {
        return false;
    };
    if source_end > source_len || destination_end > destination_len {
        return false;
    }
    // SAFETY: The caller contract and checked ranges guarantee two complete,
    // non-overlapping row regions.
    unsafe {
        std::ptr::copy_nonoverlapping(
            source.add(source_offset),
            destination.add(destination_offset),
            copy_len,
        );
    }
    true
}

/// Copies one equal-format 1-bpp bitmap row at arbitrary bit offsets.
///
/// # Safety
///
/// Source and destination must cover their supplied lengths. Exact aliasing is
/// permitted and follows the C++ loop's left-to-right bit update order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_transfer_1bpp_row(
    source: *const u8,
    source_len: usize,
    source_left: usize,
    destination: *mut u8,
    destination_len: usize,
    destination_left: usize,
    width: usize,
) -> bool {
    if source.is_null() || destination.is_null() {
        return false;
    }
    let Some(source_end) = source_left.checked_add(width) else {
        return false;
    };
    let Some(destination_end) = destination_left.checked_add(width) else {
        return false;
    };
    let Some(source_rounded_end) = source_end.checked_add(7) else {
        return false;
    };
    let Some(destination_rounded_end) = destination_end.checked_add(7) else {
        return false;
    };
    if source_rounded_end / 8 > source_len || destination_rounded_end / 8 > destination_len {
        return false;
    }
    for column in 0..width {
        let source_bit = source_left + column;
        let destination_bit = destination_left + column;
        // SAFETY: The checked bit ranges cover both addressed bytes. Raw
        // pointer reads/writes preserve the reference loop's alias order.
        unsafe {
            let source_is_set = *source.add(source_bit / 8) & (1 << (7 - source_bit % 8)) != 0;
            let destination_byte = destination.add(destination_bit / 8);
            let mask = 1 << (7 - destination_bit % 8);
            if source_is_set {
                *destination_byte |= mask;
            } else {
                *destination_byte &= !mask;
            }
        }
    }
    true
}

#[derive(Clone, Copy)]
struct IntRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl IntRect {
    fn normalize(mut self) -> Self {
        if self.left > self.right {
            std::mem::swap(&mut self.left, &mut self.right);
        }
        if self.top > self.bottom {
            std::mem::swap(&mut self.top, &mut self.bottom);
        }
        self
    }

    fn intersect(self, other: Self) -> Self {
        let this = self.normalize();
        let other = other.normalize();
        let intersection = Self {
            left: this.left.max(other.left),
            top: this.top.max(other.top),
            right: this.right.min(other.right),
            bottom: this.bottom.min(other.bottom),
        };
        if intersection.left > intersection.right || intersection.top > intersection.bottom {
            Self { left: 0, top: 0, right: 0, bottom: 0 }
        } else {
            intersection
        }
    }

    fn is_empty(self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }
}

#[expect(clippy::too_many_arguments, reason = "C ABI mirrors PDFium's rectangle contract")]
fn overlap_rect(
    destination_width: i32,
    destination_height: i32,
    mut destination_left: i32,
    mut destination_top: i32,
    mut width: i32,
    mut height: i32,
    source_width: i32,
    source_height: i32,
    mut source_left: i32,
    mut source_top: i32,
    clip: Option<IntRect>,
) -> Option<[i32; 6]> {
    if width == 0
        || height == 0
        || destination_left > destination_width
        || destination_top > destination_height
    {
        return None;
    }
    let source_right = source_left.checked_add(width)?;
    let source_bottom = source_top.checked_add(height)?;
    let source_rect =
        IntRect { left: source_left, top: source_top, right: source_right, bottom: source_bottom }
            .intersect(IntRect { left: 0, top: 0, right: source_width, bottom: source_height });
    let x_offset = destination_left.checked_sub(source_left)?;
    let y_offset = destination_top.checked_sub(source_top)?;
    let destination_right = x_offset.checked_add(source_rect.right)?;
    let destination_bottom = y_offset.checked_add(source_rect.bottom)?;
    let mut destination_rect = IntRect {
        left: x_offset.checked_add(source_rect.left)?,
        top: y_offset.checked_add(source_rect.top)?,
        right: destination_right,
        bottom: destination_bottom,
    }
    .intersect(IntRect {
        left: 0,
        top: 0,
        right: destination_width,
        bottom: destination_height,
    });
    if let Some(clip) = clip {
        destination_rect = destination_rect.intersect(clip);
    }
    destination_left = destination_rect.left;
    destination_top = destination_rect.top;
    source_left = destination_left.checked_sub(x_offset)?;
    source_top = destination_top.checked_sub(y_offset)?;
    if destination_rect.is_empty() {
        return None;
    }
    width = destination_rect.right - destination_rect.left;
    height = destination_rect.bottom - destination_rect.top;
    Some([destination_left, destination_top, width, height, source_left, source_top])
}

/// Normalizes source/destination overlap and optional clipping rectangles.
///
/// The six output values are destination left/top, width/height, and source
/// left/top, matching `CFX_DIBBase::GetOverlapRect()`.
///
/// # Safety
///
/// `output` must point to six writable `i32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_get_overlap_rect(
    destination_width: i32,
    destination_height: i32,
    destination_left: i32,
    destination_top: i32,
    width: i32,
    height: i32,
    source_width: i32,
    source_height: i32,
    source_left: i32,
    source_top: i32,
    has_clip: bool,
    clip_left: i32,
    clip_top: i32,
    clip_right: i32,
    clip_bottom: i32,
    output: *mut i32,
) -> bool {
    if output.is_null() {
        return false;
    }
    let clip = has_clip.then_some(IntRect {
        left: clip_left,
        top: clip_top,
        right: clip_right,
        bottom: clip_bottom,
    });
    let Some(result) = overlap_rect(
        destination_width,
        destination_height,
        destination_left,
        destination_top,
        width,
        height,
        source_width,
        source_height,
        source_left,
        source_top,
        clip,
    ) else {
        return false;
    };
    // SAFETY: The caller contract guarantees six writable output values.
    unsafe {
        std::ptr::copy_nonoverlapping(result.as_ptr(), output, result.len());
    }
    true
}

fn default_palette_argb(bits_per_pixel: u8, index: usize) -> Option<u32> {
    match bits_per_pixel {
        1 if index < 2 => Some(if index == 0 { 0xff00_0000 } else { 0xffff_ffff }),
        8 if index < 256 => Some(0xff00_0000 | ((index as u32) * 0x0001_0101)),
        _ => None,
    }
}

/// Fills PDFium's default 1-bpp or 8-bpp ARGB palette.
///
/// # Safety
///
/// `output` must point to `output_len` writable `u32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_default_palette(
    bits_per_pixel: u8,
    output: *mut u32,
    output_len: usize,
) -> bool {
    let required_len = match bits_per_pixel {
        1 => 2,
        8 => 256,
        _ => return false,
    };
    if output.is_null() || output_len < required_len {
        return false;
    }
    for index in 0..required_len {
        let Some(argb) = default_palette_argb(bits_per_pixel, index) else {
            return false;
        };
        // SAFETY: The caller contract and length check cover every output.
        unsafe {
            *output.add(index) = argb;
        }
    }
    true
}

/// Returns a default palette entry for a 1-bpp or 8-bpp bitmap.
///
/// # Safety
///
/// `output` must point to one writable `u32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_get_default_palette_argb(
    bits_per_pixel: u8,
    index: usize,
    output: *mut u32,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(argb) = default_palette_argb(bits_per_pixel, index) else {
        return false;
    };
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = argb;
    }
    true
}

/// Finds an exact ARGB color in an explicit or default palette.
///
/// # Safety
///
/// When `palette_len` is non-zero, `palette` must cover that many `u32`
/// entries. `output` must point to one writable `i32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_find_palette(
    bits_per_pixel: u8,
    palette: *const u32,
    palette_len: usize,
    color: u32,
    output: *mut i32,
) -> bool {
    if output.is_null() || !matches!(bits_per_pixel, 1 | 8) {
        return false;
    }
    let result = if palette_len == 0 {
        if bits_per_pixel == 1 {
            i32::from(color as u8 == 255)
        } else {
            i32::from(color as u8)
        }
    } else {
        let required_len = 1usize << bits_per_pixel;
        if palette.is_null() || palette_len < required_len {
            return false;
        }
        // SAFETY: The caller contract and length check cover the required
        // palette entries.
        let palette = unsafe { slice::from_raw_parts(palette, required_len) };
        palette.iter().position(|entry| *entry == color).map_or(-1, |index| index as i32)
    };
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = result;
    }
    true
}

fn palette_bgr(palette: &[u32], index: usize) -> Option<[u8; 3]> {
    let argb = *palette.get(index)?;
    Some([argb as u8, (argb >> 8) as u8, (argb >> 16) as u8])
}

fn convert_buffer_row(
    destination_format: u16,
    source_format: u16,
    source: &[u8],
    source_left: usize,
    palette: &[u32],
    output: &mut [u8],
    width: usize,
) -> bool {
    let source_bits = usize::from(source_format & 0xff);
    let destination_components = match destination_format {
        0x008 | 0x108 => 1,
        0x018 => 3,
        0x020 | 0x220 => 4,
        _ => return false,
    };
    let Some(required_output) = width.checked_mul(destination_components) else {
        return false;
    };
    if output.len() < required_output {
        return false;
    }
    let palette_required = if palette.is_empty() {
        0
    } else {
        match source_bits {
            1 => 2,
            8 => 256,
            _ => return false,
        }
    };
    if palette.len() < palette_required {
        return false;
    }

    for pixel in 0..width {
        let Some(source_pixel) = source_left.checked_add(pixel) else {
            return false;
        };
        let index = match source_bits {
            1 => {
                let Some(byte) = source.get(source_pixel / 8) else {
                    return false;
                };
                usize::from((byte >> (7 - source_pixel % 8)) & 1)
            }
            8 => {
                let Some(value) = source.get(source_pixel) else {
                    return false;
                };
                usize::from(*value)
            }
            24 | 32 => source_pixel,
            _ => return false,
        };

        if destination_components == 1 {
            output[pixel] = match source_bits {
                1 if !palette.is_empty() && destination_format == 0x008 => {
                    if index == 0 {
                        255
                    } else {
                        0
                    }
                }
                1 => {
                    if index == 0 {
                        0
                    } else {
                        255
                    }
                }
                8 if palette.is_empty() || destination_format == 0x008 => index as u8,
                8 => {
                    let Some(bgr) = palette_bgr(palette, index) else {
                        return false;
                    };
                    gray([bgr[0], bgr[1], bgr[2], 255])
                }
                24 | 32 => {
                    let components = source_bits / 8;
                    let Some(offset) = index.checked_mul(components) else {
                        return false;
                    };
                    let Some(end) = offset.checked_add(3) else {
                        return false;
                    };
                    let Some(pixel) = source.get(offset..end) else {
                        return false;
                    };
                    gray([pixel[0], pixel[1], pixel[2], 255])
                }
                _ => return false,
            };
            continue;
        }

        let bgr = match source_bits {
            1 | 8 if !palette.is_empty() => {
                let Some(bgr) = palette_bgr(palette, index) else {
                    return false;
                };
                bgr
            }
            1 => {
                let value = if index == 0 { 0 } else { 255 };
                [value, value, value]
            }
            8 => {
                let value = index as u8;
                [value, value, value]
            }
            24 | 32 => {
                let components = source_bits / 8;
                let Some(offset) = index.checked_mul(components) else {
                    return false;
                };
                let Some(end) = offset.checked_add(3) else {
                    return false;
                };
                let Some(pixel) = source.get(offset..end) else {
                    return false;
                };
                [pixel[0], pixel[1], pixel[2]]
            }
            _ => return false,
        };
        let destination = pixel * destination_components;
        output[destination..destination + 3].copy_from_slice(&bgr);
    }
    true
}

/// Converts one DIB row between the retained mask/index/RGB formats.
///
/// Four-component destinations intentionally leave their fourth byte
/// untouched, matching the retained conversion loops.
///
/// # Safety
///
/// Source, palette, and output pointers must cover their supplied lengths.
/// Source and output must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_convert_buffer_row(
    destination_format: u16,
    source_format: u16,
    source: *const u8,
    source_len: usize,
    source_left: usize,
    palette: *const u32,
    palette_len: usize,
    output: *mut u8,
    output_len: usize,
    width: usize,
) -> bool {
    if source.is_null() || output.is_null() || (palette_len != 0 && palette.is_null()) {
        return false;
    }
    // SAFETY: The caller contract guarantees all three regions for the
    // supplied lengths and forbids source/output overlap.
    let (source, palette, output) = unsafe {
        (
            slice::from_raw_parts(source, source_len),
            if palette_len == 0 { &[] } else { slice::from_raw_parts(palette, palette_len) },
            slice::from_raw_parts_mut(output, output_len),
        )
    };
    convert_buffer_row(
        destination_format,
        source_format,
        source,
        source_left,
        palette,
        output,
        width,
    )
}

/// Copies each packed BGRA pixel's alpha channel into its red channel.
///
/// # Safety
///
/// `buffer` must be valid for `pitch * height` writable bytes. Each row must
/// contain at least `width * 4` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bgra_set_red_from_alpha(
    buffer: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
) -> bool {
    if buffer.is_null() || !packed_rows_are_valid(width, height, pitch, 4) {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee a
            // complete BGRA pixel at every computed row/column offset.
            unsafe {
                let pixel = buffer.add(row * pitch + column * 4);
                *pixel.add(2) = *pixel.add(3);
            }
        }
    }
    true
}

/// Sets every packed BGRA pixel's alpha channel to fully opaque.
///
/// # Safety
///
/// `buffer` must be valid for `pitch * height` writable bytes. Each row must
/// contain at least `width * 4` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bgra_set_opaque_alpha(
    buffer: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
) -> bool {
    if buffer.is_null() || !packed_rows_are_valid(width, height, pitch, 4) {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee a
            // complete BGRA pixel at every computed row/column offset.
            unsafe {
                *buffer.add(row * pitch + column * 4 + 3) = 255;
            }
        }
    }
    true
}

/// Multiplies each packed BGRA alpha by the corresponding 8-bit mask value.
///
/// # Safety
///
/// `buffer` and `mask` must cover their respective `pitch * height` regions,
/// with at least `width * 4` BGRA bytes and `width` mask bytes per row. The
/// regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bgra_multiply_alpha_mask(
    buffer: *mut u8,
    buffer_pitch: usize,
    mask: *const u8,
    mask_pitch: usize,
    width: usize,
    height: usize,
) -> bool {
    if buffer.is_null()
        || mask.is_null()
        || !packed_rows_are_valid(width, height, buffer_pitch, 4)
        || !packed_rows_are_valid(width, height, mask_pitch, 1)
    {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee both
            // addressed bytes, and the two regions do not overlap.
            unsafe {
                let alpha = buffer.add(row * buffer_pitch + column * 4 + 3);
                *alpha = (u16::from(*alpha) * u16::from(*mask.add(row * mask_pitch + column)) / 255)
                    as u8;
            }
        }
    }
    true
}

/// Multiplies every packed BGRA alpha by one 8-bit factor.
///
/// # Safety
///
/// `buffer` must be valid for `pitch * height` writable bytes. Each row must
/// contain at least `width * 4` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_bgra_multiply_alpha(
    buffer: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    alpha: u8,
) -> bool {
    if buffer.is_null() || !packed_rows_are_valid(width, height, pitch, 4) {
        return false;
    }
    for row in 0..height {
        for column in 0..width {
            // SAFETY: The caller contract and validation above guarantee the
            // addressed alpha byte at every computed row/column offset.
            unsafe {
                let destination = buffer.add(row * pitch + column * 4 + 3);
                *destination = (u16::from(*destination) * u16::from(alpha) / 255) as u8;
            }
        }
    }
    true
}

/// Converts one Adobe CMYK sample to standard RGB.
///
/// # Safety
///
/// Each output pointer must be valid for one writable byte and the output
/// locations must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_adobe_cmyk_to_rgb(
    cyan: u8,
    magenta: u8,
    yellow: u8,
    key: u8,
    red: *mut u8,
    green: *mut u8,
    blue: *mut u8,
) -> bool {
    if red.is_null() || green.is_null() || blue.is_null() {
        return false;
    }
    let rgb = adobe_cmyk_to_standard_rgb(cyan, magenta, yellow, key);
    // SAFETY: The caller contract guarantees three valid, non-overlapping
    // output locations.
    unsafe {
        *red = rgb[0];
        *green = rgb[1];
        *blue = rgb[2];
    }
    true
}

/// Converts a packed CMYK row to the packed BGR order expected by PDFium.
///
/// # Safety
///
/// `source` must be valid for `pixel_count * 4` bytes and `output` for
/// `pixel_count * 3` writable bytes. The regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_adobe_cmyk_to_bgr_row(
    source: *const u8,
    output: *mut u8,
    pixel_count: usize,
) -> bool {
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees complete, non-overlapping
        // packed source and output rows at every computed offset.
        unsafe {
            let source = source.add(pixel * 4);
            let rgb =
                adobe_cmyk_to_standard_rgb(*source, *source.add(1), *source.add(2), *source.add(3));
            let output = output.add(pixel * 3);
            *output = rgb[2];
            *output.add(1) = rgb[1];
            *output.add(2) = rgb[0];
        }
    }
    true
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
// RUST_PORT_METRICS_END abi_thunk

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

/// Composites a packed BGRA row into packed BGR or BGRx output.
///
/// # Safety
///
/// `source` must be valid for `pixel_count * 4` bytes and `output` for
/// `pixel_count * output_components` bytes. `output_components` must be 3 or 4.
/// `clip` must be null or valid for `pixel_count` bytes. The regions must not
/// overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_bgra_to_bgr_row(
    mode: u8,
    source: *const u8,
    clip: *const u8,
    output: *mut u8,
    output_components: usize,
    rgb_byte_order: bool,
    pixel_count: usize,
) -> bool {
    let Ok(mode) = BlendMode::try_from(mode) else {
        return false;
    };
    if !matches!(output_components, 3 | 4) {
        return false;
    }
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees complete source and output
        // pixels at each computed offset and non-overlapping regions.
        unsafe {
            let input = (source.add(pixel * 4) as *const [u8; 4]).read_unaligned();
            let output =
                slice::from_raw_parts_mut(output.add(pixel * output_components), output_components);
            let clip = if clip.is_null() { 255 } else { *clip.add(pixel) };
            composite_bgra_to_bgr_pixel(mode, input, clip, rgb_byte_order, output);
        }
    }
    true
}

/// Composites packed BGRA pixels into an 8-bit gray or alpha-mask row.
///
/// # Safety
///
/// `source` must be valid for `pixel_count * 4` bytes and `output` for
/// `pixel_count` bytes. `clip` must be null or valid for `pixel_count` bytes.
/// The source and output regions must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_bgra_to_byte_row(
    mode: u8,
    source: *const u8,
    clip: *const u8,
    output: *mut u8,
    is_mask: bool,
    pixel_count: usize,
) -> bool {
    if !is_mask && mode > 15 {
        return false;
    }
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees complete non-overlapping
        // source and output rows and an optional complete clip row.
        unsafe {
            let input = (source.add(pixel * 4) as *const [u8; 4]).read_unaligned();
            let clip = if clip.is_null() { 255 } else { *clip.add(pixel) };
            let mut destination = *output.add(pixel);
            if !composite_bgra_to_byte_pixel(mode, input, clip, is_mask, &mut destination) {
                return false;
            }
            *output.add(pixel) = destination;
        }
    }
    true
}

/// Composites an opaque BGR/BGRx source row into a supported destination row.
///
/// Target values are 0=gray, 1=mask, 2=BGR/BGRx, and 3=BGRA.
///
/// # Safety
///
/// Source and output must be valid for `pixel_count` packed pixels using the
/// supplied component counts. `clip` must be null or valid for `pixel_count`
/// bytes. Source and output must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_opaque_row(
    mode: u8,
    source: *const u8,
    source_components: usize,
    clip: *const u8,
    output: *mut u8,
    output_components: usize,
    target: u8,
    rgb_byte_order: bool,
    pixel_count: usize,
) -> bool {
    if !matches!(source_components, 3 | 4)
        || !matches!((target, output_components), (0 | 1, 1) | (2, 3 | 4) | (3, 4))
    {
        return false;
    }
    if target == 2
        && mode == BlendMode::Normal as u8
        && clip.is_null()
        && !rgb_byte_order
        && source_components == output_components
    {
        // SAFETY: The caller contract guarantees complete, non-overlapping
        // source/output rows. This preserves the C++ fast path, including the
        // unused fourth byte in BGRx pixels.
        unsafe {
            std::ptr::copy_nonoverlapping(source, output, pixel_count * source_components);
        }
        return true;
    }
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees complete, non-overlapping
        // source/output rows and an optional complete clip row.
        unsafe {
            let source = source.add(pixel * source_components);
            let input = [*source, *source.add(1), *source.add(2), 255];
            let output =
                slice::from_raw_parts_mut(output.add(pixel * output_components), output_components);
            let clip = if clip.is_null() { 255 } else { *clip.add(pixel) };
            if !composite_opaque_pixel(mode, input, clip, target, rgb_byte_order, output) {
                return false;
            }
        }
    }
    true
}

/// Composites an 8-bit or 1-bit mask row using a constant BGRA mask color.
///
/// Target values match `pdfium_rust_composite_opaque_row()`.
///
/// # Safety
///
/// `source` must cover every addressed byte, `output` must cover
/// `pixel_count` packed destination pixels, and `clip` must be null or valid
/// for `pixel_count` bytes. Source and output must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_mask_row(
    mode: u8,
    source: *const u8,
    source_left: usize,
    source_is_bit_mask: bool,
    clip: *const u8,
    mask_blue: u8,
    mask_green: u8,
    mask_red: u8,
    mask_alpha: u8,
    output: *mut u8,
    output_components: usize,
    target: u8,
    rgb_byte_order: bool,
    pixel_count: usize,
) -> bool {
    if !matches!((target, output_components), (0 | 1, 1) | (2, 3 | 4) | (3, 4)) {
        return false;
    }
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees that each addressed source,
        // clip, and output byte is within its corresponding region.
        unsafe {
            let source_value = if source_is_bit_mask {
                let bit = source_left + pixel;
                if *source.add(bit / 8) & (1 << (7 - bit % 8)) == 0 {
                    0
                } else {
                    255
                }
            } else {
                *source.add(pixel)
            };
            let mut coverage = u32::from(mask_alpha) * u32::from(source_value);
            if !clip.is_null() {
                coverage *= u32::from(*clip.add(pixel));
                coverage /= 255;
            }
            coverage /= 255;
            let input = [mask_blue, mask_green, mask_red, 255];
            let output =
                slice::from_raw_parts_mut(output.add(pixel * output_components), output_components);
            let effective_mode = if target <= 1 { BlendMode::Normal as u8 } else { mode };
            if target == 3 && matches!(mode, 12..=15) {
                let Ok(mode) = BlendMode::try_from(mode) else {
                    return false;
                };
                let mut destination = if rgb_byte_order {
                    [output[2], output[1], output[0], output[3]]
                } else {
                    [output[0], output[1], output[2], output[3]]
                };
                composite_mask_bgra_pixel(mode, input, coverage as u8, &mut destination);
                if rgb_byte_order {
                    output[..3].copy_from_slice(&[destination[2], destination[1], destination[0]]);
                } else {
                    output[..3].copy_from_slice(&destination[..3]);
                }
                output[3] = destination[3];
                continue;
            }
            if !composite_opaque_pixel(
                effective_mode,
                input,
                coverage as u8,
                target,
                rgb_byte_order,
                output,
            ) {
                return false;
            }
        }
    }
    true
}

/// Composites a 1-bit or 8-bit indexed row using optional gray/ARGB palettes.
///
/// # Safety
///
/// Source, palette, clip, and output pointers must cover the lengths supplied
/// by the caller. Source and output must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_composite_palette_row(
    mode: u8,
    source: *const u8,
    source_left: usize,
    source_is_bit: bool,
    gray_palette: *const u8,
    gray_palette_len: usize,
    argb_palette: *const u32,
    argb_palette_len: usize,
    clip: *const u8,
    output: *mut u8,
    output_components: usize,
    target: u8,
    rgb_byte_order: bool,
    pixel_count: usize,
) -> bool {
    if !matches!((target, output_components), (0 | 1, 1) | (2, 3 | 4) | (3, 4)) {
        return false;
    }
    for pixel in 0..pixel_count {
        // SAFETY: The caller contract guarantees each addressed source,
        // palette, clip, and output element.
        unsafe {
            let index = if source_is_bit {
                let bit = source_left + pixel;
                usize::from((*source.add(bit / 8) >> (7 - bit % 8)) & 1)
            } else {
                usize::from(*source.add(pixel))
            };
            let input = if target == 0 {
                let value = if index < gray_palette_len {
                    *gray_palette.add(index)
                } else if source_is_bit {
                    if index == 0 {
                        0
                    } else {
                        255
                    }
                } else {
                    index as u8
                };
                [value, value, value, 255]
            } else if target == 1 {
                [0, 0, 0, 255]
            } else {
                let argb = if index < argb_palette_len {
                    *argb_palette.add(index)
                } else {
                    let value = if source_is_bit && index != 0 { 255 } else { index as u8 };
                    u32::from(value) * 0x00010101
                };
                [argb as u8, (argb >> 8) as u8, (argb >> 16) as u8, 255]
            };
            let clip = if clip.is_null() { 255 } else { *clip.add(pixel) };
            let output =
                slice::from_raw_parts_mut(output.add(pixel * output_components), output_components);
            let effective_mode = if target >= 2 { 0 } else { mode };
            if !composite_opaque_pixel(effective_mode, input, clip, target, rgb_byte_order, output)
            {
                return false;
            }
        }
    }
    true
}
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
    fn rgb_blend_should_preserve_non_separable_reference_case() {
        assert_eq!([0, 123, 49], rgb_blend(BlendMode::Hue, [0, 255, 100, 255], [255, 100, 0, 255]));
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

    #[test]
    fn cmyk_should_preserve_reference_endpoints() {
        assert_eq!([255, 255, 255], adobe_cmyk_to_standard_rgb(0, 0, 0, 0));
        assert_eq!([0, 0, 0], adobe_cmyk_to_standard_rgb(255, 255, 255, 255));
    }

    #[test]
    fn pitch_and_size_should_align_rows_to_four_bytes() {
        assert_eq!(Some((4, 12)), calculate_pitch_and_size(3, 3, 0x008, 0));
        assert_eq!(Some((12, 36)), calculate_pitch_and_size(3, 3, 0x018, 0));
    }

    #[test]
    fn pitch_and_size_should_reject_overflow_and_short_pitch() {
        assert_eq!(None, calculate_pitch_and_size(101, 200, 0x220, 400));
        assert_eq!(None, calculate_pitch_and_size(1_073_747_000, 1, 0x220, 0));
    }

    #[test]
    fn transfer_bitmap_row_should_respect_pixel_offsets() {
        let source = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let mut destination = [99; 12];
        // SAFETY: The stack arrays are valid, distinct regions with the exact
        // lengths supplied to the FFI function.
        assert!(unsafe {
            pdfium_rust_transfer_bitmap_row(
                source.as_ptr(),
                source.len(),
                1,
                destination.as_mut_ptr(),
                destination.len(),
                2,
                2,
                3,
            )
        });
        assert_eq!([99, 99, 99, 99, 99, 99, 3, 4, 5, 6, 7, 8], destination);
    }

    #[test]
    fn transfer_1bpp_row_should_preserve_surrounding_bits() {
        let source = [0b1011_0110];
        let mut destination = [0b0100_0001];
        // SAFETY: The stack arrays cover the single-byte source/destination
        // regions supplied to the FFI function.
        assert!(unsafe {
            pdfium_rust_transfer_1bpp_row(
                source.as_ptr(),
                source.len(),
                2,
                destination.as_mut_ptr(),
                destination.len(),
                1,
                4,
            )
        });
        assert_eq!([0b0110_1001], destination);
    }

    #[test]
    fn overlap_rect_should_apply_source_destination_and_clip_bounds() {
        let clip = IntRect { left: 0, top: 30, right: 50, bottom: 90 };
        assert_eq!(
            Some([0, 30, 50, 60, 15, 20]),
            overlap_rect(400, 300, -10, 20, 100, 80, 200, 200, 5, 10, Some(clip))
        );
    }

    #[test]
    fn default_palette_should_preserve_pdfium_argb_values() {
        assert_eq!(Some(0xff00_0000), default_palette_argb(1, 0));
        assert_eq!(Some(0xffff_ffff), default_palette_argb(1, 1));
        assert_eq!(Some(0xffad_adad), default_palette_argb(8, 0xad));
    }

    #[test]
    fn buffer_conversion_should_map_custom_palette_to_bgr() {
        let source = [0, 1, 2];
        let mut palette = [0; 256];
        palette[0] = 0xff11_2233;
        palette[1] = 0xff44_5566;
        palette[2] = 0xff77_8899;
        let mut output = [0; 9];

        assert!(convert_buffer_row(0x018, 0x108, &source, 0, &palette, &mut output, 3,));
        assert_eq!([0x33, 0x22, 0x11, 0x66, 0x55, 0x44, 0x99, 0x88, 0x77], output);
    }

    #[test]
    fn buffer_conversion_should_preserve_fourth_destination_byte() {
        let source = [0x13, 0x27, 0x42];
        let mut output = [0, 0, 0, 0xad];

        assert!(convert_buffer_row(0x220, 0x018, &source, 0, &[], &mut output, 1,));
        assert_eq!([0x13, 0x27, 0x42, 0xad], output);
    }
}
