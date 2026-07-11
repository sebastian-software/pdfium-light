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

#[derive(Clone, Copy)]
struct GlyphBoundsInput {
    valid: bool,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
}

#[derive(Clone, Copy)]
struct GlyphBoundsPlan {
    started: bool,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

type ReadGlyphBounds = unsafe extern "C" fn(
    context: *mut core::ffi::c_void,
    index: usize,
    output_valid: *mut u8,
    output_left: *mut i32,
    output_top: *mut i32,
    output_width: *mut i32,
    output_height: *mut i32,
) -> bool;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum GlyphBitmapLookupAction {
    Reject = 0,
    LookupRequestedKey = 1,
    ReturnNativeCached = 2,
    LookupNonNativeAndDisableNative = 3,
}

fn plan_glyph_bitmap_lookup(
    glyph_is_valid: bool,
    native_text: bool,
    native_cache_hit: bool,
) -> GlyphBitmapLookupAction {
    if !glyph_is_valid {
        GlyphBitmapLookupAction::Reject
    } else if !native_text {
        GlyphBitmapLookupAction::LookupRequestedKey
    } else if native_cache_hit {
        GlyphBitmapLookupAction::ReturnNativeCached
    } else {
        GlyphBitmapLookupAction::LookupNonNativeAndDisableNative
    }
}

fn include_glyph_bounds(
    mut plan: GlyphBoundsPlan,
    input: GlyphBoundsInput,
    anti_alias_is_lcd: bool,
) -> GlyphBoundsPlan {
    if !input.valid {
        return plan;
    }
    let width = if anti_alias_is_lcd { input.width / 3 } else { input.width };
    let Some(right) = input.left.checked_add(width) else {
        return plan;
    };
    let Some(bottom) = input.top.checked_add(input.height) else {
        return plan;
    };
    if !plan.started {
        return GlyphBoundsPlan { started: true, left: input.left, top: input.top, right, bottom };
    }
    plan.left = plan.left.min(input.left);
    plan.top = plan.top.min(input.top);
    plan.right = plan.right.max(right);
    plan.bottom = plan.bottom.max(bottom);
    plan
}

fn round_like_fxsys(value: f32) -> i32 {
    if value.is_nan() {
        return 0;
    }
    if value < i32::MIN as f32 {
        return i32::MIN;
    }
    if value >= i32::MAX as f32 {
        return i32::MAX;
    }
    value.round() as i32
}

fn plan_glyph_device_origin(
    device_x: f32,
    device_y: f32,
    anti_alias_is_lcd: bool,
) -> Option<GlyphOriginPlan> {
    let x = if anti_alias_is_lcd {
        if !device_x.is_finite() || device_x < i32::MIN as f32 || device_x >= i32::MAX as f32 {
            return None;
        }
        device_x.floor() as i32
    } else {
        round_like_fxsys(device_x)
    };
    Some(GlyphOriginPlan { x, y: round_like_fxsys(device_y) })
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

/// Plans the integer device origin used to place one glyph bitmap.
///
/// LCD x coordinates use floor conversion; other coordinates use the PDFium
/// saturated, half-away-from-zero rounding contract. Non-finite or out-of-range
/// LCD x values are rejected so C++ can retain its platform conversion oracle.
///
/// # Safety
///
/// Both output pointers must point to writable `i32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_plan_glyph_device_origin(
    device_x: f32,
    device_y: f32,
    anti_alias_is_lcd: bool,
    output_x: *mut i32,
    output_y: *mut i32,
) -> bool {
    if output_x.is_null() || output_y.is_null() {
        return false;
    }
    let Some(plan) = plan_glyph_device_origin(device_x, device_y, anti_alias_is_lcd) else {
        return false;
    };
    // SAFETY: The caller guarantees two live, writable output values.
    unsafe {
        *output_x = plan.x;
        *output_y = plan.y;
    }
    true
}

/// Aggregates the checked bitmap bounds for an ordered borrowed glyph list.
///
/// The callback supplies one transient set of scalar bounds at a time. Missing
/// glyphs and overflowing right/bottom edges are skipped exactly like the C++
/// oracle; an empty result is the all-zero rectangle.
///
/// # Safety
///
/// `read_bounds`, when present, must be safe to call `glyph_count` times with
/// `context` and writable scalar outputs. All rectangle outputs must point to
/// writable `i32` values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_plan_glyph_bounds(
    glyph_count: usize,
    anti_alias_is_lcd: bool,
    context: *mut core::ffi::c_void,
    read_bounds: Option<ReadGlyphBounds>,
    output_left: *mut i32,
    output_top: *mut i32,
    output_right: *mut i32,
    output_bottom: *mut i32,
) -> bool {
    if output_left.is_null()
        || output_top.is_null()
        || output_right.is_null()
        || output_bottom.is_null()
        || (glyph_count != 0 && read_bounds.is_none())
    {
        return false;
    }
    let mut plan = GlyphBoundsPlan { started: false, left: 0, top: 0, right: 0, bottom: 0 };
    for index in 0..glyph_count {
        let mut valid = 0_u8;
        let mut left = 0;
        let mut top = 0;
        let mut width = 0;
        let mut height = 0;
        let Some(callback) = read_bounds else {
            return false;
        };
        // SAFETY: The caller guarantees the callback/context contract, and all
        // scalar outputs are live for this invocation.
        if !unsafe {
            callback(context, index, &mut valid, &mut left, &mut top, &mut width, &mut height)
        } || valid > 1
        {
            return false;
        }
        plan = include_glyph_bounds(
            plan,
            GlyphBoundsInput { valid: valid != 0, left, top, width, height },
            anti_alias_is_lcd,
        );
    }
    // SAFETY: The caller guarantees four live, writable rectangle outputs.
    unsafe {
        *output_left = plan.left;
        *output_top = plan.top;
        *output_right = plan.right;
        *output_bottom = plan.bottom;
    }
    true
}

/// Selects the observable bitmap-glyph cache action after an optional native
/// cache probe.
///
/// # Safety
///
/// `output_action` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_plan_glyph_bitmap_lookup(
    glyph_is_valid: bool,
    native_text: bool,
    native_cache_hit: bool,
    output_action: *mut u8,
) -> bool {
    if output_action.is_null() {
        return false;
    }
    let action = plan_glyph_bitmap_lookup(glyph_is_valid, native_text, native_cache_hit);
    // SAFETY: The caller guarantees one live, writable action byte.
    unsafe {
        *output_action = action as u8;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn read_test_bounds(
        context: *mut core::ffi::c_void,
        index: usize,
        output_valid: *mut u8,
        output_left: *mut i32,
        output_top: *mut i32,
        output_width: *mut i32,
        output_height: *mut i32,
    ) -> bool {
        if context.is_null()
            || output_valid.is_null()
            || output_left.is_null()
            || output_top.is_null()
            || output_width.is_null()
            || output_height.is_null()
        {
            return false;
        }
        // SAFETY: This test callback is invoked with a live two-element array
        // as its context.
        let inputs = unsafe { &*context.cast::<[GlyphBoundsInput; 2]>() };
        let Some(input) = inputs.get(index).copied() else {
            return false;
        };
        // SAFETY: The boundary supplies live scalar outputs for this callback.
        unsafe {
            *output_valid = u8::from(input.valid);
            *output_left = input.left;
            *output_top = input.top;
            *output_width = input.width;
            *output_height = input.height;
        }
        true
    }

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

    #[test]
    fn glyph_origin_should_apply_signed_glyph_and_bitmap_offsets() {
        let Some(plan) = plan_glyph_origin(100, -50, -7, 11, 3, -5) else {
            panic!("bounded glyph placement must produce an origin");
        };
        assert_eq!((plan.x, plan.y), (90, -56));
    }

    #[test]
    fn glyph_origin_should_reject_overflow_at_every_checked_operation() {
        let plans = [
            plan_glyph_origin(i32::MAX, 0, 1, 0, 0, 0),
            plan_glyph_origin(i32::MIN, 0, 0, 0, 1, 0),
            plan_glyph_origin(0, i32::MIN, 0, 1, 0, 0),
            plan_glyph_origin(0, i32::MAX, 0, 0, 0, -1),
        ];
        assert!(plans.iter().all(Option::is_none));
    }

    #[test]
    fn glyph_origin_ffi_should_encode_overflow_and_reject_null_outputs() {
        let mut valid = 9_u8;
        let mut x = 17;
        let mut y = 23;
        // SAFETY: All outputs are live and writable for one value.
        assert!(unsafe {
            pdfium_rust_plan_glyph_origin(i32::MAX, 0, 1, 0, 0, 0, &mut valid, &mut x, &mut y)
        });
        assert_eq!((valid, x, y), (0, 0, 0));

        valid = 9;
        x = 17;
        y = 23;
        // SAFETY: The null validity output is explicitly rejected before the
        // other live outputs are mutated.
        assert!(!unsafe {
            pdfium_rust_plan_glyph_origin(1, 2, 3, 4, 5, 6, core::ptr::null_mut(), &mut x, &mut y)
        });
        assert_eq!((valid, x, y), (9, 17, 23));
    }

    #[test]
    fn glyph_device_origin_should_round_non_lcd_like_pdfium() {
        let cases = [
            (3.5, -3.5, (4, -4)),
            (3.49, -3.49, (3, -3)),
            (f32::NAN, f32::NAN, (0, 0)),
            (f32::MAX, f32::MIN_POSITIVE, (i32::MAX, 0)),
            (-f32::MAX, -f32::MIN_POSITIVE, (i32::MIN, 0)),
        ];
        for (x, y, expected) in cases {
            let Some(plan) = plan_glyph_device_origin(x, y, false) else {
                panic!("non-LCD rounding has a saturated result");
            };
            assert_eq!((plan.x, plan.y), expected);
        }
    }

    #[test]
    fn glyph_device_origin_should_floor_lcd_x_and_round_y() {
        let Some(plan) = plan_glyph_device_origin(-3.1, 8.5, true) else {
            panic!("finite LCD coordinates must produce an origin");
        };
        assert_eq!((plan.x, plan.y), (-4, 9));
    }

    #[test]
    fn glyph_device_origin_ffi_should_reject_unsupported_lcd_x_without_mutation() {
        let mut x = 17;
        let mut y = 23;
        // SAFETY: Both outputs are live and writable; the unsupported LCD x
        // value is rejected before either is mutated.
        assert!(!unsafe {
            pdfium_rust_plan_glyph_device_origin(f32::INFINITY, 4.0, true, &mut x, &mut y)
        });
        assert_eq!((x, y), (17, 23));

        // SAFETY: A null x output is explicitly rejected before the live y
        // output is mutated.
        assert!(!unsafe {
            pdfium_rust_plan_glyph_device_origin(1.0, 2.0, false, core::ptr::null_mut(), &mut y)
        });
        assert_eq!(y, 23);
    }

    #[test]
    fn glyph_bounds_should_skip_missing_entries_and_aggregate_extents() {
        let empty = GlyphBoundsPlan { started: false, left: 0, top: 0, right: 0, bottom: 0 };
        let skipped = include_glyph_bounds(
            empty,
            GlyphBoundsInput { valid: false, left: -99, top: -99, width: 1, height: 1 },
            false,
        );
        let first = include_glyph_bounds(
            skipped,
            GlyphBoundsInput { valid: true, left: 10, top: 20, width: 4, height: 5 },
            false,
        );
        let result = include_glyph_bounds(
            first,
            GlyphBoundsInput { valid: true, left: -3, top: 25, width: 20, height: 2 },
            false,
        );
        assert_eq!((result.left, result.top, result.right, result.bottom), (-3, 20, 17, 27));
    }

    #[test]
    fn glyph_bounds_should_divide_lcd_bitmap_width_by_three() {
        let result = include_glyph_bounds(
            GlyphBoundsPlan { started: false, left: 0, top: 0, right: 0, bottom: 0 },
            GlyphBoundsInput { valid: true, left: 7, top: 8, width: 11, height: 5 },
            true,
        );
        assert_eq!((result.left, result.top, result.right, result.bottom), (7, 8, 10, 13));
    }

    #[test]
    fn glyph_bounds_should_skip_overflowing_right_or_bottom_edges() {
        let empty = GlyphBoundsPlan { started: false, left: 0, top: 0, right: 0, bottom: 0 };
        let right_overflow = include_glyph_bounds(
            empty,
            GlyphBoundsInput { valid: true, left: i32::MAX, top: 0, width: 1, height: 1 },
            false,
        );
        let bottom_overflow = include_glyph_bounds(
            right_overflow,
            GlyphBoundsInput { valid: true, left: 0, top: i32::MAX, width: 1, height: 1 },
            false,
        );
        assert!(!bottom_overflow.started);
    }

    #[test]
    fn glyph_bounds_ffi_should_iterate_borrowed_inputs_and_reject_missing_callback() {
        let mut inputs = [
            GlyphBoundsInput { valid: true, left: 5, top: 7, width: 4, height: 6 },
            GlyphBoundsInput { valid: true, left: -2, top: 9, width: 3, height: 2 },
        ];
        let mut left = 91;
        let mut top = 92;
        let mut right = 93;
        let mut bottom = 94;
        // SAFETY: The callback context and all rectangle outputs are live for
        // the duration of the synchronous call.
        assert!(unsafe {
            pdfium_rust_plan_glyph_bounds(
                inputs.len(),
                false,
                inputs.as_mut_ptr().cast(),
                Some(read_test_bounds),
                &mut left,
                &mut top,
                &mut right,
                &mut bottom,
            )
        });
        assert_eq!((left, top, right, bottom), (-2, 7, 9, 13));

        left = 91;
        top = 92;
        right = 93;
        bottom = 94;
        // SAFETY: A missing callback is explicitly rejected before the live
        // rectangle outputs are mutated.
        assert!(!unsafe {
            pdfium_rust_plan_glyph_bounds(
                1,
                false,
                inputs.as_mut_ptr().cast(),
                None,
                &mut left,
                &mut top,
                &mut right,
                &mut bottom,
            )
        });
        assert_eq!((left, top, right, bottom), (91, 92, 93, 94));
    }
}
