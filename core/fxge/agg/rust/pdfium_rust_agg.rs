// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

const MIN_DASH_CYCLE_DEVICE_PIXELS: f32 = 0.1;
const MIN_DASH_LENGTH: f32 = 0.000_001;
const REPLACEMENT_DASH_LENGTH: f32 = 0.1;

const LINE_CAP_BUTT: u8 = 0;
const LINE_CAP_ROUND: u8 = 1;
const LINE_CAP_SQUARE: u8 = 2;
const LINE_JOIN_MITER: u8 = 0;
const LINE_JOIN_ROUND: u8 = 1;
const LINE_JOIN_BEVEL: u8 = 2;

/// Plain-data stroke settings returned to the AGG adapter.
#[repr(C)]
pub struct RustAggStrokePlan {
    line_cap: u8,
    line_join: u8,
    reserved: [u8; 2],
    width: f32,
    miter_limit: f32,
}

struct StrokeInputs {
    line_cap: u8,
    line_join: u8,
    line_width: f32,
    scale: f32,
    has_object_to_device: bool,
    object_x_unit: f32,
    object_y_unit: f32,
    miter_limit: f32,
}

fn plan_stroke(inputs: StrokeInputs) -> RustAggStrokePlan {
    let line_cap = match inputs.line_cap {
        LINE_CAP_ROUND => LINE_CAP_ROUND,
        LINE_CAP_SQUARE => LINE_CAP_SQUARE,
        _ => LINE_CAP_BUTT,
    };
    let line_join = match inputs.line_join {
        LINE_JOIN_ROUND => LINE_JOIN_ROUND,
        LINE_JOIN_BEVEL => LINE_JOIN_BEVEL,
        _ => LINE_JOIN_MITER,
    };
    let width = inputs.line_width * inputs.scale;
    let unit = if inputs.has_object_to_device {
        1.0 / ((inputs.object_x_unit + inputs.object_y_unit) / 2.0)
    } else {
        1.0
    };
    // Preserve `std::max(width, unit)`: a NaN width remains the first operand.
    let width = if width < unit { unit } else { width };
    RustAggStrokePlan {
        line_cap,
        line_join,
        reserved: [0; 2],
        width,
        miter_limit: inputs.miter_limit,
    }
}

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

fn normalized_dash_value(value: f32, scale: f32) -> f32 {
    let value = if value <= MIN_DASH_LENGTH { REPLACEMENT_DASH_LENGTH } else { value };
    (value * scale).abs()
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

/// Plans the scalar AGG stroke settings without taking ownership of inputs.
///
/// # Safety
///
/// `output` must point to one writable `RustAggStrokePlan` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_plan_agg_stroke(
    line_cap: u8,
    line_join: u8,
    line_width: f32,
    scale: f32,
    has_object_to_device: bool,
    object_x_unit: f32,
    object_y_unit: f32,
    miter_limit: f32,
    output: *mut RustAggStrokePlan,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable output value.
    unsafe {
        *output = plan_stroke(StrokeInputs {
            line_cap,
            line_join,
            line_width,
            scale,
            has_object_to_device,
            object_x_unit,
            object_y_unit,
            miter_limit,
        });
    }
    true
}

type DashValueCallback = unsafe extern "C" fn(*mut core::ffi::c_void, f32);

/// Normalizes and emits an AGG dash pattern without allocating in Rust.
///
/// # Safety
///
/// When `dash_count` is nonzero, `dash_values` must point to that many
/// readable `f32` values. `callback` must be a valid function that accepts
/// `context` for every emitted value. `dash_start` must point to one writable
/// `f32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_emit_agg_dash_pattern(
    dash_values: *const f32,
    dash_count: usize,
    dash_phase: f32,
    scale: f32,
    context: *mut core::ffi::c_void,
    callback: Option<DashValueCallback>,
    dash_start: *mut f32,
) -> bool {
    if dash_start.is_null() || callback.is_none() || (dash_count != 0 && dash_values.is_null()) {
        return false;
    }
    let dash_array = if dash_count == 0 {
        &[]
    } else {
        // SAFETY: The caller guarantees `dash_count` readable values.
        unsafe { core::slice::from_raw_parts(dash_values, dash_count) }
    };
    let emit = match callback {
        Some(emit) => emit,
        None => return false,
    };
    for &value in dash_array {
        // SAFETY: The caller guarantees that `emit` accepts `context` for
        // every value in the borrowed dash span.
        unsafe {
            emit(context, normalized_dash_value(value, scale));
        }
    }
    // SAFETY: The caller guarantees one writable output value.
    unsafe {
        *dash_start = dash_phase * scale;
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

    #[test]
    fn stroke_plan_should_map_caps_joins_and_miter_limit() {
        let cases = [
            (LINE_CAP_BUTT, LINE_JOIN_MITER),
            (LINE_CAP_ROUND, LINE_JOIN_ROUND),
            (LINE_CAP_SQUARE, LINE_JOIN_BEVEL),
            (u8::MAX, u8::MAX),
        ];
        for (line_cap, line_join) in cases {
            let plan = plan_stroke(StrokeInputs {
                line_cap,
                line_join,
                line_width: 2.0,
                scale: 1.0,
                has_object_to_device: false,
                object_x_unit: 0.0,
                object_y_unit: 0.0,
                miter_limit: 7.5,
            });
            assert_eq!(
                plan.line_cap,
                if line_cap <= LINE_CAP_SQUARE { line_cap } else { LINE_CAP_BUTT }
            );
            assert_eq!(
                plan.line_join,
                if line_join <= LINE_JOIN_BEVEL { line_join } else { LINE_JOIN_MITER }
            );
            assert_eq!(plan.miter_limit, 7.5);
        }
    }

    #[test]
    fn stroke_plan_should_preserve_width_floor_and_nonfinite_semantics() {
        let without_matrix = plan_stroke(StrokeInputs {
            line_cap: LINE_CAP_BUTT,
            line_join: LINE_JOIN_MITER,
            line_width: 0.25,
            scale: 2.0,
            has_object_to_device: false,
            object_x_unit: 0.0,
            object_y_unit: 0.0,
            miter_limit: 1.0,
        });
        assert_eq!(without_matrix.width, 1.0);

        let with_matrix = plan_stroke(StrokeInputs {
            line_width: 0.25,
            has_object_to_device: true,
            object_x_unit: 0.25,
            object_y_unit: 0.75,
            ..StrokeInputs {
                line_cap: LINE_CAP_BUTT,
                line_join: LINE_JOIN_MITER,
                line_width: 0.0,
                scale: 2.0,
                has_object_to_device: false,
                object_x_unit: 0.0,
                object_y_unit: 0.0,
                miter_limit: 1.0,
            }
        });
        assert_eq!(with_matrix.width, 2.0);

        let nan_width = plan_stroke(StrokeInputs {
            line_width: f32::NAN,
            ..StrokeInputs {
                line_cap: LINE_CAP_BUTT,
                line_join: LINE_JOIN_MITER,
                line_width: 0.0,
                scale: 1.0,
                has_object_to_device: false,
                object_x_unit: 0.0,
                object_y_unit: 0.0,
                miter_limit: 1.0,
            }
        });
        assert!(nan_width.width.is_nan());
    }

    #[test]
    fn stroke_plan_ffi_should_reject_null_output() {
        // SAFETY: A null output is explicitly supported and rejected before
        // any write occurs.
        assert!(!unsafe {
            pdfium_rust_plan_agg_stroke(
                LINE_CAP_BUTT,
                LINE_JOIN_MITER,
                1.0,
                1.0,
                false,
                0.0,
                0.0,
                1.0,
                core::ptr::null_mut(),
            )
        });
    }

    #[test]
    fn dash_normalization_should_preserve_replacement_scaling_and_abs() {
        assert_eq!(normalized_dash_value(0.0, 2.0), 0.2);
        assert_eq!(normalized_dash_value(MIN_DASH_LENGTH, -2.0), 0.2);
        assert_eq!(normalized_dash_value(-3.0, 2.0), 0.2);
        assert_eq!(normalized_dash_value(3.0, -2.0), 6.0);
        assert!(normalized_dash_value(3.0, f32::NAN).is_nan());
    }

    unsafe extern "C" fn collect_dash_value(context: *mut core::ffi::c_void, value: f32) {
        // SAFETY: The test passes an exclusive pointer to a live `Vec<f32>`.
        let values = unsafe { &mut *context.cast::<Vec<f32>>() };
        values.push(value);
    }

    #[test]
    fn dash_normalization_ffi_should_emit_all_values_and_phase() {
        let input = [0.0_f32, 2.5, -4.0];
        let mut values: Vec<f32> = Vec::new();
        let mut dash_start = 0.0;
        // SAFETY: The input and output spans remain live, and the callback
        // receives the exclusive vector pointer for the duration of the call.
        assert!(unsafe {
            pdfium_rust_emit_agg_dash_pattern(
                input.as_ptr(),
                input.len(),
                1.25,
                -2.0,
                (&mut values as *mut Vec<f32>).cast(),
                Some(collect_dash_value),
                &mut dash_start,
            )
        });
        assert_eq!(values, [0.2, 5.0, 0.2]);
        assert_eq!(dash_start, -2.5);
    }

    #[test]
    fn dash_normalization_ffi_should_reject_invalid_boundaries() {
        let mut dash_start = 9.0;
        // SAFETY: A missing callback is explicitly rejected before input is
        // read or output is written.
        assert!(!unsafe {
            pdfium_rust_emit_agg_dash_pattern(
                core::ptr::null(),
                0,
                1.0,
                1.0,
                core::ptr::null_mut(),
                None,
                &mut dash_start,
            )
        });
        assert_eq!(dash_start, 9.0);
        // SAFETY: A null nonempty input is explicitly rejected before the
        // callback runs or output is written.
        assert!(!unsafe {
            pdfium_rust_emit_agg_dash_pattern(
                core::ptr::null(),
                1,
                1.0,
                1.0,
                core::ptr::null_mut(),
                Some(collect_dash_value),
                &mut dash_start,
            )
        });
        assert_eq!(dash_start, 9.0);
    }
}
