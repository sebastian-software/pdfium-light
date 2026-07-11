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
const PATH_POINT_LINE: u8 = 0;
const PATH_POINT_BEZIER: u8 = 1;
const PATH_POINT_MOVE: u8 = 2;
const PATH_COMMAND_MOVE: u8 = 0;
const PATH_COMMAND_LINE: u8 = 1;
const PATH_COMMAND_BEZIER: u8 = 2;
const PATH_COMMAND_CLOSE: u8 = 3;
const MAX_PATH_POSITION: f32 = 32_000.0;

/// Plain-data stroke settings returned to the AGG adapter.
#[repr(C)]
pub struct RustAggStrokePlan {
    line_cap: u8,
    line_join: u8,
    reserved: [u8; 2],
    width: f32,
    miter_limit: f32,
}

/// Plain-data path point borrowed from the C++ path owner.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct RustAggPathPoint {
    x: f32,
    y: f32,
    point_type: u8,
    close_figure: u8,
    reserved: [u8; 2],
}

#[derive(Clone, Copy)]
struct Matrix {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

fn hard_clip(value: f32) -> f32 {
    value.clamp(-MAX_PATH_POSITION, MAX_PATH_POSITION)
}

fn transform_and_clip(point: RustAggPathPoint, matrix: Option<Matrix>) -> [f32; 2] {
    let (x, y) = match matrix {
        Some(matrix) => (
            matrix.a * point.x + matrix.c * point.y + matrix.e,
            matrix.b * point.x + matrix.d * point.y + matrix.f,
        ),
        None => (point.x, point.y),
    };
    [hard_clip(x), hard_clip(y)]
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

type PathPointCallback = unsafe extern "C" fn(*mut core::ffi::c_void, usize, *mut RustAggPathPoint);
type PathCommandCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u8, *const f32, usize);

unsafe fn read_path_point(
    context: *mut core::ffi::c_void,
    index: usize,
    callback: PathPointCallback,
) -> RustAggPathPoint {
    let mut point =
        RustAggPathPoint { x: 0.0, y: 0.0, point_type: u8::MAX, close_figure: 0, reserved: [0; 2] };
    // SAFETY: The caller guarantees that the callback writes one point for
    // every index below the declared point count.
    unsafe {
        callback(context, index, &mut point);
    }
    point
}

unsafe fn emit_path_command(
    context: *mut core::ffi::c_void,
    command: u8,
    coordinates: &[f32],
    callback: PathCommandCallback,
) {
    // SAFETY: The caller guarantees that the callback consumes the borrowed
    // coordinate span before returning.
    unsafe {
        callback(context, command, coordinates.as_ptr(), coordinates.len());
    }
}

/// Iterates, transforms, clips, and emits a PDF path without allocating.
///
/// # Safety
///
/// `read_callback` must write one valid `RustAggPathPoint` for every requested
/// index below `point_count`. `command_callback` must consume each borrowed
/// coordinate span before returning. Both callbacks must accept `context` for
/// the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_emit_agg_path(
    point_count: usize,
    has_matrix: bool,
    matrix_a: f32,
    matrix_b: f32,
    matrix_c: f32,
    matrix_d: f32,
    matrix_e: f32,
    matrix_f: f32,
    context: *mut core::ffi::c_void,
    read_callback: Option<PathPointCallback>,
    command_callback: Option<PathCommandCallback>,
) -> bool {
    let (read_point, emit_command) = match (read_callback, command_callback) {
        (Some(read_point), Some(emit_command)) => (read_point, emit_command),
        _ => return false,
    };
    let matrix = has_matrix.then_some(Matrix {
        a: matrix_a,
        b: matrix_b,
        c: matrix_c,
        d: matrix_d,
        e: matrix_e,
        f: matrix_f,
    });
    let mut index = 0;
    while index < point_count {
        // SAFETY: `index` is below the declared point count.
        let point = unsafe { read_path_point(context, index, read_point) };
        let mut position = transform_and_clip(point, matrix);
        let mut close_figure = point.close_figure != 0;
        match point.point_type {
            PATH_POINT_MOVE => {
                // SAFETY: The coordinate array remains live for the callback.
                unsafe {
                    emit_path_command(context, PATH_COMMAND_MOVE, &position, emit_command);
                }
            }
            PATH_POINT_LINE => {
                if index > 0 {
                    // SAFETY: `index - 1` is below the declared point count.
                    let previous = unsafe { read_path_point(context, index - 1, read_point) };
                    let next_is_open_move = if index + 1 == point_count {
                        true
                    } else {
                        // SAFETY: `index + 1` is below the declared point count.
                        let next = unsafe { read_path_point(context, index + 1, read_point) };
                        next.point_type == PATH_POINT_MOVE && next.close_figure == 0
                    };
                    if previous.point_type == PATH_POINT_MOVE
                        && previous.close_figure == 0
                        && next_is_open_move
                        && point.x == previous.x
                        && point.y == previous.y
                    {
                        position[0] += 1.0;
                    }
                }
                // SAFETY: The coordinate array remains live for the callback.
                unsafe {
                    emit_path_command(context, PATH_COMMAND_LINE, &position, emit_command);
                }
            }
            PATH_POINT_BEZIER if index > 0 && index + 2 < point_count => {
                // SAFETY: The guarded indices are below the declared count.
                let previous = unsafe { read_path_point(context, index - 1, read_point) };
                // SAFETY: The guarded indices are below the declared count.
                let next = unsafe { read_path_point(context, index + 1, read_point) };
                // SAFETY: The guarded indices are below the declared count.
                let end = unsafe { read_path_point(context, index + 2, read_point) };
                let previous_position = transform_and_clip(previous, matrix);
                let next_position = transform_and_clip(next, matrix);
                let end_position = transform_and_clip(end, matrix);
                let coordinates = [
                    previous_position[0],
                    previous_position[1],
                    position[0],
                    position[1],
                    next_position[0],
                    next_position[1],
                    end_position[0],
                    end_position[1],
                ];
                // SAFETY: The coordinate array remains live for the callback.
                unsafe {
                    emit_path_command(context, PATH_COMMAND_BEZIER, &coordinates, emit_command);
                }
                index += 2;
                close_figure = end.close_figure != 0;
            }
            _ => {}
        }
        if close_figure {
            // SAFETY: The empty coordinate span remains valid for the callback.
            unsafe {
                emit_path_command(context, PATH_COMMAND_CLOSE, &[], emit_command);
            }
        }
        index += 1;
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

    #[derive(Debug, PartialEq)]
    struct CollectedPathCommand {
        command: u8,
        coordinates: Vec<f32>,
    }

    struct PathCallbackContext {
        points: Vec<RustAggPathPoint>,
        commands: Vec<CollectedPathCommand>,
    }

    fn path_point(x: f32, y: f32, point_type: u8, close_figure: bool) -> RustAggPathPoint {
        RustAggPathPoint {
            x,
            y,
            point_type,
            close_figure: u8::from(close_figure),
            reserved: [0; 2],
        }
    }

    unsafe extern "C" fn read_test_path_point(
        context: *mut core::ffi::c_void,
        index: usize,
        output: *mut RustAggPathPoint,
    ) {
        // SAFETY: The tests pass a live context, a valid in-range index, and
        // one writable output point.
        let context = unsafe { &mut *context.cast::<PathCallbackContext>() };
        // SAFETY: The FFI iterator requests only indices below `point_count`.
        unsafe {
            *output = context.points[index];
        }
    }

    unsafe extern "C" fn collect_test_path_command(
        context: *mut core::ffi::c_void,
        command: u8,
        coordinates: *const f32,
        coordinate_count: usize,
    ) {
        // SAFETY: The tests pass a live, exclusively borrowed context.
        let context = unsafe { &mut *context.cast::<PathCallbackContext>() };
        let coordinates = if coordinate_count == 0 {
            &[]
        } else {
            // SAFETY: The emitter guarantees a readable borrowed coordinate
            // span for the duration of this callback.
            unsafe { core::slice::from_raw_parts(coordinates, coordinate_count) }
        };
        context.commands.push(CollectedPathCommand { command, coordinates: coordinates.to_vec() });
    }

    fn collect_path_commands(
        points: Vec<RustAggPathPoint>,
        matrix: Option<Matrix>,
    ) -> Vec<CollectedPathCommand> {
        let mut context = PathCallbackContext { points, commands: Vec::new() };
        let has_matrix = matrix.is_some();
        let matrix = matrix.unwrap_or(Matrix { a: 0.0, b: 0.0, c: 0.0, d: 0.0, e: 0.0, f: 0.0 });
        let point_count = context.points.len();
        // SAFETY: Both callbacks use the live exclusive context and consume
        // every borrowed value before returning.
        assert!(unsafe {
            pdfium_rust_emit_agg_path(
                point_count,
                has_matrix,
                matrix.a,
                matrix.b,
                matrix.c,
                matrix.d,
                matrix.e,
                matrix.f,
                (&mut context as *mut PathCallbackContext).cast(),
                Some(read_test_path_point),
                Some(collect_test_path_command),
            )
        });
        context.commands
    }

    #[test]
    fn path_emission_should_preserve_moves_lines_single_points_and_close() {
        let commands = collect_path_commands(
            vec![
                path_point(4.0, 5.0, PATH_POINT_MOVE, false),
                path_point(4.0, 5.0, PATH_POINT_LINE, true),
            ],
            None,
        );
        assert_eq!(
            commands,
            [
                CollectedPathCommand { command: PATH_COMMAND_MOVE, coordinates: vec![4.0, 5.0] },
                CollectedPathCommand { command: PATH_COMMAND_LINE, coordinates: vec![5.0, 5.0] },
                CollectedPathCommand { command: PATH_COMMAND_CLOSE, coordinates: vec![] },
            ]
        );
    }

    #[test]
    fn path_emission_should_transform_clip_and_group_bezier_points() {
        let commands = collect_path_commands(
            vec![
                path_point(1.0, 2.0, PATH_POINT_MOVE, false),
                path_point(3.0, 4.0, PATH_POINT_BEZIER, false),
                path_point(5.0, 6.0, PATH_POINT_BEZIER, false),
                path_point(40_000.0, 8.0, PATH_POINT_BEZIER, true),
            ],
            Some(Matrix { a: 2.0, b: 0.0, c: 0.0, d: 3.0, e: 10.0, f: -5.0 }),
        );
        assert_eq!(commands[0].coordinates, [12.0, 1.0]);
        assert_eq!(commands[1].coordinates, [12.0, 1.0, 16.0, 7.0, 20.0, 13.0, 32_000.0, 19.0]);
        assert_eq!(commands[2].command, PATH_COMMAND_CLOSE);
    }

    #[test]
    fn path_emission_ffi_should_reject_missing_callbacks() {
        // SAFETY: Missing callbacks are explicitly rejected before any point
        // is read or command emitted.
        assert!(!unsafe {
            pdfium_rust_emit_agg_path(
                0,
                false,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                core::ptr::null_mut(),
                None,
                None,
            )
        });
    }
}
