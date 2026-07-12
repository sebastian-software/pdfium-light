// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
#define CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_

#include <stddef.h>
#include <stdint.h>

#include <optional>
#include <utility>

#include "core/fxcrt/span.h"
#include "core/fxcrt/widestring.h"

namespace pdfium::rust {

using RustTextIndexMapCallback = bool (*)(void* context,
                                          int32_t input,
                                          int32_t* output);

class RustTextPageFind final {
 public:
  RustTextPageFind(WideStringView page_text,
                   WideStringView query,
                   bool match_whole_word,
                   bool consecutive,
                   std::optional<size_t> start,
                   void* context,
                   RustTextIndexMapCallback text_to_character,
                   RustTextIndexMapCallback character_to_text);
  RustTextPageFind(const RustTextPageFind&) = delete;
  RustTextPageFind& operator=(const RustTextPageFind&) = delete;
  ~RustTextPageFind();

  bool valid() const { return state_ != nullptr; }
  bool FindFirst() const;
  std::optional<bool> FindNext();
  std::optional<bool> FindPrevious();
  std::optional<std::pair<size_t, size_t>> GetResult() const;

 private:
  void* state_;
  void* context_;
  RustTextIndexMapCallback text_to_character_;
  RustTextIndexMapCallback character_to_text_;
};

using RustTextIsAlphanumericCallback = bool (*)(void* context,
                                                uint32_t character);

struct RustTextLinkRange {
  size_t start;
  size_t count;
};

class RustTextLinkExtract final {
 public:
  RustTextLinkExtract();
  RustTextLinkExtract(const RustTextLinkExtract&) = delete;
  RustTextLinkExtract& operator=(const RustTextLinkExtract&) = delete;
  ~RustTextLinkExtract();

  bool Extract(WideStringView page_text,
               pdfium::span<const uint32_t> characters,
               pdfium::span<const uint8_t> flags,
               void* context,
               RustTextIsAlphanumericCallback is_alphanumeric);
  size_t size() const;
  std::optional<RustTextLinkRange> GetRange(size_t index) const;
  std::optional<WideString> GetUrl(size_t index) const;

 private:
  void* state_;
};

}  // namespace pdfium::rust

#endif  // CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
