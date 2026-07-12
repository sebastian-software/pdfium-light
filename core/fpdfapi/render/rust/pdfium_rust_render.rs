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

const PATH_FILL_NONE: u8 = 0;
const PATH_FILL_EVEN_ODD: u8 = 1;
const PATH_FILL_WINDING: u8 = 2;
const PATH_PLAN_FILL_MASK: u8 = 0x03;
const PATH_PLAN_STROKE: u8 = 1 << 2;
const PATH_PLAN_DRAW: u8 = 1 << 3;

const PATH_OPTIONS_FILL_MASK: u8 = 0x03;
const PATH_OPTIONS_RECT_AA: u8 = 1 << 2;
const PATH_OPTIONS_ALIASED: u8 = 1 << 3;
const PATH_OPTIONS_ADJUST_STROKE: u8 = 1 << 4;
const PATH_OPTIONS_STROKE: u8 = 1 << 5;
const PATH_OPTIONS_TEXT_MODE: u8 = 1 << 6;

const TEXT_PLAN_SKIP: u8 = 1;
const TEXT_PLAN_TYPE3: u8 = 2;
const TEXT_PLAN_NORMAL: u8 = 3;
const TEXT_PLAN_FILL: u8 = 1 << 0;
const TEXT_PLAN_STROKE: u8 = 1 << 1;
const TEXT_PLAN_CLIP: u8 = 1 << 2;
const TEXT_PATH_OPTION_STROKE: u8 = 1 << 0;
const TEXT_PATH_OPTION_STROKE_TEXT_MODE: u8 = 1 << 1;
const TEXT_PATH_OPTION_ADJUST_STROKE: u8 = 1 << 2;
const TEXT_PATH_OPTION_ALIASED: u8 = 1 << 3;
const TEXT_BACKEND_PATTERN: u8 = 1;
const TEXT_BACKEND_PATH: u8 = 2;
const TEXT_BACKEND_NORMAL: u8 = 3;

type RenderLayerCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u32) -> bool;
type TextBackendCallback = unsafe extern "C" fn(*mut core::ffi::c_void, u8) -> bool;

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

fn build_path_paint_plan(
    fill_type: u8,
    stroke: bool,
    forced_color: bool,
    convert_fill_to_stroke: bool,
) -> Option<u8> {
    match fill_type {
        PATH_FILL_NONE | PATH_FILL_EVEN_ODD | PATH_FILL_WINDING => {}
        _ => return None,
    }
    if fill_type == PATH_FILL_NONE && !stroke {
        return Some(PATH_FILL_NONE);
    }

    let (planned_fill, planned_stroke) =
        if forced_color && convert_fill_to_stroke && fill_type != PATH_FILL_NONE {
            (PATH_FILL_NONE, true)
        } else {
            (fill_type, stroke)
        };
    let mut plan = planned_fill & PATH_PLAN_FILL_MASK;
    if planned_stroke {
        plan |= PATH_PLAN_STROKE;
    }
    plan |= PATH_PLAN_DRAW;
    Some(plan)
}

fn path_matrix_is_available(a: f32, b: f32, c: f32, d: f32) -> bool {
    if a == 0.0 || d == 0.0 {
        return b != 0.0 && c != 0.0;
    }
    if b == 0.0 || c == 0.0 {
        return a != 0.0 && d != 0.0;
    }
    true
}

fn build_path_fill_options(
    fill_type: u8,
    rect_aa: bool,
    no_path_smooth: bool,
    stroke_adjust: bool,
    stroke: bool,
    type3_char: bool,
) -> Option<u8> {
    match fill_type {
        PATH_FILL_NONE | PATH_FILL_EVEN_ODD | PATH_FILL_WINDING => {}
        _ => return None,
    }
    let mut options = fill_type & PATH_OPTIONS_FILL_MASK;
    if fill_type != PATH_FILL_NONE && rect_aa {
        options |= PATH_OPTIONS_RECT_AA;
    }
    if no_path_smooth {
        options |= PATH_OPTIONS_ALIASED;
    }
    if stroke_adjust {
        options |= PATH_OPTIONS_ADJUST_STROKE;
    }
    if stroke {
        options |= PATH_OPTIONS_STROKE;
    }
    if type3_char {
        options |= PATH_OPTIONS_TEXT_MODE;
    }
    Some(options)
}

fn build_text_render_plan(
    has_char_codes: bool,
    text_mode: i8,
    is_type3: bool,
    has_clipping_path: bool,
    font_has_face: bool,
) -> Option<(u8, u8)> {
    if !has_char_codes || text_mode == 3 {
        return Some((TEXT_PLAN_SKIP, 0));
    }
    if is_type3 {
        return Some((TEXT_PLAN_TYPE3, 0));
    }
    if text_mode == 7 {
        return Some((TEXT_PLAN_SKIP, 0));
    }
    if !(0..=6).contains(&text_mode) {
        return None;
    }
    if has_clipping_path {
        return Some((TEXT_PLAN_NORMAL, TEXT_PLAN_CLIP));
    }
    let bits = match text_mode {
        0 | 4 => TEXT_PLAN_FILL,
        1 | 5 if font_has_face => TEXT_PLAN_STROKE,
        1 | 5 => TEXT_PLAN_FILL,
        2 | 6 if font_has_face => TEXT_PLAN_FILL | TEXT_PLAN_STROKE,
        2 | 6 => TEXT_PLAN_FILL,
        _ => return None,
    };
    Some((TEXT_PLAN_NORMAL, bits))
}

fn text_uses_pattern(
    is_fill: bool,
    is_stroke: bool,
    fill_is_pattern: bool,
    stroke_is_pattern: bool,
) -> bool {
    (is_fill && fill_is_pattern) || (is_stroke && stroke_is_pattern)
}

fn text_uses_path_backend(is_clip: bool, is_stroke: bool) -> bool {
    is_clip || is_stroke
}

fn text_needs_device_matrix_adjustment(is_stroke: bool, ctm_a: f32, ctm_d: f32) -> bool {
    is_stroke && (ctm_a != 1.0 || ctm_d != 1.0)
}

fn build_text_path_fill_options(
    is_stroke: bool,
    is_fill: bool,
    stroke_adjust: bool,
    no_text_smooth: bool,
) -> u8 {
    let mut bits = 0;
    if is_stroke && is_fill {
        bits |= TEXT_PATH_OPTION_STROKE | TEXT_PATH_OPTION_STROKE_TEXT_MODE;
    }
    if stroke_adjust {
        bits |= TEXT_PATH_OPTION_ADJUST_STROKE;
    }
    if no_text_smooth {
        bits |= TEXT_PATH_OPTION_ALIASED;
    }
    bits
}

fn text_backend_command(uses_pattern: bool, is_clip: bool, is_stroke: bool) -> u8 {
    if uses_pattern {
        TEXT_BACKEND_PATTERN
    } else if is_clip || is_stroke {
        TEXT_BACKEND_PATH
    } else {
        TEXT_BACKEND_NORMAL
    }
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

/// Plans path fill/stroke behavior before native color and backend work.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_path_paint_plan(
    fill_type: u8,
    stroke: bool,
    forced_color: bool,
    convert_fill_to_stroke: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(plan) = build_path_paint_plan(fill_type, stroke, forced_color, convert_fill_to_stroke)
    else {
        return false;
    };
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = plan;
    }
    true
}

/// Checks whether a path matrix can be sent to the retained backend.
///
/// # Safety
///
/// `output` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_path_matrix_is_available(
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    output: *mut bool,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = path_matrix_is_available(a, b, c, d);
    }
    true
}

/// Builds the retained backend's path fill and graph-state options.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_path_fill_options(
    fill_type: u8,
    rect_aa: bool,
    no_path_smooth: bool,
    stroke_adjust: bool,
    stroke: bool,
    type3_char: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    let Some(options) = build_path_fill_options(
        fill_type,
        rect_aa,
        no_path_smooth,
        stroke_adjust,
        stroke,
        type3_char,
    ) else {
        return false;
    };
    // SAFETY: The caller contract guarantees one writable output value.
    unsafe {
        *output = options;
    }
    true
}

/// Plans text dispatch while C++ retains fonts, colors, matrices, and drawing.
///
/// # Safety
///
/// Both outputs must point to one writable `u8` value. Neither is written
/// when the rendering mode is outside PDF's supported range.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_text_render_plan(
    has_char_codes: bool,
    text_mode: i8,
    is_type3: bool,
    has_clipping_path: bool,
    font_has_face: bool,
    output_action: *mut u8,
    output_bits: *mut u8,
) -> bool {
    if output_action.is_null() || output_bits.is_null() {
        return false;
    }
    let Some((action, bits)) = build_text_render_plan(
        has_char_codes,
        text_mode,
        is_type3,
        has_clipping_path,
        font_has_face,
    ) else {
        return false;
    };
    // SAFETY: Both pointers were checked and the caller guarantees one
    // writable byte at each location.
    unsafe {
        *output_action = action;
        *output_bits = bits;
    }
    true
}

/// Decides whether the text renderer must take its retained pattern path.
///
/// # Safety
///
/// `output` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_uses_pattern(
    is_fill: bool,
    is_stroke: bool,
    fill_is_pattern: bool,
    stroke_is_pattern: bool,
    output: *mut bool,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable bool output.
    unsafe {
        *output = text_uses_pattern(is_fill, is_stroke, fill_is_pattern, stroke_is_pattern);
    }
    true
}

/// Selects the retained text backend from clip and stroke behavior.
///
/// # Safety
///
/// `output` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_uses_path_backend(
    is_clip: bool,
    is_stroke: bool,
    output: *mut bool,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable bool output.
    unsafe {
        *output = text_uses_path_backend(is_clip, is_stroke);
    }
    true
}

/// Decides whether a stroked text call needs the existing CTM adjustment.
///
/// # Safety
///
/// `output` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_text_needs_device_matrix_adjustment(
    is_stroke: bool,
    ctm_a: f32,
    ctm_d: f32,
    output: *mut bool,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable bool output.
    unsafe {
        *output = text_needs_device_matrix_adjustment(is_stroke, ctm_a, ctm_d);
    }
    true
}

/// Builds the retained path-text fill options from scalar text state.
///
/// # Safety
///
/// `output` must point to one writable `u8` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_build_text_path_fill_options(
    is_stroke: bool,
    is_fill: bool,
    stroke_adjust: bool,
    no_text_smooth: bool,
    output: *mut u8,
) -> bool {
    if output.is_null() {
        return false;
    }
    // SAFETY: The caller guarantees one writable byte output.
    unsafe {
        *output = build_text_path_fill_options(is_stroke, is_fill, stroke_adjust, no_text_smooth);
    }
    true
}

/// Selects and invokes the existing C++ text backend exactly once.
///
/// # Safety
///
/// `context` and `callback` must remain valid for the synchronous call.
/// `output_result` must point to one writable `bool` value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pdfium_rust_run_text_backend(
    uses_pattern: bool,
    is_clip: bool,
    is_stroke: bool,
    context: *mut core::ffi::c_void,
    callback: Option<TextBackendCallback>,
    output_result: *mut bool,
) -> bool {
    if context.is_null() || output_result.is_null() {
        return false;
    }
    let Some(callback) = callback else {
        return false;
    };
    let command = text_backend_command(uses_pattern, is_clip, is_stroke);
    // SAFETY: The caller guarantees a live context and synchronous callback
    // for the duration of this single invocation.
    let result = unsafe { callback(context, command) };
    // SAFETY: The checked output pointer refers to one writable result.
    unsafe {
        *output_result = result;
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

    struct TextBackendState {
        command: u8,
        result: bool,
        calls: usize,
    }

    unsafe extern "C" fn record_text_backend(context: *mut core::ffi::c_void, command: u8) -> bool {
        // SAFETY: The test passes one live `TextBackendState` for the call.
        let state = unsafe { &mut *context.cast::<TextBackendState>() };
        state.command = command;
        state.calls += 1;
        state.result
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

    #[test]
    fn path_paint_plan_should_preserve_fill_and_stroke_modes() {
        assert_eq!(Some(0), build_path_paint_plan(PATH_FILL_NONE, false, false, false));
        assert_eq!(
            Some(PATH_PLAN_STROKE | PATH_PLAN_DRAW),
            build_path_paint_plan(PATH_FILL_NONE, true, false, false)
        );
        assert_eq!(
            Some(PATH_FILL_EVEN_ODD | PATH_PLAN_DRAW),
            build_path_paint_plan(PATH_FILL_EVEN_ODD, false, false, false)
        );
        assert_eq!(
            Some(PATH_FILL_WINDING | PATH_PLAN_STROKE | PATH_PLAN_DRAW),
            build_path_paint_plan(PATH_FILL_WINDING, true, false, false)
        );
    }

    #[test]
    fn path_paint_plan_should_convert_forced_fills_to_strokes() {
        for fill_type in [PATH_FILL_EVEN_ODD, PATH_FILL_WINDING] {
            assert_eq!(
                Some(PATH_PLAN_STROKE | PATH_PLAN_DRAW),
                build_path_paint_plan(fill_type, false, true, true)
            );
            assert_eq!(
                Some(fill_type | PATH_PLAN_DRAW),
                build_path_paint_plan(fill_type, false, true, false)
            );
            assert_eq!(
                Some(fill_type | PATH_PLAN_DRAW),
                build_path_paint_plan(fill_type, false, false, true)
            );
        }
    }

    #[test]
    fn path_paint_plan_should_reject_invalid_boundaries() {
        assert_eq!(None, build_path_paint_plan(3, true, false, false));
        let mut output = 0xa5;
        // SAFETY: `output` is one writable byte for the duration of the call.
        assert!(!unsafe { pdfium_rust_build_path_paint_plan(3, true, false, false, &mut output) });
        assert_eq!(0xa5, output);
        // SAFETY: A null output is explicitly supported and rejected without
        // dereferencing it.
        assert!(!unsafe {
            pdfium_rust_build_path_paint_plan(
                PATH_FILL_WINDING,
                true,
                false,
                false,
                core::ptr::null_mut(),
            )
        });
    }

    #[test]
    fn path_matrix_availability_should_match_retained_axis_rules() {
        let cases = [
            ((1.0, 0.0, 0.0, 1.0), true),
            ((0.0, 1.0, 1.0, 0.0), true),
            ((0.0, 0.0, 1.0, 1.0), false),
            ((1.0, 1.0, 0.0, 0.0), false),
            ((1.0, 0.0, 1.0, 1.0), true),
            ((1.0, 1.0, 1.0, 1.0), true),
            ((-0.0, 1.0, 1.0, -0.0), true),
            ((f32::INFINITY, 0.0, 0.0, 1.0), true),
            ((f32::NAN, f32::NAN, f32::NAN, f32::NAN), true),
        ];
        for ((a, b, c, d), expected) in cases {
            assert_eq!(expected, path_matrix_is_available(a, b, c, d));
        }
    }

    #[test]
    fn path_matrix_ffi_should_reject_a_null_output() {
        // SAFETY: A null output is explicitly supported and rejected without
        // dereferencing it.
        assert!(!unsafe {
            pdfium_rust_path_matrix_is_available(1.0, 0.0, 0.0, 1.0, core::ptr::null_mut())
        });
    }

    #[test]
    fn path_fill_options_should_map_all_graph_state_flags() {
        assert_eq!(
            Some(PATH_FILL_NONE),
            build_path_fill_options(PATH_FILL_NONE, true, false, false, false, false)
        );
        assert_eq!(
            Some(PATH_FILL_EVEN_ODD | PATH_OPTIONS_RECT_AA),
            build_path_fill_options(PATH_FILL_EVEN_ODD, true, false, false, false, false)
        );
        assert_eq!(
            Some(
                PATH_FILL_WINDING
                    | PATH_OPTIONS_RECT_AA
                    | PATH_OPTIONS_ALIASED
                    | PATH_OPTIONS_ADJUST_STROKE
                    | PATH_OPTIONS_STROKE
                    | PATH_OPTIONS_TEXT_MODE
            ),
            build_path_fill_options(PATH_FILL_WINDING, true, true, true, true, true)
        );
    }

    #[test]
    fn path_fill_options_should_keep_rect_aa_fill_only() {
        assert_eq!(
            Some(PATH_OPTIONS_STROKE),
            build_path_fill_options(PATH_FILL_NONE, true, false, false, true, false)
        );
        assert_eq!(
            Some(PATH_FILL_WINDING),
            build_path_fill_options(PATH_FILL_WINDING, false, false, false, false, false)
        );
    }

    #[test]
    fn path_fill_options_should_reject_invalid_boundaries() {
        assert_eq!(None, build_path_fill_options(3, true, true, true, true, true));
        let mut output = 0xa5;
        // SAFETY: `output` is one writable byte for the duration of the call.
        assert!(!unsafe {
            pdfium_rust_build_path_fill_options(3, true, true, true, true, true, &mut output)
        });
        assert_eq!(0xa5, output);
        // SAFETY: A null output is explicitly supported and rejected without
        // dereferencing it.
        assert!(!unsafe {
            pdfium_rust_build_path_fill_options(
                PATH_FILL_WINDING,
                true,
                true,
                true,
                true,
                true,
                core::ptr::null_mut(),
            )
        });
    }

    #[test]
    fn text_render_plan_should_preserve_pdf_mode_dispatch() {
        assert_eq!(Some((TEXT_PLAN_SKIP, 0)), build_text_render_plan(false, 0, false, false, true));
        assert_eq!(Some((TEXT_PLAN_SKIP, 0)), build_text_render_plan(true, 3, false, false, true));
        assert_eq!(Some((TEXT_PLAN_TYPE3, 0)), build_text_render_plan(true, 0, true, false, true));
        assert_eq!(Some((TEXT_PLAN_TYPE3, 0)), build_text_render_plan(true, 7, true, false, true));
        assert_eq!(
            Some((TEXT_PLAN_NORMAL, TEXT_PLAN_FILL | TEXT_PLAN_STROKE)),
            build_text_render_plan(true, 2, false, false, true)
        );
        assert_eq!(
            Some((TEXT_PLAN_NORMAL, TEXT_PLAN_FILL)),
            build_text_render_plan(true, 5, false, false, false)
        );
        assert_eq!(
            Some((TEXT_PLAN_NORMAL, TEXT_PLAN_CLIP)),
            build_text_render_plan(true, 0, false, true, true)
        );
        assert_eq!(None, build_text_render_plan(true, -1, false, false, true));
    }

    #[test]
    fn text_render_plan_ffi_should_reject_invalid_boundaries_without_mutation() {
        let mut action = 91_u8;
        let mut bits = 92_u8;
        // SAFETY: The outputs are live; an invalid mode is rejected before
        // either one is written.
        assert!(!unsafe {
            pdfium_rust_build_text_render_plan(true, -1, false, false, true, &mut action, &mut bits)
        });
        assert_eq!((action, bits), (91, 92));
    }

    #[test]
    fn text_pattern_plan_should_consider_only_active_paints() {
        assert!(!text_uses_pattern(false, false, true, true));
        assert!(!text_uses_pattern(true, false, false, true));
        assert!(!text_uses_pattern(false, true, true, false));
        assert!(text_uses_pattern(true, false, true, false));
        assert!(text_uses_pattern(false, true, false, true));
    }

    #[test]
    fn text_backend_plan_should_select_path_for_clip_or_stroke() {
        assert!(!text_uses_path_backend(false, false));
        assert!(text_uses_path_backend(true, false));
        assert!(text_uses_path_backend(false, true));
    }

    #[test]
    fn text_matrix_adjustment_should_require_stroke_and_nonidentity_scale() {
        assert!(!text_needs_device_matrix_adjustment(false, 2.0, 3.0));
        assert!(!text_needs_device_matrix_adjustment(true, 1.0, 1.0));
        assert!(text_needs_device_matrix_adjustment(true, 2.0, 1.0));
        assert!(text_needs_device_matrix_adjustment(true, -0.0, f32::NAN));
    }

    #[test]
    fn text_path_options_should_preserve_combined_stroke_semantics() {
        assert_eq!(0, build_text_path_fill_options(true, false, false, false));
        assert_eq!(
            TEXT_PATH_OPTION_STROKE | TEXT_PATH_OPTION_STROKE_TEXT_MODE,
            build_text_path_fill_options(true, true, false, false)
        );
        assert_eq!(
            TEXT_PATH_OPTION_ADJUST_STROKE | TEXT_PATH_OPTION_ALIASED,
            build_text_path_fill_options(false, true, true, true)
        );
    }

    #[test]
    fn text_backend_execution_should_preserve_priority_and_result() {
        let cases = [
            ((true, true, true), TEXT_BACKEND_PATTERN),
            ((false, true, false), TEXT_BACKEND_PATH),
            ((false, false, true), TEXT_BACKEND_PATH),
            ((false, false, false), TEXT_BACKEND_NORMAL),
        ];
        for ((uses_pattern, is_clip, is_stroke), expected_command) in cases {
            let mut state = TextBackendState { command: 0, result: false, calls: 0 };
            let mut result = true;
            // SAFETY: The context and output remain live for the synchronous
            // callback, which records exactly one command.
            assert!(unsafe {
                pdfium_rust_run_text_backend(
                    uses_pattern,
                    is_clip,
                    is_stroke,
                    (&mut state as *mut TextBackendState).cast(),
                    Some(record_text_backend),
                    &mut result,
                )
            });
            assert_eq!(expected_command, state.command);
            assert_eq!(1, state.calls);
            assert!(!result);
        }
    }

    #[test]
    fn text_backend_execution_should_reject_invalid_boundaries_without_mutation() {
        let mut state = TextBackendState { command: 0, result: true, calls: 0 };
        let mut result = true;
        // SAFETY: A missing callback is rejected before the live context and
        // output are read or written.
        assert!(!unsafe {
            pdfium_rust_run_text_backend(
                false,
                false,
                false,
                (&mut state as *mut TextBackendState).cast(),
                None,
                &mut result,
            )
        });
        assert_eq!(0, state.calls);
        assert!(result);
    }
}
