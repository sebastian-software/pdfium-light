// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/render/rust/rust_render_adapter.h"

#include "public/fpdfview.h"

namespace {

extern "C" bool pdfium_rust_build_render_request_plan(uint32_t flags,
                                                      bool has_color_scheme,
                                                      bool restore_device,
                                                      uint32_t* output);

static_assert(FPDF_ANNOT == 0x01);
static_assert(FPDF_LCD_TEXT == 0x02);
static_assert(FPDF_NO_NATIVETEXT == 0x04);
static_assert(FPDF_GRAYSCALE == 0x08);
static_assert(FPDF_CONVERT_FILL_TO_STROKE == 0x20);
static_assert(FPDF_RENDER_LIMITEDIMAGECACHE == 0x200);
static_assert(FPDF_RENDER_FORCEHALFTONE == 0x400);
static_assert(FPDF_PRINTING == 0x800);
static_assert(FPDF_RENDER_NO_SMOOTHTEXT == 0x1000);
static_assert(FPDF_RENDER_NO_SMOOTHIMAGE == 0x2000);
static_assert(FPDF_RENDER_NO_SMOOTHPATH == 0x4000);

}  // namespace

namespace pdfium::rust {

std::optional<RenderRequestPlan> BuildRustRenderRequestPlan(
    int flags,
    bool has_color_scheme,
    bool restore_device) {
  uint32_t bits = 0;
  if (!pdfium_rust_build_render_request_plan(static_cast<uint32_t>(flags),
                                             has_color_scheme, restore_device,
                                             &bits)) {
    return std::nullopt;
  }
  return RenderRequestPlan(bits);
}

}  // namespace pdfium::rust
