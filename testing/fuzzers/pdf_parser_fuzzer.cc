// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stdint.h>

#include "core/fpdfapi/parser/cpdf_simple_parser.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/bytestring.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/span.h"
#include "public/fpdfview.h"

extern "C" int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {
  static constexpr size_t kMaxDataSize = 1024 * 1024;
  if (size > kMaxDataSize) {
    return 0;
  }

  // SAFETY: The fuzzer contract provides `size` readable bytes at `data`.
  const auto input = UNSAFE_BUFFERS(pdfium::span(data, size));
  CPDF_SimpleParser reference(input);
  CPDF_SimpleParser candidate(input);
  while (true) {
    ByteStringView reference_word;
    {
      pdfium::rust::ScopedRustParserImplementationForTesting implementation(
          false);
      reference_word = reference.GetWord();
    }
    ByteStringView candidate_word;
    {
      pdfium::rust::ScopedRustParserImplementationForTesting implementation(
          true);
      candidate_word = candidate.GetWord();
    }
    CHECK_EQ(reference_word, candidate_word);
    CHECK_EQ(reference.GetCurrentPosition(), candidate.GetCurrentPosition());
    if (reference_word.IsEmpty()) {
      break;
    }
  }

  FPDF_DOCUMENT document = FPDF_LoadMemDocument64(data, size, nullptr);
  if (document) {
    FPDF_CloseDocument(document);
  }
  return 0;
}
