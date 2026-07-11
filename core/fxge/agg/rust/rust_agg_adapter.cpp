// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/agg/rust/rust_agg_adapter.h"

#include <bit>

namespace {

struct RustAggStrokePlan {
  uint8_t line_cap;
  uint8_t line_join;
  uint8_t reserved[2];
  float width;
  float miter_limit;
};

static_assert(sizeof(RustAggStrokePlan) == 12);
static_assert(sizeof(fxge::AggPathPoint) == 12);

extern "C" bool pdfium_rust_should_apply_agg_dash_pattern(
    const float* dash_values,
    size_t dash_count,
    float scale,
    bool* output);

extern "C" bool pdfium_rust_plan_agg_stroke(uint8_t line_cap,
                                            uint8_t line_join,
                                            float line_width,
                                            float scale,
                                            bool has_object_to_device,
                                            float object_x_unit,
                                            float object_y_unit,
                                            float miter_limit,
                                            RustAggStrokePlan* output);

extern "C" bool pdfium_rust_emit_agg_dash_pattern(
    const float* dash_values,
    size_t dash_count,
    float dash_phase,
    float scale,
    void* context,
    fxge::AggDashValueCallback callback,
    float* dash_start);

extern "C" bool pdfium_rust_emit_agg_path(
    size_t point_count,
    bool has_matrix,
    float matrix_a,
    float matrix_b,
    float matrix_c,
    float matrix_d,
    float matrix_e,
    float matrix_f,
    void* context,
    fxge::AggPathPointCallback read_callback,
    fxge::AggPathCommandCallback command_callback);

thread_local bool g_use_rust_agg_candidate = true;
thread_local std::vector<uint8_t>* g_agg_trace_for_testing = nullptr;

void AppendFloatToAggTrace(float value) {
  const uint32_t bits = std::bit_cast<uint32_t>(value);
  for (int shift = 0; shift < 32; shift += 8) {
    g_agg_trace_for_testing->push_back(static_cast<uint8_t>(bits >> shift));
  }
}

bool AggTraceHasRecord(pdfium::span<const uint8_t> trace, uint8_t wanted_tag) {
  size_t offset = 0;
  while (offset < trace.size()) {
    const uint8_t tag = trace[offset];
    if (tag == wanted_tag) {
      return true;
    }
    size_t record_size = 0;
    switch (tag) {
      case 0x44:
        record_size = 2;
        break;
      case 0x53:
        record_size = 11;
        break;
      case 0x56:
      case 0x50:
        record_size = 5;
        break;
      case 0x41:
        if (offset + 3 > trace.size()) {
          return false;
        }
        record_size = 3 + static_cast<size_t>(trace[offset + 2]) * 4;
        break;
      default:
        return false;
    }
    if (record_size > trace.size() - offset) {
      return false;
    }
    offset += record_size;
  }
  return false;
}

}  // namespace

namespace fxge {

std::optional<bool> RustShouldApplyAggDashPattern(
    pdfium::span<const float> dash_array,
    float scale) {
  bool should_apply = false;
  if (!pdfium_rust_should_apply_agg_dash_pattern(
          dash_array.data(), dash_array.size(), scale, &should_apply)) {
    return std::nullopt;
  }
  return should_apply;
}

std::optional<AggStrokePlan> RustPlanAggStroke(uint8_t line_cap,
                                               uint8_t line_join,
                                               float line_width,
                                               float scale,
                                               bool has_object_to_device,
                                               float object_x_unit,
                                               float object_y_unit,
                                               float miter_limit) {
  RustAggStrokePlan plan{};
  if (!pdfium_rust_plan_agg_stroke(line_cap, line_join, line_width, scale,
                                   has_object_to_device, object_x_unit,
                                   object_y_unit, miter_limit, &plan) ||
      plan.line_cap > static_cast<uint8_t>(AggLineCap::kSquare) ||
      plan.line_join > static_cast<uint8_t>(AggLineJoin::kBevel)) {
    return std::nullopt;
  }
  return AggStrokePlan{
      .line_cap = static_cast<AggLineCap>(plan.line_cap),
      .line_join = static_cast<AggLineJoin>(plan.line_join),
      .width = plan.width,
      .miter_limit = plan.miter_limit,
  };
}

std::optional<float> RustEmitAggDashPattern(
    pdfium::span<const float> dash_array,
    float dash_phase,
    float scale,
    void* context,
    AggDashValueCallback callback) {
  float dash_start = 0.0f;
  if (!pdfium_rust_emit_agg_dash_pattern(dash_array.data(), dash_array.size(),
                                         dash_phase, scale, context, callback,
                                         &dash_start)) {
    return std::nullopt;
  }
  return dash_start;
}

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
                     AggPathCommandCallback command_callback) {
  return pdfium_rust_emit_agg_path(point_count, has_matrix, matrix_a, matrix_b,
                                   matrix_c, matrix_d, matrix_e, matrix_f,
                                   context, read_callback, command_callback);
}

bool UseRustAggCandidate() {
  return g_use_rust_agg_candidate;
}

bool SetUseRustAggCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_agg_candidate;
  g_use_rust_agg_candidate = use_candidate;
  return previous;
}

ScopedAggTraceForTesting::ScopedAggTraceForTesting(std::vector<uint8_t>* trace)
    : previous_(g_agg_trace_for_testing) {
  g_agg_trace_for_testing = trace;
}

ScopedAggTraceForTesting::~ScopedAggTraceForTesting() {
  g_agg_trace_for_testing = previous_;
}

void RecordAggDashDecisionForTesting(bool should_apply) {
  if (g_agg_trace_for_testing) {
    g_agg_trace_for_testing->push_back(0x44);
    g_agg_trace_for_testing->push_back(static_cast<uint8_t>(should_apply));
  }
}

void RecordAggStrokePlanForTesting(const AggStrokePlan& plan) {
  if (!g_agg_trace_for_testing) {
    return;
  }
  g_agg_trace_for_testing->push_back(0x53);
  g_agg_trace_for_testing->push_back(static_cast<uint8_t>(plan.line_cap));
  g_agg_trace_for_testing->push_back(static_cast<uint8_t>(plan.line_join));
  for (float value : {plan.width, plan.miter_limit}) {
    AppendFloatToAggTrace(value);
  }
}

void RecordAggDashValueForTesting(float value) {
  if (g_agg_trace_for_testing) {
    g_agg_trace_for_testing->push_back(0x56);
    AppendFloatToAggTrace(value);
  }
}

void RecordAggDashStartForTesting(float value) {
  if (g_agg_trace_for_testing) {
    g_agg_trace_for_testing->push_back(0x50);
    AppendFloatToAggTrace(value);
  }
}

void RecordAggPathCommandForTesting(uint8_t command,
                                    pdfium::span<const float> coordinates) {
  if (!g_agg_trace_for_testing || coordinates.size() > UINT8_MAX) {
    return;
  }
  g_agg_trace_for_testing->push_back(0x41);
  g_agg_trace_for_testing->push_back(command);
  g_agg_trace_for_testing->push_back(static_cast<uint8_t>(coordinates.size()));
  for (float value : coordinates) {
    AppendFloatToAggTrace(value);
  }
}

bool AggTraceHasDashValuesForTesting(pdfium::span<const uint8_t> trace) {
  return AggTraceHasRecord(trace, 0x56);
}

bool AggTraceHasPathCommandsForTesting(pdfium::span<const uint8_t> trace) {
  return AggTraceHasRecord(trace, 0x41);
}

}  // namespace fxge
