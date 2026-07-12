// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdftext/rust/rust_text_adapter.h"

#include <vector>

#include "core/fxcrt/compiler_specific.h"

extern "C" bool pdfium_rust_text_index_at_position(
    size_t character_count,
    float point_x,
    float point_y,
    float tolerance_width,
    float tolerance_height,
    void* context,
    pdfium::rust::RustTextRectCallback get_rect,
    int32_t* output);
extern "C" void* pdfium_rust_text_index_map_new(const uint8_t* included,
                                                 size_t included_len);
extern "C" void pdfium_rust_text_index_map_free(void* state);
extern "C" int32_t pdfium_rust_text_index_map_character_from_text(
    const void* state,
    int32_t text_index);
extern "C" int32_t pdfium_rust_text_index_map_text_from_character(
    const void* state,
    int32_t character_index);
extern "C" bool pdfium_rust_text_index_map_page_text_range(
    const void* state,
    int32_t start,
    int32_t count,
    int32_t character_count,
    int32_t* text_start,
    int32_t* text_count);
extern "C" void* pdfium_rust_text_selection_rects_new(
    size_t character_count,
    int32_t start,
    int32_t count,
    void* context,
    pdfium::rust::RustTextSelectionRectCallback get_character);
extern "C" void pdfium_rust_text_selection_rects_free(void* state);
extern "C" size_t pdfium_rust_text_selection_rects_count(const void* state);
extern "C" bool pdfium_rust_text_selection_rects_get(const void* state,
                                                      size_t index,
                                                      float* left,
                                                      float* bottom,
                                                      float* right,
                                                      float* top);
extern "C" void* pdfium_rust_text_predicate_result_new(
    size_t character_count,
    void* context,
    pdfium::rust::RustTextPredicateCharacterCallback get_character);
extern "C" void pdfium_rust_text_predicate_result_free(void* state);
extern "C" size_t pdfium_rust_text_predicate_result_len(const void* state);
extern "C" bool pdfium_rust_text_predicate_result_copy(const void* state,
                                                        uint32_t* output,
                                                        size_t output_capacity);
extern "C" void* pdfium_rust_text_line_plan_new(const uint32_t* text,
                                                 size_t text_len);
extern "C" void pdfium_rust_text_line_plan_free(void* state);
extern "C" size_t pdfium_rust_text_line_plan_kept_count(const void* state);
extern "C" bool pdfium_rust_text_line_plan_kept_index(const void* state,
                                                       size_t index,
                                                       size_t* source_index);
extern "C" bool pdfium_rust_text_line_plan_set_segments(
    void* state,
    pdfium::rust::RustTextDirection overall_direction,
    const pdfium::rust::RustTextBidiSegment* segments,
    size_t segment_count);
extern "C" size_t pdfium_rust_text_line_plan_emission_count(const void* state);
extern "C" bool pdfium_rust_text_line_plan_emission(const void* state,
                                                     size_t index,
                                                     size_t* character_index,
                                                     bool* is_rtl);
extern "C" void* pdfium_rust_text_find_new(const uint32_t* page_text,
                                           size_t page_text_len,
                                           const uint32_t* query,
                                           size_t query_len,
                                           bool match_whole_word,
                                           bool consecutive,
                                           bool has_start,
                                           size_t start);
extern "C" void pdfium_rust_text_find_free(void* state);
extern "C" bool pdfium_rust_text_find_first(const void* state);
extern "C" bool pdfium_rust_text_find_next(void* state, bool* matched);
extern "C" bool pdfium_rust_text_find_previous(
    void* state,
    void* context,
    pdfium::rust::RustTextIndexMapCallback text_to_character,
    pdfium::rust::RustTextIndexMapCallback character_to_text,
    bool* matched);
extern "C" bool pdfium_rust_text_find_result(const void* state,
                                             size_t* start,
                                             size_t* end);
extern "C" void* pdfium_rust_text_link_extract_new();
extern "C" void pdfium_rust_text_link_extract_free(void* state);
extern "C" bool pdfium_rust_text_link_extract_run(
    void* state,
    const uint32_t* page_text,
    size_t page_text_len,
    const uint32_t* characters,
    const uint8_t* flags,
    size_t character_count,
    void* context,
    pdfium::rust::RustTextIsAlphanumericCallback is_alphanumeric);
extern "C" size_t pdfium_rust_text_link_extract_count(const void* state);
extern "C" bool pdfium_rust_text_link_extract_range(const void* state,
                                                    size_t index,
                                                    size_t* start,
                                                    size_t* count);
extern "C" bool pdfium_rust_text_link_extract_url(const void* state,
                                                  size_t index,
                                                  uint32_t* output,
                                                  size_t output_capacity,
                                                  size_t* url_len);

namespace pdfium::rust {

static_assert(offsetof(RustTextBidiSegment, direction) == 2 * sizeof(size_t));
static_assert(sizeof(RustTextBidiSegment) == 3 * sizeof(size_t));

namespace {

std::vector<uint32_t> ToCodePoints(WideStringView value) {
  std::vector<uint32_t> result;
  result.reserve(value.GetLength());
  for (wchar_t character : value) {
    result.push_back(static_cast<uint32_t>(character));
  }
  return result;
}

}  // namespace

std::optional<int> RustTextIndexAtPosition(
    size_t character_count,
    float point_x,
    float point_y,
    float tolerance_width,
    float tolerance_height,
    void* context,
    RustTextRectCallback get_rect) {
  int32_t output = -1;
  if (!pdfium_rust_text_index_at_position(
          character_count, point_x, point_y, tolerance_width,
          tolerance_height, context, get_rect, &output)) {
    return std::nullopt;
  }
  return output;
}

RustTextIndexMap::RustTextIndexMap(pdfium::span<const uint8_t> included)
    : state_(pdfium_rust_text_index_map_new(included.data(), included.size())) {}

RustTextIndexMap::~RustTextIndexMap() {
  pdfium_rust_text_index_map_free(state_);
}

int RustTextIndexMap::CharacterFromText(int text_index) const {
  return state_ ? pdfium_rust_text_index_map_character_from_text(state_,
                                                                 text_index)
                : -1;
}

int RustTextIndexMap::TextFromCharacter(int character_index) const {
  return state_ ? pdfium_rust_text_index_map_text_from_character(
                      state_, character_index)
                : -1;
}

std::optional<std::pair<int, int>> RustTextIndexMap::PageTextRange(
    int start,
    int count,
    int character_count) const {
  int32_t text_start = 0;
  int32_t text_count = 0;
  if (!state_ || !pdfium_rust_text_index_map_page_text_range(
                     state_, start, count, character_count, &text_start,
                     &text_count)) {
    return std::nullopt;
  }
  return std::make_pair(text_start, text_count);
}

RustTextSelectionRects::RustTextSelectionRects(
    size_t character_count,
    int start,
    int count,
    void* context,
    RustTextSelectionRectCallback get_character)
    : state_(pdfium_rust_text_selection_rects_new(
          character_count, start, count, context, get_character)) {}

RustTextSelectionRects::~RustTextSelectionRects() {
  pdfium_rust_text_selection_rects_free(state_);
}

size_t RustTextSelectionRects::size() const {
  return state_ ? pdfium_rust_text_selection_rects_count(state_) : 0;
}

std::optional<RustTextRect> RustTextSelectionRects::GetRect(
    size_t index) const {
  RustTextRect rect = {};
  if (!state_ || !pdfium_rust_text_selection_rects_get(
                     state_, index, &rect.left, &rect.bottom, &rect.right,
                     &rect.top)) {
    return std::nullopt;
  }
  return rect;
}

RustTextPredicateResult::RustTextPredicateResult(
    size_t character_count,
    void* context,
    RustTextPredicateCharacterCallback get_character)
    : state_(pdfium_rust_text_predicate_result_new(character_count, context,
                                                   get_character)) {}

RustTextPredicateResult::~RustTextPredicateResult() {
  pdfium_rust_text_predicate_result_free(state_);
}

std::optional<WideString> RustTextPredicateResult::GetText() const {
  if (!state_) {
    return std::nullopt;
  }
  std::vector<uint32_t> code_points(
      pdfium_rust_text_predicate_result_len(state_));
  if (!pdfium_rust_text_predicate_result_copy(
          state_, code_points.data(), code_points.size())) {
    return std::nullopt;
  }
  std::vector<wchar_t> characters;
  characters.reserve(code_points.size());
  for (uint32_t code_point : code_points) {
    characters.push_back(static_cast<wchar_t>(code_point));
  }
  return UNSAFE_BUFFERS(WideString(characters.data(), characters.size()));
}

RustTextLinePlan::RustTextLinePlan(WideStringView text) : state_(nullptr) {
  std::vector<uint32_t> code_points = ToCodePoints(text);
  state_ = pdfium_rust_text_line_plan_new(code_points.data(),
                                          code_points.size());
}

RustTextLinePlan::~RustTextLinePlan() {
  pdfium_rust_text_line_plan_free(state_);
}

size_t RustTextLinePlan::kept_count() const {
  return state_ ? pdfium_rust_text_line_plan_kept_count(state_) : 0;
}

std::optional<size_t> RustTextLinePlan::GetKeptIndex(size_t index) const {
  size_t source_index = 0;
  if (!state_ ||
      !pdfium_rust_text_line_plan_kept_index(state_, index, &source_index)) {
    return std::nullopt;
  }
  return source_index;
}

bool RustTextLinePlan::SetSegments(
    RustTextDirection overall_direction,
    pdfium::span<const RustTextBidiSegment> segments) {
  return state_ && pdfium_rust_text_line_plan_set_segments(
                       state_, overall_direction, segments.data(),
                       segments.size());
}

size_t RustTextLinePlan::emission_count() const {
  return state_ ? pdfium_rust_text_line_plan_emission_count(state_) : 0;
}

std::optional<RustTextEmission> RustTextLinePlan::GetEmission(
    size_t index) const {
  RustTextEmission emission = {};
  if (!state_ || !pdfium_rust_text_line_plan_emission(
                     state_, index, &emission.character_index,
                     &emission.is_rtl)) {
    return std::nullopt;
  }
  return emission;
}

RustTextPageFind::RustTextPageFind(WideStringView page_text,
                                   WideStringView query,
                                   bool match_whole_word,
                                   bool consecutive,
                                   std::optional<size_t> start,
                                   void* context,
                                   RustTextIndexMapCallback text_to_character,
                                   RustTextIndexMapCallback character_to_text)
    : state_(nullptr),
      context_(context),
      text_to_character_(text_to_character),
      character_to_text_(character_to_text) {
  std::vector<uint32_t> page_code_points = ToCodePoints(page_text);
  std::vector<uint32_t> query_code_points = ToCodePoints(query);
  state_ = pdfium_rust_text_find_new(
      page_code_points.data(), page_code_points.size(),
      query_code_points.data(), query_code_points.size(), match_whole_word,
      consecutive, start.has_value(), start.value_or(0));
}

RustTextPageFind::~RustTextPageFind() {
  pdfium_rust_text_find_free(state_);
}

bool RustTextPageFind::FindFirst() const {
  return valid() && pdfium_rust_text_find_first(state_);
}

std::optional<bool> RustTextPageFind::FindNext() {
  bool matched = false;
  if (!valid() || !pdfium_rust_text_find_next(state_, &matched)) {
    return std::nullopt;
  }
  return matched;
}

std::optional<bool> RustTextPageFind::FindPrevious() {
  bool matched = false;
  if (!valid() ||
      !pdfium_rust_text_find_previous(state_, context_, text_to_character_,
                                      character_to_text_, &matched)) {
    return std::nullopt;
  }
  return matched;
}

std::optional<std::pair<size_t, size_t>> RustTextPageFind::GetResult() const {
  size_t start = 0;
  size_t end = 0;
  if (!valid() || !pdfium_rust_text_find_result(state_, &start, &end)) {
    return std::nullopt;
  }
  return std::make_pair(start, end);
}

RustTextLinkExtract::RustTextLinkExtract()
    : state_(pdfium_rust_text_link_extract_new()) {}

RustTextLinkExtract::~RustTextLinkExtract() {
  pdfium_rust_text_link_extract_free(state_);
}

bool RustTextLinkExtract::Extract(
    WideStringView page_text,
    pdfium::span<const uint32_t> characters,
    pdfium::span<const uint8_t> flags,
    void* context,
    RustTextIsAlphanumericCallback is_alphanumeric) {
  if (characters.size() != flags.size()) {
    return false;
  }
  std::vector<uint32_t> page_code_points = ToCodePoints(page_text);
  return state_ && pdfium_rust_text_link_extract_run(
                       state_, page_code_points.data(), page_code_points.size(),
                       characters.data(), flags.data(), characters.size(),
                       context, is_alphanumeric);
}

size_t RustTextLinkExtract::size() const {
  return state_ ? pdfium_rust_text_link_extract_count(state_) : 0;
}

std::optional<RustTextLinkRange> RustTextLinkExtract::GetRange(
    size_t index) const {
  RustTextLinkRange result = {};
  if (!state_ || !pdfium_rust_text_link_extract_range(
                     state_, index, &result.start, &result.count)) {
    return std::nullopt;
  }
  return result;
}

std::optional<WideString> RustTextLinkExtract::GetUrl(size_t index) const {
  size_t length = 0;
  if (!state_ ||
      !pdfium_rust_text_link_extract_url(state_, index, nullptr, 0, &length)) {
    return std::nullopt;
  }
  std::vector<uint32_t> code_points(length);
  if (length != 0 &&
      !pdfium_rust_text_link_extract_url(state_, index, code_points.data(),
                                         code_points.size(), &length)) {
    return std::nullopt;
  }
  std::vector<wchar_t> characters;
  characters.reserve(code_points.size());
  for (uint32_t code_point : code_points) {
    characters.push_back(static_cast<wchar_t>(code_point));
  }
  return UNSAFE_BUFFERS(WideString(characters.data(), characters.size()));
}

}  // namespace pdfium::rust
