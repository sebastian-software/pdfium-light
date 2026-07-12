// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/freetype/rust/rust_glyph_adapter.h"

#include <bit>

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
extern "C" bool pdfium_rust_plan_glyph_device_origin(float device_x,
                                                     float device_y,
                                                     bool anti_alias_is_lcd,
                                                     int32_t* output_x,
                                                     int32_t* output_y);
extern "C" bool pdfium_rust_plan_glyph_bounds(
    size_t glyph_count,
    bool anti_alias_is_lcd,
    void* context,
    fxge::ReadGlyphBoundsCallback read_bounds,
    int32_t* output_left,
    int32_t* output_top,
    int32_t* output_right,
    int32_t* output_bottom);
extern "C" bool pdfium_rust_plan_glyph_bitmap_lookup(bool glyph_is_valid,
                                                     bool native_text,
                                                     bool native_cache_hit,
                                                     uint8_t* output_action);
extern "C" bool pdfium_rust_plan_glyph_path_cache_key(
    uint32_t glyph_index,
    int32_t destination_width,
    bool has_substitution,
    int32_t weight,
    int32_t italic_angle,
    bool font_is_vertical,
    uint32_t* output_glyph_index,
    int32_t* output_destination_width,
    int32_t* output_weight,
    int32_t* output_italic_angle,
    uint8_t* output_vertical);
extern "C" bool pdfium_rust_plan_glyph_width_cache_key(
    uint32_t glyph_index,
    int32_t destination_width,
    int32_t weight,
    uint32_t* output_glyph_index,
    int32_t* output_destination_width,
    int32_t* output_weight);
extern "C" bool pdfium_rust_plan_freetype_glyph_load(
    bool is_render,
    bool is_tt_ot,
    bool is_tricky,
    uint8_t* output_no_hinting,
    uint8_t* output_pedantic,
    uint8_t* output_retry_without_hinting);

thread_local bool g_use_rust_glyph_candidate = true;
thread_local std::vector<uint8_t>* g_glyph_trace_for_testing = nullptr;

void AppendTraceWord(uint32_t word) {
  for (int shift = 0; shift < 32; shift += 8) {
    g_glyph_trace_for_testing->push_back(static_cast<uint8_t>(word >> shift));
  }
}

bool GlyphTraceHasEvent(pdfium::span<const uint8_t> trace,
                        uint8_t expected_marker) {
  while (!trace.empty()) {
    const uint8_t marker = trace.front();
    size_t event_size = 0;
    if (marker == 0x4b && trace.size() >= 2) {
      event_size = 2 + static_cast<size_t>(trace[1]) * 4;
    } else if (marker == 0x4f) {
      event_size = 10;
    } else if (marker == 0x44) {
      event_size = 18;
    } else if (marker == 0x42) {
      event_size = 17;
    } else if (marker == 0x4c) {
      event_size = 5;
    } else if (marker == 0x46) {
      event_size = 5;
    } else {
      return false;
    }
    if (trace.size() < event_size) {
      return false;
    }
    if (marker == expected_marker) {
      return marker != 0x4f || trace[1] <= 1;
    }
    trace = trace.subspan(event_size);
  }
  return false;
}

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

std::optional<GlyphOriginPlan> RustPlanGlyphDeviceOrigin(
    float device_x,
    float device_y,
    bool anti_alias_is_lcd) {
  int32_t x = 0;
  int32_t y = 0;
  if (!pdfium_rust_plan_glyph_device_origin(device_x, device_y,
                                            anti_alias_is_lcd, &x, &y)) {
    return std::nullopt;
  }
  return GlyphOriginPlan{.valid = true, .x = x, .y = y};
}

std::optional<GlyphBoundsPlan> RustPlanGlyphBounds(
    size_t glyph_count,
    bool anti_alias_is_lcd,
    void* context,
    ReadGlyphBoundsCallback read_bounds) {
  GlyphBoundsPlan plan = {};
  if (!pdfium_rust_plan_glyph_bounds(glyph_count, anti_alias_is_lcd, context,
                                     read_bounds, &plan.left, &plan.top,
                                     &plan.right, &plan.bottom)) {
    return std::nullopt;
  }
  return plan;
}

std::optional<GlyphBitmapLookupAction> RustPlanGlyphBitmapLookup(
    bool glyph_is_valid,
    bool native_text,
    bool native_cache_hit) {
  uint8_t action = 0;
  if (!pdfium_rust_plan_glyph_bitmap_lookup(glyph_is_valid, native_text,
                                            native_cache_hit, &action) ||
      action > static_cast<uint8_t>(
                   GlyphBitmapLookupAction::kLookupNonNativeAndDisableNative)) {
    return std::nullopt;
  }
  return static_cast<GlyphBitmapLookupAction>(action);
}

std::optional<GlyphPathCacheKeyPlan> RustPlanGlyphPathCacheKey(
    uint32_t glyph_index,
    int32_t destination_width,
    bool has_substitution,
    int32_t weight,
    int32_t italic_angle,
    bool font_is_vertical) {
  GlyphPathCacheKeyPlan plan = {};
  uint8_t vertical = 0;
  if (!pdfium_rust_plan_glyph_path_cache_key(
          glyph_index, destination_width, has_substitution, weight,
          italic_angle, font_is_vertical, &plan.glyph_index,
          &plan.destination_width, &plan.weight, &plan.italic_angle,
          &vertical) ||
      vertical > 1) {
    return std::nullopt;
  }
  plan.vertical = !!vertical;
  return plan;
}

std::optional<GlyphWidthCacheKeyPlan> RustPlanGlyphWidthCacheKey(
    uint32_t glyph_index,
    int32_t destination_width,
    int32_t weight) {
  GlyphWidthCacheKeyPlan plan = {};
  if (!pdfium_rust_plan_glyph_width_cache_key(
          glyph_index, destination_width, weight, &plan.glyph_index,
          &plan.destination_width, &plan.weight)) {
    return std::nullopt;
  }
  return plan;
}

std::optional<FreeTypeGlyphLoadPlan> RustPlanFreeTypeGlyphLoad(
    bool is_render,
    bool is_tt_ot,
    bool is_tricky) {
  uint8_t no_hinting = 0;
  uint8_t pedantic = 0;
  uint8_t retry_without_hinting = 0;
  if (!pdfium_rust_plan_freetype_glyph_load(
          is_render, is_tt_ot, is_tricky, &no_hinting, &pedantic,
          &retry_without_hinting) ||
      no_hinting > 1 || pedantic > 1 || retry_without_hinting > 1) {
    return std::nullopt;
  }
  return FreeTypeGlyphLoadPlan{.no_hinting = !!no_hinting,
                               .pedantic = !!pedantic,
                               .retry_without_hinting =
                                   !!retry_without_hinting};
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
    AppendTraceWord(word);
  }
}

void RecordGlyphOriginForTesting(bool valid, int32_t x, int32_t y) {
  if (!g_glyph_trace_for_testing) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x4f);
  g_glyph_trace_for_testing->push_back(valid);
  for (int32_t coordinate : {x, y}) {
    AppendTraceWord(static_cast<uint32_t>(coordinate));
  }
}

void RecordGlyphDeviceOriginForTesting(float device_x,
                                       float device_y,
                                       bool anti_alias_is_lcd,
                                       int32_t x,
                                       int32_t y) {
  if (!g_glyph_trace_for_testing) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x44);
  g_glyph_trace_for_testing->push_back(anti_alias_is_lcd);
  AppendTraceWord(std::bit_cast<uint32_t>(device_x));
  AppendTraceWord(std::bit_cast<uint32_t>(device_y));
  AppendTraceWord(static_cast<uint32_t>(x));
  AppendTraceWord(static_cast<uint32_t>(y));
}

void RecordGlyphBoundsForTesting(int32_t left,
                                 int32_t top,
                                 int32_t right,
                                 int32_t bottom) {
  if (!g_glyph_trace_for_testing) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x42);
  for (int32_t coordinate : {left, top, right, bottom}) {
    AppendTraceWord(static_cast<uint32_t>(coordinate));
  }
}

void RecordGlyphBitmapLookupForTesting(bool glyph_is_valid,
                                       bool native_text,
                                       bool native_cache_hit,
                                       GlyphBitmapLookupAction action) {
  if (!g_glyph_trace_for_testing) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x4c);
  g_glyph_trace_for_testing->push_back(glyph_is_valid);
  g_glyph_trace_for_testing->push_back(native_text);
  g_glyph_trace_for_testing->push_back(native_cache_hit);
  g_glyph_trace_for_testing->push_back(static_cast<uint8_t>(action));
}

void RecordFreeTypeGlyphLoadPlanForTesting(
    bool is_render,
    const FreeTypeGlyphLoadPlan& plan) {
  if (!g_glyph_trace_for_testing) {
    return;
  }
  g_glyph_trace_for_testing->push_back(0x46);
  g_glyph_trace_for_testing->push_back(is_render);
  g_glyph_trace_for_testing->push_back(plan.no_hinting);
  g_glyph_trace_for_testing->push_back(plan.pedantic);
  g_glyph_trace_for_testing->push_back(plan.retry_without_hinting);
}

bool GlyphTraceHasOriginPlansForTesting(pdfium::span<const uint8_t> trace) {
  return GlyphTraceHasEvent(trace, 0x4f);
}

bool GlyphTraceHasDeviceOriginPlansForTesting(
    pdfium::span<const uint8_t> trace) {
  return GlyphTraceHasEvent(trace, 0x44);
}

bool GlyphTraceHasBoundsPlansForTesting(pdfium::span<const uint8_t> trace) {
  return GlyphTraceHasEvent(trace, 0x42);
}

bool GlyphTraceHasBitmapLookupPlansForTesting(
    pdfium::span<const uint8_t> trace) {
  return GlyphTraceHasEvent(trace, 0x4c);
}

bool GlyphTraceHasFreeTypeLoadPlansForTesting(
    pdfium::span<const uint8_t> trace) {
  return GlyphTraceHasEvent(trace, 0x46);
}

}  // namespace fxge
