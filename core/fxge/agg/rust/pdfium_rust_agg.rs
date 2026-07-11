// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

const MIN_DASH_CYCLE_DEVICE_PIXELS: f32 = 0.1;

fn should_apply_dash_pattern(dash_array: &[f32], scale: f32) -> bool {
    if dash_array.is_empty() {
        return false;
    }
    let mut cycle_length = 0.0_f32;
    for &value in dash_array {
        if !value.is_finite() {
            return false;
        }
        cycle_length += value.max(0.0);
    }
    match (cycle_length * scale).partial_cmp(&MIN_DASH_CYCLE_DEVICE_PIXELS) {
        Some(core::cmp::Ordering::Less) => false,
        Some(_) | None => true,
    }
}

/// Decides whether AGG should receive the PDF dash pattern.
///
/// # Safety
///
/// When `dash_count` is nonzero, `dash_values` must point to that many
/// readable `f32` values. `output` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_should_apply_agg_dash_pattern(
    dash_values: *const f32,
    dash_count: usize,
    scale: f32,
    output: *mut bool,
) -> bool {
    if output.is_null() || (dash_count != 0 && dash_values.is_null()) {
        return false;
    }
    let dash_array = if dash_count == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees `dash_count` readable values.
        unsafe { core::slice::from_raw_parts(dash_values, dash_count) }
    };
    // SAFETY: The caller guarantees one writable output value.
    unsafe {
        *output = should_apply_dash_pattern(dash_array, scale);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_pattern_should_preserve_threshold_and_negative_clamping() {
        assert!(!should_apply_dash_pattern(&[], 1.0));
        assert!(!should_apply_dash_pattern(&[0.049, 0.05], 1.0));
        assert!(should_apply_dash_pattern(&[0.05, 0.05], 1.0));
        assert!(should_apply_dash_pattern(&[-10.0, 0.2], 1.0));
        assert!(!should_apply_dash_pattern(&[-10.0], 1.0));
    }

    #[test]
    fn dash_pattern_should_reject_nonfinite_values_but_preserve_scale_nan() {
        assert!(!should_apply_dash_pattern(&[f32::NAN], 1.0));
        assert!(!should_apply_dash_pattern(&[f32::INFINITY], 1.0));
        assert!(should_apply_dash_pattern(&[1.0], f32::NAN));
        assert!(should_apply_dash_pattern(&[1.0], f32::INFINITY));
        assert!(!should_apply_dash_pattern(&[1.0], -1.0));
    }

    #[test]
    fn dash_pattern_ffi_should_reject_invalid_boundaries() {
        let mut output = true;
        // SAFETY: A null nonempty input is explicitly supported and rejected
        // before the input is read.
        assert!(!unsafe {
            pdfium_rust_should_apply_agg_dash_pattern(core::ptr::null(), 1, 1.0, &mut output)
        });
        assert!(output);
        // SAFETY: A null output is explicitly supported and rejected before
        // the valid input is read.
        assert!(!unsafe {
            pdfium_rust_should_apply_agg_dash_pattern(
                [1.0_f32].as_ptr(),
                1,
                1.0,
                core::ptr::null_mut(),
            )
        });
    }
}
