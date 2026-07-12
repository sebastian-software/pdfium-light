// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
#define CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_

#include <stddef.h>
#include <stdint.h>

#include <optional>
#include <utility>
#include <vector>

#include "core/fxcrt/span.h"
#include "core/fxcrt/widestring.h"

namespace pdfium::rust {

using RustTextIndexMapCallback = bool (*)(void* context,
                                          int32_t input,
                                          int32_t* output);

class RustTextPageFind final {
 public:
  RustTextPageFind(WideStringView page_text,
                   pdfium::span<const WideString> words,
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

}  // namespace pdfium::rust

#endif  // CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
