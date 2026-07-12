// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdftext/rust/rust_text_adapter.h"

#include <vector>

#include "core/fxcrt/compiler_specific.h"

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
