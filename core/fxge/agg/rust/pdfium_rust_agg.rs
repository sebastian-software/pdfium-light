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
    !(cycle_length * scale < MIN_DASH_CYCLE_DEVICE_PIXELS)
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
