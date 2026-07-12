// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdftext/rust/rust_text_adapter.h"

#include <vector>

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

namespace pdfium::rust {

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

}  // namespace pdfium::rust
