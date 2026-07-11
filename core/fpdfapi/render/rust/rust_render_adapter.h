// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_
#define CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_

#include <stdint.h>

#include <optional>
#include <vector>

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

// Production defaults to the Rust candidate. The setter exists only so the
// same-process differential harness can keep the retained C++ oracle isolated.
bool UseRustRenderCandidate();
bool SetUseRustRenderCandidateForTesting(bool use_candidate);

}  // namespace pdfium::rust

#endif  // CORE_FPDFAPI_RENDER_RUST_RUST_RENDER_ADAPTER_H_
