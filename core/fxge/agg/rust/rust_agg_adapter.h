// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
#define CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_

#include <stdint.h>

#include <optional>
#include <vector>

#include "core/fxcrt/span.h"

namespace fxge {

enum class AggLineCap : uint8_t { kButt = 0, kRound = 1, kSquare = 2 };
enum class AggLineJoin : uint8_t { kMiter = 0, kRound = 1, kBevel = 2 };

struct AggStrokePlan {
  AggLineCap line_cap;
  AggLineJoin line_join;
  float width;
  float miter_limit;
};

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

}  // namespace fxge

#endif  // CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
