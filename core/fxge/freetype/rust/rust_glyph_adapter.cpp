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

thread_local bool g_use_rust_glyph_candidate = true;

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

bool UseRustGlyphCandidate() {
  return g_use_rust_glyph_candidate;
}

bool SetUseRustGlyphCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_glyph_candidate;
  g_use_rust_glyph_candidate = use_candidate;
  return previous;
}

}  // namespace fxge
