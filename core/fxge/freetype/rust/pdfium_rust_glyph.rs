// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

const MAX_GLYPH_CACHE_KEY_WORDS: usize = 10;

struct GlyphCacheKeyInputs {
    matrix: [i32; 4],
    destination_width: i32,
    anti_alias: i32,
    has_substitution: bool,
    weight: i32,
    italic_angle: i32,
    vertical: bool,
    native_text: bool,
}

struct GlyphCacheKey {
    words: [u32; MAX_GLYPH_CACHE_KEY_WORDS],
    len: usize,
}

struct GlyphOriginPlan {
    x: i32,
    y: i32,
}

fn plan_glyph_origin(
    origin_x: i32,
    origin_y: i32,
    glyph_left: i32,
    glyph_top: i32,
    offset_x: i32,
    offset_y: i32,
) -> Option<GlyphOriginPlan> {
    Some(GlyphOriginPlan {
        x: origin_x.checked_add(glyph_left)?.checked_sub(offset_x)?,
        y: origin_y.checked_sub(glyph_top)?.checked_sub(offset_y)?,
    })
}

fn push_word(key: &mut GlyphCacheKey, value: i32) -> bool {
    let Some(output) = key.words.get_mut(key.len) else {
        return false;
    };
    *output = value as u32;
    key.len += 1;
    true
}

fn build_glyph_cache_key(inputs: GlyphCacheKeyInputs) -> Option<GlyphCacheKey> {
    let mut key = GlyphCacheKey { words: [0; MAX_GLYPH_CACHE_KEY_WORDS], len: 0 };
    for value in inputs.matrix {
        if !push_word(&mut key, value) {
            return None;
        }
    }
    if !push_word(&mut key, inputs.destination_width) || !push_word(&mut key, inputs.anti_alias) {
        return None;
    }
    if inputs.has_substitution
        && (!push_word(&mut key, inputs.weight)
            || !push_word(&mut key, inputs.italic_angle)
            || !push_word(&mut key, i32::from(inputs.vertical)))
    {
        return None;
    }
    if inputs.native_text && !push_word(&mut key, 3) {
        return None;
    }
    Some(key)
}

/// Builds the exact word sequence used as the C++ glyph-bitmap cache key.
///
/// # Safety
///
/// `output_len` must point to one writable `usize`. `output` must point to at
/// least `output_capacity` writable `u32` values, and the capacity must fit the
/// selected key shape.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_fill_glyph_cache_key(
    matrix_a: i32,
    matrix_b: i32,
    matrix_c: i32,
    matrix_d: i32,
    destination_width: i32,
    anti_alias: i32,
    has_substitution: bool,
    weight: i32,
    italic_angle: i32,
    vertical: bool,
    native_text: bool,
    output: *mut u32,
    output_capacity: usize,
    output_len: *mut usize,
) -> bool {
    if output_len.is_null() {
        return false;
    }
    let Some(key) = build_glyph_cache_key(GlyphCacheKeyInputs {
        matrix: [matrix_a, matrix_b, matrix_c, matrix_d],
        destination_width,
        anti_alias,
        has_substitution,
        weight,
        italic_angle,
        vertical,
        native_text,
    }) else {
        return false;
    };
    if output_capacity < key.len || (key.len != 0 && output.is_null()) {
        return false;
    }
    // SAFETY: The caller guarantees a writable span of `output_capacity`
    // words, and the capacity was checked against the selected key length.
    let output = unsafe { core::slice::from_raw_parts_mut(output, output_capacity) };
    let Some(destination) = output.get_mut(..key.len) else {
        return false;
    };
    let Some(source) = key.words.get(..key.len) else {
        return false;
    };
    destination.copy_from_slice(source);
    // SAFETY: The caller guarantees one writable length value.
    unsafe {
        *output_len = key.len;
    }
    true
}

/// Plans the checked integer origin of one rendered glyph bitmap.
///
/// A valid boundary call succeeds even when the arithmetic overflows; in that
/// case `output_valid` is zero and both coordinate outputs are zero.
///
/// # Safety
///
/// All three output pointers must point to writable values of their respective
/// types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_plan_glyph_origin(
    origin_x: i32,
    origin_y: i32,
    glyph_left: i32,
    glyph_top: i32,
    offset_x: i32,
    offset_y: i32,
    output_valid: *mut u8,
    output_x: *mut i32,
    output_y: *mut i32,
) -> bool {
    if output_valid.is_null() || output_x.is_null() || output_y.is_null() {
        return false;
    }
    let plan = plan_glyph_origin(origin_x, origin_y, glyph_left, glyph_top, offset_x, offset_y);
    let (valid, x, y) = plan.map_or((0, 0, 0), |value| (1, value.x, value.y));
    // SAFETY: The caller guarantees that all three checked output pointers are
    // live and writable for one value.
    unsafe {
        *output_valid = valid;
        *output_x = x;
        *output_y = y;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> GlyphCacheKeyInputs {
        GlyphCacheKeyInputs {
            matrix: [10, -20, 30, -40],
            destination_width: 500,
            anti_alias: 2,
            has_substitution: false,
            weight: 0,
            italic_angle: 0,
            vertical: false,
            native_text: false,
        }
    }

    #[test]
    fn cache_key_should_preserve_base_and_signed_word_representation() {
        let Some(key) = build_glyph_cache_key(inputs()) else {
            panic!("fixed base key must fit the bounded output");
        };
        assert_eq!(key.len, 6);
        assert_eq!(&key.words[..key.len], &[10, (-20_i32) as u32, 30, (-40_i32) as u32, 500, 2]);
    }

    #[test]
    fn cache_key_should_append_substitution_and_native_discriminators() {
        let Some(key) = build_glyph_cache_key(GlyphCacheKeyInputs {
            has_substitution: true,
            weight: 700,
            italic_angle: -12,
            vertical: true,
            native_text: true,
            ..inputs()
        }) else {
            panic!("maximum supported key must fit the bounded output");
        };
        assert_eq!(key.len, MAX_GLYPH_CACHE_KEY_WORDS);
        assert_eq!(
            key.words,
            [10, (-20_i32) as u32, 30, (-40_i32) as u32, 500, 2, 700, (-12_i32) as u32, 1, 3,]
        );
    }

    #[test]
    fn cache_key_ffi_should_reject_invalid_boundaries_without_mutation() {
        let mut output = [0xfeed_beef_u32; MAX_GLYPH_CACHE_KEY_WORDS];
        let mut output_len = 99;
        // SAFETY: The deliberately short output is live and writable; the
        // boundary rejects it before writing either output.
        assert!(!unsafe {
            pdfium_rust_fill_glyph_cache_key(
                1,
                2,
                3,
                4,
                5,
                0,
                true,
                400,
                0,
                false,
                true,
                output.as_mut_ptr(),
                9,
                &mut output_len,
            )
        });
        assert_eq!(output, [0xfeed_beef; MAX_GLYPH_CACHE_KEY_WORDS]);
        assert_eq!(output_len, 99);
        // SAFETY: A null length output is explicitly rejected before the
        // otherwise valid word span is written.
        assert!(!unsafe {
            pdfium_rust_fill_glyph_cache_key(
                1,
                2,
                3,
                4,
                5,
                0,
                false,
                0,
                0,
                false,
                false,
                output.as_mut_ptr(),
                output.len(),
                core::ptr::null_mut(),
            )
        });
    }
}
