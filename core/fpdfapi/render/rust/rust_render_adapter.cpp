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
extern "C" bool pdfium_rust_build_page_object_render_command(
    uint32_t page_object_type,
    uint8_t* output);
extern "C" bool pdfium_rust_build_object_list_command(bool is_stop_object,
                                                      bool is_present,
                                                      bool is_active,
                                                      float object_left,
                                                      float object_bottom,
                                                      float object_right,
                                                      float object_top,
                                                      float clip_left,
                                                      float clip_bottom,
                                                      float clip_right,
                                                      float clip_top,
                                                      uint8_t* output);

thread_local bool g_use_rust_render_candidate = true;

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

std::optional<PageObjectRenderCommand> BuildRustPageObjectRenderCommand(
    uint32_t page_object_type) {
  uint8_t command = 0;
  if (!pdfium_rust_build_page_object_render_command(page_object_type,
                                                    &command)) {
    return std::nullopt;
  }
  switch (command) {
    case static_cast<uint8_t>(PageObjectRenderCommand::kText):
      return PageObjectRenderCommand::kText;
    case static_cast<uint8_t>(PageObjectRenderCommand::kPath):
      return PageObjectRenderCommand::kPath;
    case static_cast<uint8_t>(PageObjectRenderCommand::kImage):
      return PageObjectRenderCommand::kImage;
    case static_cast<uint8_t>(PageObjectRenderCommand::kShading):
      return PageObjectRenderCommand::kShading;
    case static_cast<uint8_t>(PageObjectRenderCommand::kForm):
      return PageObjectRenderCommand::kForm;
    default:
      return std::nullopt;
  }
}

std::optional<ObjectListCommand> BuildRustObjectListCommand(bool is_stop_object,
                                                            bool is_present,
                                                            bool is_active,
                                                            float object_left,
                                                            float object_bottom,
                                                            float object_right,
                                                            float object_top,
                                                            float clip_left,
                                                            float clip_bottom,
                                                            float clip_right,
                                                            float clip_top) {
  uint8_t command = 0;
  if (!pdfium_rust_build_object_list_command(
          is_stop_object, is_present, is_active, object_left, object_bottom,
          object_right, object_top, clip_left, clip_bottom, clip_right,
          clip_top, &command)) {
    return std::nullopt;
  }
  switch (command) {
    case static_cast<uint8_t>(ObjectListCommand::kStop):
      return ObjectListCommand::kStop;
    case static_cast<uint8_t>(ObjectListCommand::kSkip):
      return ObjectListCommand::kSkip;
    case static_cast<uint8_t>(ObjectListCommand::kRender):
      return ObjectListCommand::kRender;
    default:
      return std::nullopt;
  }
}

bool UseRustRenderCandidate() {
  return g_use_rust_render_candidate;
}

bool SetUseRustRenderCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_render_candidate;
  g_use_rust_render_candidate = use_candidate;
  return previous;
}

}  // namespace pdfium::rust
