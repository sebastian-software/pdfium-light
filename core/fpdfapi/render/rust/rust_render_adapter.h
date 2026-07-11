// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_
#define CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_

#include <stdint.h>

#include <cstddef>
#include <optional>
#include <vector>

#include "core/fxge/cfx_fillrenderoptions.h"

namespace pdfium::rust {

enum class RenderPlanBit : uint32_t {
  kAnnotations = 1u << 0,
  kClearType = 1u << 1,
  kNoNativeText = 1u << 2,
  kGrayscale = 1u << 3,
  kConvertFillToStroke = 1u << 4,
  kLimitedImageCache = 1u << 5,
  kForceHalftone = 1u << 6,
  kPrinting = 1u << 7,
  kNoTextSmoothing = 1u << 8,
  kNoImageSmoothing = 1u << 9,
  kNoPathSmoothing = 1u << 10,
  kForcedColor = 1u << 11,
  kRestoreDevice = 1u << 12,
};

enum class PageObjectRenderCommand : uint8_t {
  kText = 1,
  kPath,
  kImage,
  kShading,
  kForm,
};

enum class ObjectListCommand : uint8_t {
  kStop = 1,
  kSkip,
  kRender,
};

enum class RenderLayerPlanBit : uint8_t {
  kSetOptions = 1u << 0,
  kApplyLastMatrix = 1u << 1,
};

enum class RenderLayerCompletionBit : uint8_t {
  kOptimizeCache = 1u << 0,
  kStop = 1u << 1,
};

class RenderLayerPlan final {
 public:
  explicit RenderLayerPlan(uint8_t bits) : bits_(bits) {}

  bool Has(RenderLayerPlanBit bit) const {
    return (bits_ & static_cast<uint8_t>(bit)) != 0;
  }

 private:
  uint8_t bits_;
};

class RenderLayerCompletion final {
 public:
  explicit RenderLayerCompletion(uint8_t bits) : bits_(bits) {}

  bool Has(RenderLayerCompletionBit bit) const {
    return (bits_ & static_cast<uint8_t>(bit)) != 0;
  }

 private:
  uint8_t bits_;
};

class PathPaintPlan final {
 public:
  PathPaintPlan(CFX_FillRenderOptions::FillType fill_type,
                bool stroke,
                bool should_draw)
      : fill_type_(fill_type), stroke_(stroke), should_draw_(should_draw) {}

  CFX_FillRenderOptions::FillType fill_type() const { return fill_type_; }
  bool stroke() const { return stroke_; }
  bool should_draw() const { return should_draw_; }

 private:
  CFX_FillRenderOptions::FillType fill_type_;
  bool stroke_;
  bool should_draw_;
};

enum class TextRenderAction : uint8_t { kSkip = 1, kType3, kNormal };

class TextRenderPlan final {
 public:
  TextRenderPlan(TextRenderAction action, uint8_t bits)
      : action_(action), bits_(bits) {}
  TextRenderAction action() const { return action_; }
  bool fill() const { return bits_ & 1u; }
  bool stroke() const { return bits_ & 2u; }
  bool clip() const { return bits_ & 4u; }

 private:
  TextRenderAction action_;
  uint8_t bits_;
};

class RenderRequestPlan final {
 public:
  explicit RenderRequestPlan(uint32_t bits) : bits_(bits) {}

  bool Has(RenderPlanBit bit) const {
    return (bits_ & static_cast<uint32_t>(bit)) != 0;
  }

 private:
  uint32_t bits_;
};

std::optional<RenderRequestPlan> BuildRustRenderRequestPlan(
    int flags,
    bool has_color_scheme,
    bool restore_device);

std::optional<PageObjectRenderCommand> BuildRustPageObjectRenderCommand(
    uint32_t page_object_type);

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
                                                            float clip_top);

std::optional<RenderLayerPlan> BuildRustRenderLayerPlan(bool has_options,
                                                        bool has_last_matrix);
std::optional<RenderLayerCompletion> BuildRustRenderLayerCompletion(
    bool limited_image_cache,
    bool stopped);

using RenderLayerCallback = bool (*)(void* context, uint32_t layer_index);
bool RunRustRenderLayers(size_t layer_count,
                         void* context,
                         RenderLayerCallback callback);
std::optional<PathPaintPlan> BuildRustPathPaintPlan(
    CFX_FillRenderOptions::FillType fill_type,
    bool stroke,
    bool forced_color,
    bool convert_fill_to_stroke);
std::optional<bool> RustPathMatrixIsAvailable(float a,
                                              float b,
                                              float c,
                                              float d);
std::optional<CFX_FillRenderOptions> BuildRustPathFillOptions(
    CFX_FillRenderOptions::FillType fill_type,
    bool rect_aa,
    bool no_path_smooth,
    bool stroke_adjust,
    bool stroke,
    bool type3_char);
std::optional<TextRenderPlan> BuildRustTextRenderPlan(
    bool has_char_codes,
    int8_t text_mode,
    bool is_type3,
    bool has_clipping_path,
    bool font_has_face);
std::optional<bool> RustTextUsesPattern(bool is_fill,
                                        bool is_stroke,
                                        bool fill_is_pattern,
                                        bool stroke_is_pattern);
std::optional<bool> RustTextUsesPathBackend(bool is_clip, bool is_stroke);
std::optional<bool> RustTextNeedsDeviceMatrixAdjustment(bool is_stroke,
                                                        float ctm_a,
                                                        float ctm_d);

class ScopedRenderTraceForTesting final {
 public:
  explicit ScopedRenderTraceForTesting(std::vector<uint8_t>* trace);
  ScopedRenderTraceForTesting(const ScopedRenderTraceForTesting&) = delete;
  ScopedRenderTraceForTesting& operator=(const ScopedRenderTraceForTesting&) =
      delete;
  ~ScopedRenderTraceForTesting();

 private:
  std::vector<uint8_t>* previous_;
};

void RecordRenderTraceForTesting(ObjectListCommand command);
void RecordRenderTraceForTesting(PageObjectRenderCommand command);
void RecordRenderTraceForTesting(const RenderLayerPlan& plan);
void RecordRenderTraceForTesting(const RenderLayerCompletion& completion);
void RecordRenderTraceForTesting(const PathPaintPlan& plan);
void RecordPathMatrixAvailabilityForTesting(bool available);
void RecordPathFillOptionsForTesting(const CFX_FillRenderOptions& options);
void RecordTextRenderPlanForTesting(const TextRenderPlan& plan);

// Production defaults to the Rust candidate. The setter exists only so the
// same-process differential harness can keep the retained C++ oracle isolated.
bool UseRustRenderCandidate();
bool SetUseRustRenderCandidateForTesting(bool use_candidate);

}  // namespace pdfium::rust

#endif  // CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_
