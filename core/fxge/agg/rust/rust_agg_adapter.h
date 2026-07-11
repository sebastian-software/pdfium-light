// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
#define CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_

#include <stddef.h>
#include <stdint.h>

#include <optional>
#include <vector>

#include "core/fxcrt/span.h"

namespace fxge {

enum class AggLineCap : uint8_t { kButt = 0, kRound = 1, kSquare = 2 };
enum class AggLineJoin : uint8_t { kMiter = 0, kRound = 1, kBevel = 2 };
enum class AggFillRule : uint8_t { kEvenOdd = 0, kNonZero = 1 };
enum class AggStrokeMode : uint8_t { kNone = 0, kZeroArea = 1, kNormal = 2 };

struct AggStrokePlan {
  AggLineCap line_cap;
  AggLineJoin line_join;
  float width;
  float miter_limit;
};

struct AggPathPoint {
  float x;
  float y;
  uint8_t point_type;
  uint8_t close_figure;
  uint8_t reserved[2];
};

struct AggPathDrawPlan {
  bool draw_fill;
  AggStrokeMode stroke_mode;
  AggFillRule fill_rule;
};

using AggDashValueCallback = void (*)(void* context, float value);
using AggPathPointCallback = void (*)(void* context,
                                      size_t index,
                                      AggPathPoint* point);
using AggPathCommandCallback = void (*)(void* context,
                                        uint8_t command,
                                        const float* coordinates,
                                        size_t coordinate_count);

std::optional<bool> RustShouldApplyAggDashPattern(
    pdfium::span<const float> dash_array,
    float scale);

std::optional<AggStrokePlan> RustPlanAggStroke(uint8_t line_cap,
                                               uint8_t line_join,
                                               float line_width,
                                               float scale,
                                               bool has_object_to_device,
                                               float object_x_unit,
                                               float object_y_unit,
                                               float miter_limit);

std::optional<AggPathDrawPlan> RustPlanAggPathDraw(uint8_t fill_type,
                                                   uint32_t fill_color,
                                                   bool has_graph_state,
                                                   uint32_t stroke_color,
                                                   bool zero_area);

std::optional<float> RustEmitAggDashPattern(
    pdfium::span<const float> dash_array,
    float dash_phase,
    float scale,
    void* context,
    AggDashValueCallback callback);

bool RustEmitAggPath(size_t point_count,
                     bool has_matrix,
                     float matrix_a,
                     float matrix_b,
                     float matrix_c,
                     float matrix_d,
                     float matrix_e,
                     float matrix_f,
                     void* context,
                     AggPathPointCallback read_callback,
                     AggPathCommandCallback command_callback);

bool UseRustAggCandidate();
bool SetUseRustAggCandidateForTesting(bool use_candidate);

class ScopedAggTraceForTesting final {
 public:
  explicit ScopedAggTraceForTesting(std::vector<uint8_t>* trace);
  ScopedAggTraceForTesting(const ScopedAggTraceForTesting&) = delete;
  ScopedAggTraceForTesting& operator=(const ScopedAggTraceForTesting&) = delete;
  ~ScopedAggTraceForTesting();

 private:
  std::vector<uint8_t>* previous_;
};

void RecordAggDashDecisionForTesting(bool should_apply);
void RecordAggStrokePlanForTesting(const AggStrokePlan& plan);
void RecordAggDashValueForTesting(float value);
void RecordAggDashStartForTesting(float value);
void RecordAggPathCommandForTesting(uint8_t command,
                                    pdfium::span<const float> coordinates);
bool AggTraceHasDashValuesForTesting(pdfium::span<const uint8_t> trace);
bool AggTraceHasPathCommandsForTesting(pdfium::span<const uint8_t> trace);

}  // namespace fxge

#endif  // CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
