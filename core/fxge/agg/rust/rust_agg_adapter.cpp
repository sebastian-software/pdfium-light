// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/agg/rust/rust_agg_adapter.h"

namespace {

extern "C" bool pdfium_rust_should_apply_agg_dash_pattern(
    const float* dash_values,
    size_t dash_count,
    float scale,
    bool* output);

thread_local bool g_use_rust_agg_candidate = true;
thread_local std::vector<uint8_t>* g_agg_trace_for_testing = nullptr;

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
    g_agg_trace_for_testing->push_back(static_cast<uint8_t>(should_apply));
  }
}

}  // namespace fxge
