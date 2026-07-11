// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/freetype/rust/rust_glyph_adapter.h"

namespace {

extern "C" bool pdfium_rust_fill_glyph_cache_key(int32_t matrix_a,
                                                 int32_t matrix_b,
                                                 int32_t matrix_c,
                                                 int32_t matrix_d,
                                                 int32_t destination_width,
                                                 int32_t anti_alias,
                                                 bool has_substitution,
                                                 int32_t weight,
                                                 int32_t italic_angle,
                                                 bool vertical,
                                                 bool native_text,
                                                 uint32_t* output,
                                                 size_t output_capacity,
                                                 size_t* output_len);
extern "C" bool pdfium_rust_plan_glyph_origin(int32_t origin_x,
                                              int32_t origin_y,
                                              int32_t glyph_left,
                                              int32_t glyph_top,
                                              int32_t offset_x,
                                              int32_t offset_y,
                                              uint8_t* output_valid,
                                              int32_t* output_x,
                                              int32_t* output_y);

thread_local bool g_use_rust_glyph_candidate = true;
thread_local std::vector<uint8_t>* g_glyph_trace_for_testing = nullptr;

}  // namespace

namespace fxge {

std::optional<size_t> RustFillGlyphCacheKey(const GlyphCacheKeyInputs& inputs,
                                            pdfium::span<uint32_t> output) {
  size_t output_len = 0;
  if (!pdfium_rust_fill_glyph_cache_key(
          inputs.matrix_a, inputs.matrix_b, inputs.matrix_c, inputs.matrix_d,
          inputs.destination_width, inputs.anti_alias, inputs.has_substitution,
          inputs.weight, inputs.italic_angle, inputs.vertical,
          inputs.native_text, output.data(), output.size(), &output_len) ||
      output_len > output.size()) {
    return std::nullopt;
  }
  return output_len;
}

std::optional<GlyphOriginPlan> RustPlanGlyphOrigin(int32_t origin_x,
                                                   int32_t origin_y,
                                                   int32_t glyph_left,
                                                   int32_t glyph_top,
                                                   int32_t offset_x,
                                                   int32_t offset_y) {
  uint8_t valid = 0;
  int32_t x = 0;
  int32_t y = 0;
  if (!pdfium_rust_plan_glyph_origin(origin_x, origin_y, glyph_left, glyph_top,
                                     offset_x, offset_y, &valid, &x, &y) ||
      valid > 1) {
    return std::nullopt;
  }
  return GlyphOriginPlan{.valid = !!valid, .x = x, .y = y};
}

bool UseRustGlyphCandidate() {
  return g_use_rust_glyph_candidate;
}

bool SetUseRustGlyphCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_glyph_candidate;
  g_use_rust_glyph_candidate = use_candidate;
  return previous;
}

ScopedGlyphTraceForTesting::ScopedGlyphTraceForTesting(
    std::vector<uint8_t>* trace)
    : previous_(g_glyph_trace_for_testing) {
  g_glyph_trace_for_testing = trace;
}

ScopedGlyphTraceForTesting::~ScopedGlyphTraceForTesting() {
  g_glyph_trace_for_testing = previous_;
}

void RecordGlyphCacheKeyForTesting(pdfium::span<const uint32_t> key) {
  if (!g_glyph_trace_for_testing || key.size() > UINT8_MAX) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x4b);
  g_glyph_trace_for_testing->push_back(static_cast<uint8_t>(key.size()));
  for (uint32_t word : key) {
    for (int shift = 0; shift < 32; shift += 8) {
      g_glyph_trace_for_testing->push_back(static_cast<uint8_t>(word >> shift));
    }
  }
}

}  // namespace fxge
