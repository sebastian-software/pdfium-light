// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
#define CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_

#include <optional>

#include "core/fxcrt/span.h"

namespace fxge {

std::optional<bool> RustShouldApplyAggDashPattern(
    pdfium::span<const float> dash_array,
    float scale);

bool UseRustAggCandidate();
bool SetUseRustAggCandidateForTesting(bool use_candidate);

}  // namespace fxge

#endif  // CORE_FXGE_AGG_RUST_RUST_AGG_ADAPTER_H_
