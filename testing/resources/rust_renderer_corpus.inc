// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Versioned renderer differential corpus. Keep normal, boundary, image, path,
// text, transparency, and annotation coverage represented as the Rust-owned
// rendering boundary expands.
//
// Arguments: stable case name, fixture, zero-based page index, render flags.
RUST_RENDER_CASE(HelloWorld, "hello_world.pdf", 0, 0)
RUST_RENDER_CASE(ManyRectangles, "many_rectangles.pdf", 0, 0)
RUST_RENDER_CASE(TextAndFonts, "text_font.pdf", 0, 0)
RUST_RENDER_CASE(ImageAndTransparency, "matte.pdf", 0, 0)
RUST_RENDER_CASE(AnnotationAppearance, "bug_1658.pdf", 0, FPDF_ANNOT)
