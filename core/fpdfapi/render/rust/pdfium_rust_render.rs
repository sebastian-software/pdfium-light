// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

const FPDF_ANNOT: u32 = 0x01;
const FPDF_LCD_TEXT: u32 = 0x02;
const FPDF_NO_NATIVETEXT: u32 = 0x04;
const FPDF_GRAYSCALE: u32 = 0x08;
const FPDF_CONVERT_FILL_TO_STROKE: u32 = 0x20;
const FPDF_RENDER_LIMITEDIMAGECACHE: u32 = 0x200;
const FPDF_RENDER_FORCEHALFTONE: u32 = 0x400;
const FPDF_PRINTING: u32 = 0x800;
const FPDF_RENDER_NO_SMOOTHTEXT: u32 = 0x1000;
const FPDF_RENDER_NO_SMOOTHIMAGE: u32 = 0x2000;
const FPDF_RENDER_NO_SMOOTHPATH: u32 = 0x4000;

const PLAN_ANNOTATIONS: u32 = 1 << 0;
const PLAN_CLEAR_TYPE: u32 = 1 << 1;
const PLAN_NO_NATIVE_TEXT: u32 = 1 << 2;
const PLAN_GRAYSCALE: u32 = 1 << 3;
const PLAN_CONVERT_FILL_TO_STROKE: u32 = 1 << 4;
const PLAN_LIMITED_IMAGE_CACHE: u32 = 1 << 5;
const PLAN_FORCE_HALFTONE: u32 = 1 << 6;
const PLAN_PRINTING: u32 = 1 << 7;
const PLAN_NO_TEXT_SMOOTHING: u32 = 1 << 8;
const PLAN_NO_IMAGE_SMOOTHING: u32 = 1 << 9;
const PLAN_NO_PATH_SMOOTHING: u32 = 1 << 10;
const PLAN_FORCED_COLOR: u32 = 1 << 11;
const PLAN_RESTORE_DEVICE: u32 = 1 << 12;

const PAGE_OBJECT_TEXT: u32 = 1;
const PAGE_OBJECT_PATH: u32 = 2;
const PAGE_OBJECT_IMAGE: u32 = 3;
const PAGE_OBJECT_SHADING: u32 = 4;
const PAGE_OBJECT_FORM: u32 = 5;

const RENDER_COMMAND_TEXT: u8 = 1;
const RENDER_COMMAND_PATH: u8 = 2;
const RENDER_COMMAND_IMAGE: u8 = 3;
const RENDER_COMMAND_SHADING: u8 = 4;
const RENDER_COMMAND_FORM: u8 = 5;

const OBJECT_LIST_COMMAND_STOP: u8 = 1;
const OBJECT_LIST_COMMAND_SKIP: u8 = 2;
const OBJECT_LIST_COMMAND_RENDER: u8 = 3;

const LAYER_PLAN_SET_OPTIONS: u8 = 1 << 0;
const LAYER_PLAN_APPLY_LAST_MATRIX: u8 = 1 << 1;
const LAYER_COMPLETION_OPTIMIZE_CACHE: u8 = 1 << 0;
const LAYER_COMPLETION_STOP: u8 = 1 << 1;

type RenderLayerCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> bool;

fn build_render_request_plan(flags: u32, has_color_scheme: bool, restore_device: bool) -> u32 {
    let mappings = [
        (FPDF_ANNOT, PLAN_ANNOTATIONS),
        (FPDF_LCD_TEXT, PLAN_CLEAR_TYPE),
        (FPDF_NO_NATIVETEXT, PLAN_NO_NATIVE_TEXT),
        (FPDF_GRAYSCALE, PLAN_GRAYSCALE),
        (FPDF_CONVERT_FILL_TO_STROKE, PLAN_CONVERT_FILL_TO_STROKE),
        (FPDF_RENDER_LIMITEDIMAGECACHE, PLAN_LIMITED_IMAGE_CACHE),
        (FPDF_RENDER_FORCEHALFTONE, PLAN_FORCE_HALFTONE),
        (FPDF_PRINTING, PLAN_PRINTING),
        (FPDF_RENDER_NO_SMOOTHTEXT, PLAN_NO_TEXT_SMOOTHING),
        (FPDF_RENDER_NO_SMOOTHIMAGE, PLAN_NO_IMAGE_SMOOTHING),
        (FPDF_RENDER_NO_SMOOTHPATH, PLAN_NO_PATH_SMOOTHING),
    ];
    let mut plan = 0;
    for (flag, plan_bit) in mappings {
        if flags & flag != 0 {
            plan |= plan_bit;
        }
    }
    if has_color_scheme {
        plan |= PLAN_FORCED_COLOR;
    } else {
        plan &= !PLAN_CONVERT_FILL_TO_STROKE;
    }
    if restore_device {
        plan |= PLAN_RESTORE_DEVICE;
    }
    plan
}

fn build_page_object_render_command(page_object_type: u32) -> Option<u8> {
    match page_object_type {
        PAGE_OBJECT_TEXT => Some(RENDER_COMMAND_TEXT),
        PAGE_OBJECT_PATH => Some(RENDER_COMMAND_PATH),
        PAGE_OBJECT_IMAGE => Some(RENDER_COMMAND_IMAGE),
        PAGE_OBJECT_SHADING => Some(RENDER_COMMAND_SHADING),
        PAGE_OBJECT_FORM => Some(RENDER_COMMAND_FORM),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct FloatRect {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

fn build_object_list_command(
    is_stop_object: bool,
    is_present: bool,
    is_active: bool,
    object_rect: FloatRect,
    clip_rect: FloatRect,
) -> u8 {
    if is_stop_object {
        return OBJECT_LIST_COMMAND_STOP;
    }
    if !is_present || !is_active {
        return OBJECT_LIST_COMMAND_SKIP;
    }
    if object_rect.left > clip_rect.right
        || object_rect.right < clip_rect.left
        || object_rect.bottom > clip_rect.top
        || object_rect.top < clip_rect.bottom
    {
        return OBJECT_LIST_COMMAND_SKIP;
    }
    OBJECT_LIST_COMMAND_RENDER
}

fn build_render_layer_plan(has_options: bool, has_last_matrix: bool) -> u8 {
    let mut plan = 0;
    if has_options {
        plan |= LAYER_PLAN_SET_OPTIONS;
    }
    if has_last_matrix {
        plan |= LAYER_PLAN_APPLY_LAST_MATRIX;
    }
    plan
}

fn build_render_layer_completion(limited_image_cache: bool, stopped: bool) -> u8 {
    let mut plan = 0;
    if limited_image_cache {
        plan |= LAYER_COMPLETION_OPTIMIZE_CACHE;
    }
    if stopped {
        plan |= LAYER_COMPLETION_STOP;
    }
    plan
}

/// Builds a compact render request plan from the supported public flags.
///
/// # Safety
///
/// `output` must point to one writable `u32` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_render_request_plan(
    flags: u32,
    has_color_scheme: bool,
    restore_device: bool,
    output: *mut u32,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = build_render_request_plan(flags, has_color_scheme, restore_device);
    }
    true
}

/// Maps a page-object type to the render command that handles it.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_page_object_render_command(
    page_object_type: u32,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(command) = build_page_object_render_command(page_object_type) else {
        return false;
    };
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = command;
    }
    true
}

/// Plans whether one object-list entry stops, skips, or enters rendering.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_object_list_command(
    is_stop_object: bool,
    is_present: bool,
    is_active: bool,
    object_left: f32,
    object_bottom: f32,
    object_right: f32,
    object_top: f32,
    clip_left: f32,
    clip_bottom: f32,
    clip_right: f32,
    clip_top: f32,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let command = build_object_list_command(
        is_stop_object,
        is_present,
        is_active,
        FloatRect {
            left: object_left,
            bottom: object_bottom,
            right: object_right,
            top: object_top,
        },
        FloatRect { left: clip_left, bottom: clip_bottom, right: clip_right, top: clip_top },
    );
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = command;
    }
    true
}

/// Plans the optional setup performed before one render layer.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_render_layer_plan(
    has_options: bool,
    has_last_matrix: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = build_render_layer_plan(has_options, has_last_matrix);
    }
    true
}

/// Plans cache cleanup and traversal after one render layer.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_render_layer_completion(
    limited_image_cache: bool,
    stopped: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = build_render_layer_completion(limited_image_cache, stopped);
    }
    true
}

/// Runs render layers in order until the callback reports a stopped status.
///
/// # Safety
///
/// `context` must remain valid for every callback invocation. `callback` must
/// obey its own C ABI contract for that context and each index below
/// `layer_count`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_run_render_layers(
    layer_count: u32,
    context: *mut core::ffi::c_void,
    callback: Option<RenderLayerCallback>,
) -> bool {
    if context.is_null() {
        return false;
    }
    let Some(callback) = callback else {
        return false;
    };
    for index in 0..layer_count {
        // SAFETY: The caller guarantees that the context and callback remain
        // valid for every index in this bounded loop.
        if unsafe { callback(context, index) } {
            break;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct LayerLoopState {
        visited: [u32; 4],
        visited_count: usize,
        stop_at: Option<u32>,
    }

    unsafe extern "C" fn record_render_layer(
        context: *mut core::ffi::c_void,
        layer_index: u32,
    ) -> bool {
        // SAFETY: The tests pass a unique, live `LayerLoopState` pointer for
        // the complete callback loop.
        let state = unsafe { &mut *context.cast::<LayerLoopState>() };
        state.visited[state.visited_count] = layer_index;
        state.visited_count += 1;
        state.stop_at == Some(layer_index)
    }

    #[test]
    fn request_plan_should_map_every_supported_flag() {
        let flags = FPDF_ANNOT
            | FPDF_LCD_TEXT
            | FPDF_NO_NATIVETEXT
            | FPDF_GRAYSCALE
            | FPDF_CONVERT_FILL_TO_STROKE
            | FPDF_RENDER_LIMITEDIMAGECACHE
            | FPDF_RENDER_FORCEHALFTONE
            | FPDF_PRINTING
            | FPDF_RENDER_NO_SMOOTHTEXT
            | FPDF_RENDER_NO_SMOOTHIMAGE
            | FPDF_RENDER_NO_SMOOTHPATH;
        assert_eq!(0x1fff, build_render_request_plan(flags, true, true));
    }

    #[test]
    fn fill_to_stroke_should_require_a_color_scheme() {
        assert_eq!(0, build_render_request_plan(FPDF_CONVERT_FILL_TO_STROKE, false, false));
        assert_eq!(PLAN_FORCED_COLOR, build_render_request_plan(0, true, false));
    }

    #[test]
    fn page_object_render_command_should_map_every_supported_type() {
        let cases = [
            (PAGE_OBJECT_TEXT, RENDER_COMMAND_TEXT),
            (PAGE_OBJECT_PATH, RENDER_COMMAND_PATH),
            (PAGE_OBJECT_IMAGE, RENDER_COMMAND_IMAGE),
            (PAGE_OBJECT_SHADING, RENDER_COMMAND_SHADING),
            (PAGE_OBJECT_FORM, RENDER_COMMAND_FORM),
        ];
        for (page_object_type, expected) in cases {
            assert_eq!(Some(expected), build_page_object_render_command(page_object_type));
        }
    }

    #[test]
    fn page_object_render_command_should_reject_invalid_inputs() {
        assert_eq!(None, build_page_object_render_command(0));
        assert_eq!(None, build_page_object_render_command(6));

        let mut output = 0xa5;
        // SAFETY: `output` is one writable byte for the duration of the call.
        assert!(!unsafe { pdfium_rust_build_page_object_render_command(6, &mut output) });
        assert_eq!(0xa5, output);
        // SAFETY: A null output is explicitly supported and rejected without
        // dereferencing it.
        assert!(!unsafe {
            pdfium_rust_build_page_object_render_command(PAGE_OBJECT_TEXT, core::ptr::null_mut())
        });
    }

    #[test]
    fn object_list_stop_should_take_precedence() {
        let empty = FloatRect { left: 0.0, bottom: 0.0, right: 0.0, top: 0.0 };
        assert_eq!(
            OBJECT_LIST_COMMAND_STOP,
            build_object_list_command(true, false, false, empty, empty)
        );
    }

    #[test]
    fn object_list_should_skip_missing_inactive_and_outside_objects() {
        let clip = FloatRect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 };
        let inside = FloatRect { left: 2.0, bottom: 2.0, right: 8.0, top: 8.0 };
        assert_eq!(
            OBJECT_LIST_COMMAND_SKIP,
            build_object_list_command(false, false, false, inside, clip)
        );
        assert_eq!(
            OBJECT_LIST_COMMAND_SKIP,
            build_object_list_command(false, true, false, inside, clip)
        );

        let outside = [
            FloatRect { left: 11.0, bottom: 2.0, right: 12.0, top: 8.0 },
            FloatRect { left: -2.0, bottom: 2.0, right: -1.0, top: 8.0 },
            FloatRect { left: 2.0, bottom: 11.0, right: 8.0, top: 12.0 },
            FloatRect { left: 2.0, bottom: -2.0, right: 8.0, top: -1.0 },
        ];
        for object_rect in outside {
            assert_eq!(
                OBJECT_LIST_COMMAND_SKIP,
                build_object_list_command(false, true, true, object_rect, clip)
            );
        }
    }

    #[test]
    fn object_list_should_render_touching_and_nan_bounds() {
        let clip = FloatRect { left: 0.0, bottom: 0.0, right: 10.0, top: 10.0 };
        let touching = FloatRect { left: 10.0, bottom: 2.0, right: 12.0, top: 8.0 };
        assert_eq!(
            OBJECT_LIST_COMMAND_RENDER,
            build_object_list_command(false, true, true, touching, clip)
        );

        let nan_bound = FloatRect { left: f32::NAN, bottom: 2.0, right: 12.0, top: 8.0 };
        assert_eq!(
            OBJECT_LIST_COMMAND_RENDER,
            build_object_list_command(false, true, true, nan_bound, clip)
        );
    }

    #[test]
    fn object_list_ffi_should_reject_a_null_output() {
        // SAFETY: A null output is explicitly supported and rejected without
        // dereferencing it.
        assert!(!unsafe {
            pdfium_rust_build_object_list_command(
                false,
                true,
                true,
                0.0,
                0.0,
                1.0,
                1.0,
                0.0,
                0.0,
                1.0,
                1.0,
                core::ptr::null_mut(),
            )
        });
    }

    #[test]
    fn render_layer_plan_should_map_all_optional_setup() {
        assert_eq!(0, build_render_layer_plan(false, false));
        assert_eq!(LAYER_PLAN_SET_OPTIONS, build_render_layer_plan(true, false));
        assert_eq!(LAYER_PLAN_APPLY_LAST_MATRIX, build_render_layer_plan(false, true));
        assert_eq!(
            LAYER_PLAN_SET_OPTIONS | LAYER_PLAN_APPLY_LAST_MATRIX,
            build_render_layer_plan(true, true)
        );
    }

    #[test]
    fn render_layer_completion_should_map_cache_and_stop() {
        assert_eq!(0, build_render_layer_completion(false, false));
        assert_eq!(LAYER_COMPLETION_OPTIMIZE_CACHE, build_render_layer_completion(true, false));
        assert_eq!(LAYER_COMPLETION_STOP, build_render_layer_completion(false, true));
        assert_eq!(
            LAYER_COMPLETION_OPTIMIZE_CACHE | LAYER_COMPLETION_STOP,
            build_render_layer_completion(true, true)
        );
    }

    #[test]
    fn render_layer_ffi_should_reject_null_outputs() {
        // SAFETY: Null outputs are explicitly supported and rejected without
        // dereferencing them.
        assert!(!unsafe {
            pdfium_rust_build_render_layer_plan(false, false, core::ptr::null_mut())
        });
        // SAFETY: Null outputs are explicitly supported and rejected without
        // dereferencing them.
        assert!(!unsafe {
            pdfium_rust_build_render_layer_completion(false, false, core::ptr::null_mut())
        });
    }

    #[test]
    fn render_layer_loop_should_visit_in_order_and_stop() {
        let mut all_layers =
            LayerLoopState { visited: [u32::MAX; 4], visited_count: 0, stop_at: None };
        // SAFETY: The context points to `all_layers` for the complete call and
        // the callback accepts every generated index.
        assert!(unsafe {
            pdfium_rust_run_render_layers(
                3,
                (&mut all_layers as *mut LayerLoopState).cast(),
                Some(record_render_layer),
            )
        });
        assert_eq!(3, all_layers.visited_count);
        assert_eq!([0, 1, 2], all_layers.visited[..3]);

        let mut stopped =
            LayerLoopState { visited: [u32::MAX; 4], visited_count: 0, stop_at: Some(1) };
        // SAFETY: The context points to `stopped` for the complete call and
        // the callback accepts every generated index.
        assert!(unsafe {
            pdfium_rust_run_render_layers(
                4,
                (&mut stopped as *mut LayerLoopState).cast(),
                Some(record_render_layer),
            )
        });
        assert_eq!(2, stopped.visited_count);
        assert_eq!([0, 1], stopped.visited[..2]);
    }

    #[test]
    fn render_layer_loop_should_reject_invalid_boundaries() {
        let mut state = LayerLoopState { visited: [u32::MAX; 4], visited_count: 0, stop_at: None };
        // SAFETY: A null context is explicitly supported and rejected before
        // invoking the valid callback.
        assert!(!unsafe {
            pdfium_rust_run_render_layers(1, core::ptr::null_mut(), Some(record_render_layer))
        });
        // SAFETY: A missing callback is explicitly supported and rejected
        // before reading the valid context.
        assert!(!unsafe {
            pdfium_rust_run_render_layers(1, (&mut state as *mut LayerLoopState).cast(), None)
        });
        assert_eq!(0, state.visited_count);
    }
}
