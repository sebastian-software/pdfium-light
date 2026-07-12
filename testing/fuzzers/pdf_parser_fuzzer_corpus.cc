// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stdint.h>

#include "public/fpdfview.h"

extern "C" int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size);

int main() {
  FPDF_InitLibrary();
#define RUST_PARSER_CASE(name, input)                                        \
  do {                                                                       \
    static constexpr char k##name##Input[] = input;                          \
    LLVMFuzzerTestOneInput(reinterpret_cast<const uint8_t*>(k##name##Input), \
                           sizeof(k##name##Input) - 1);                      \
  } while (false);
#include "testing/resources/rust_parser_corpus.inc"
#undef RUST_PARSER_CASE
  FPDF_DestroyLibrary();
  return 0;
}
