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

#[cfg(test)]
mod tests {
    use super::*;

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
}
