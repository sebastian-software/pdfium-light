// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_FREETYPE_RUST_RUST_GLYPH_ADAPTER_H_
#define CORE_FXGE_FREETYPE_RUST_RUST_GLYPH_ADAPTER_H_

#include <stddef.h>
#include <stdint.h>

#include <optional>
#include <vector>

#include "core/fxcrt/span.h"

namespace fxge {

struct GlyphCacheKeyInputs {
  int32_t matrix_a;
  int32_t matrix_b;
  int32_t matrix_c;
  int32_t matrix_d;
  int32_t destination_width;
  int32_t anti_alias;
  bool has_substitution;
  int32_t weight;
  int32_t italic_angle;
  bool vertical;
  bool native_text;
};

struct GlyphOriginPlan {
  bool valid;
  int32_t x;
  int32_t y;
};

std::optional<size_t> RustFillGlyphCacheKey(const GlyphCacheKeyInputs& inputs,
                                            pdfium::span<uint32_t> output);
std::optional<GlyphOriginPlan> RustPlanGlyphOrigin(int32_t origin_x,
                                                   int32_t origin_y,
                                                   int32_t glyph_left,
                                                   int32_t glyph_top,
                                                   int32_t offset_x,
                                                   int32_t offset_y);

bool UseRustGlyphCandidate();
bool SetUseRustGlyphCandidateForTesting(bool use_candidate);

class ScopedGlyphTraceForTesting final {
 public:
  explicit ScopedGlyphTraceForTesting(std::vector<uint8_t>* trace);
  ScopedGlyphTraceForTesting(const ScopedGlyphTraceForTesting&) = delete;
  ScopedGlyphTraceForTesting& operator=(const ScopedGlyphTraceForTesting&) =
      delete;
  ~ScopedGlyphTraceForTesting();

 private:
  std::vector<uint8_t>* previous_;
};

void RecordGlyphCacheKeyForTesting(pdfium::span<const uint32_t> key);

}  // namespace fxge

#endif  // CORE_FXGE_FREETYPE_RUST_RUST_GLYPH_ADAPTER_H_
