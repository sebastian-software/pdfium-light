// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/render/rust/rust_render_adapter.h"

#include <limits>

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
extern "C" bool pdfium_rust_build_render_layer_plan(bool has_options,
                                                    bool has_last_matrix,
                                                    uint8_t* output);
extern "C" bool pdfium_rust_build_render_layer_completion(
    bool limited_image_cache,
    bool stopped,
    uint8_t* output);
extern "C" bool pdfium_rust_run_render_layers(
    uint32_t layer_count,
    void* context,
    pdfium::rust::RenderLayerCallback callback);
extern "C" bool pdfium_rust_build_path_paint_plan(uint8_t fill_type,
                                                  bool stroke,
                                                  bool forced_color,
                                                  bool convert_fill_to_stroke,
                                                  uint8_t* output);

thread_local bool g_use_rust_render_candidate = true;
thread_local std::vector<uint8_t>* g_render_trace_for_testing = nullptr;

constexpr uint8_t kPageObjectRenderCommandTraceBase = 0x10;
constexpr uint8_t kRenderLayerPlanTraceBase = 0x20;
constexpr uint8_t kRenderLayerCompletionTraceBase = 0x30;
constexpr uint8_t kPathPaintPlanTraceBase = 0x40;

constexpr uint8_t kPathPlanFillMask = 0x03;
constexpr uint8_t kPathPlanStroke = 1u << 2;
constexpr uint8_t kPathPlanDraw = 1u << 3;

static_assert(static_cast<uint8_t>(CFX_FillRenderOptions::FillType::kNoFill) ==
              0);
static_assert(static_cast<uint8_t>(CFX_FillRenderOptions::FillType::kEvenOdd) ==
              1);
static_assert(static_cast<uint8_t>(CFX_FillRenderOptions::FillType::kWinding) ==
              2);

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

std::optional<RenderLayerPlan> BuildRustRenderLayerPlan(bool has_options,
                                                        bool has_last_matrix) {
  uint8_t bits = 0;
  if (!pdfium_rust_build_render_layer_plan(has_options, has_last_matrix,
                                           &bits)) {
    return std::nullopt;
  }
  constexpr uint8_t kAllowedBits =
      static_cast<uint8_t>(RenderLayerPlanBit::kSetOptions) |
      static_cast<uint8_t>(RenderLayerPlanBit::kApplyLastMatrix);
  if ((bits & ~kAllowedBits) != 0 ||
      (!has_options &&
       (bits & static_cast<uint8_t>(RenderLayerPlanBit::kSetOptions)) != 0) ||
      (!has_last_matrix &&
       (bits & static_cast<uint8_t>(RenderLayerPlanBit::kApplyLastMatrix)) !=
           0)) {
    return std::nullopt;
  }
  return RenderLayerPlan(bits);
}

std::optional<RenderLayerCompletion> BuildRustRenderLayerCompletion(
    bool limited_image_cache,
    bool stopped) {
  uint8_t bits = 0;
  if (!pdfium_rust_build_render_layer_completion(limited_image_cache, stopped,
                                                 &bits)) {
    return std::nullopt;
  }
  constexpr uint8_t kAllowedBits =
      static_cast<uint8_t>(RenderLayerCompletionBit::kOptimizeCache) |
      static_cast<uint8_t>(RenderLayerCompletionBit::kStop);
  if ((bits & ~kAllowedBits) != 0 ||
      (!limited_image_cache &&
       (bits &
        static_cast<uint8_t>(RenderLayerCompletionBit::kOptimizeCache)) != 0) ||
      (!stopped &&
       (bits & static_cast<uint8_t>(RenderLayerCompletionBit::kStop)) != 0)) {
    return std::nullopt;
  }
  return RenderLayerCompletion(bits);
}

bool RunRustRenderLayers(size_t layer_count,
                         void* context,
                         RenderLayerCallback callback) {
  if (layer_count > std::numeric_limits<uint32_t>::max() || !context ||
      !callback) {
    return false;
  }
  return pdfium_rust_run_render_layers(static_cast<uint32_t>(layer_count),
                                       context, callback);
}

std::optional<PathPaintPlan> BuildRustPathPaintPlan(
    CFX_FillRenderOptions::FillType fill_type,
    bool stroke,
    bool forced_color,
    bool convert_fill_to_stroke) {
  uint8_t bits = 0;
  if (!pdfium_rust_build_path_paint_plan(static_cast<uint8_t>(fill_type),
                                         stroke, forced_color,
                                         convert_fill_to_stroke, &bits)) {
    return std::nullopt;
  }
  constexpr uint8_t kAllowedBits =
      kPathPlanFillMask | kPathPlanStroke | kPathPlanDraw;
  if ((bits & ~kAllowedBits) != 0) {
    return std::nullopt;
  }
  const uint8_t planned_fill = bits & kPathPlanFillMask;
  if (planned_fill >
      static_cast<uint8_t>(CFX_FillRenderOptions::FillType::kWinding)) {
    return std::nullopt;
  }
  return PathPaintPlan(
      static_cast<CFX_FillRenderOptions::FillType>(planned_fill),
      (bits & kPathPlanStroke) != 0, (bits & kPathPlanDraw) != 0);
}

ScopedRenderTraceForTesting::ScopedRenderTraceForTesting(
    std::vector<uint8_t>* trace)
    : previous_(g_render_trace_for_testing) {
  g_render_trace_for_testing = trace;
}

ScopedRenderTraceForTesting::~ScopedRenderTraceForTesting() {
  g_render_trace_for_testing = previous_;
}

void RecordRenderTraceForTesting(ObjectListCommand command) {
  if (g_render_trace_for_testing) {
    g_render_trace_for_testing->push_back(static_cast<uint8_t>(command));
  }
}

void RecordRenderTraceForTesting(PageObjectRenderCommand command) {
  if (g_render_trace_for_testing) {
    g_render_trace_for_testing->push_back(kPageObjectRenderCommandTraceBase +
                                          static_cast<uint8_t>(command));
  }
}

void RecordRenderTraceForTesting(const RenderLayerPlan& plan) {
  if (g_render_trace_for_testing) {
    uint8_t bits = 0;
    if (plan.Has(RenderLayerPlanBit::kSetOptions)) {
      bits |= static_cast<uint8_t>(RenderLayerPlanBit::kSetOptions);
    }
    if (plan.Has(RenderLayerPlanBit::kApplyLastMatrix)) {
      bits |= static_cast<uint8_t>(RenderLayerPlanBit::kApplyLastMatrix);
    }
    g_render_trace_for_testing->push_back(kRenderLayerPlanTraceBase + bits);
  }
}

void RecordRenderTraceForTesting(const RenderLayerCompletion& completion) {
  if (g_render_trace_for_testing) {
    uint8_t bits = 0;
    if (completion.Has(RenderLayerCompletionBit::kOptimizeCache)) {
      bits |= static_cast<uint8_t>(RenderLayerCompletionBit::kOptimizeCache);
    }
    if (completion.Has(RenderLayerCompletionBit::kStop)) {
      bits |= static_cast<uint8_t>(RenderLayerCompletionBit::kStop);
    }
    g_render_trace_for_testing->push_back(kRenderLayerCompletionTraceBase +
                                          bits);
  }
}

void RecordRenderTraceForTesting(const PathPaintPlan& plan) {
  if (g_render_trace_for_testing) {
    uint8_t bits = static_cast<uint8_t>(plan.fill_type());
    if (plan.stroke()) {
      bits |= kPathPlanStroke;
    }
    if (plan.should_draw()) {
      bits |= kPathPlanDraw;
    }
    g_render_trace_for_testing->push_back(kPathPaintPlanTraceBase + bits);
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
