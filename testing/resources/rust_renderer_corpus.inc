// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Versioned renderer differential corpus. Keep normal, boundary, image, path,
// text, transparency, and annotation coverage represented as the Rust-owned
// rendering boundary expands.
//
// Arguments: stable case name, fixture, zero-based page index, render flags.
RUST_RENDER_CASE(HelloWorld, "hello_world.pdf", 0, 0)
RUST_RENDER_CASE(HelloWorldLcdText, "hello_world.pdf", 0, FPDF_LCD_TEXT)
RUST_RENDER_CASE(HelloWorldNoNativeText, "hello_world.pdf", 0,
                 FPDF_NO_NATIVETEXT)
RUST_RENDER_CASE(HelloWorldGrayscale, "hello_world.pdf", 0, FPDF_GRAYSCALE)
RUST_RENDER_CASE(HelloWorldLimitedImageCache, "hello_world.pdf", 0,
                 FPDF_RENDER_LIMITEDIMAGECACHE)
RUST_RENDER_CASE(HelloWorldForceHalftone, "hello_world.pdf", 0,
                 FPDF_RENDER_FORCEHALFTONE)
RUST_RENDER_CASE(HelloWorldPrinting, "hello_world.pdf", 0, FPDF_PRINTING)
RUST_RENDER_CASE(HelloWorldNoSmoothText, "hello_world.pdf", 0,
                 FPDF_RENDER_NO_SMOOTHTEXT)
RUST_RENDER_CASE(HelloWorldNoSmoothImage, "hello_world.pdf", 0,
                 FPDF_RENDER_NO_SMOOTHIMAGE)
RUST_RENDER_CASE(HelloWorldNoSmoothPath, "hello_world.pdf", 0,
                 FPDF_RENDER_NO_SMOOTHPATH)
RUST_RENDER_CASE(ManyRectangles, "many_rectangles.pdf", 0, 0)
RUST_RENDER_CASE(TextAndFonts, "text_font.pdf", 0, 0)
RUST_RENDER_CASE(ImageAndTransparency, "matte.pdf", 0, 0)
RUST_RENDER_CASE(AnnotationAppearance, "bug_1658.pdf", 0, FPDF_ANNOT)
