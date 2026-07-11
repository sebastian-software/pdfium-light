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

}  // namespace fxge
