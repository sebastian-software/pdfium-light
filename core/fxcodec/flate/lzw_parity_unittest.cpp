// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxcodec/flate/flatemodule.h"

#include <stdint.h>

#include <array>
#include <vector>

#include "core/fxcodec/rust/rust_codec_adapter.h"
#include "core/fxcrt/data_vector.h"
#include "core/fxcrt/fx_extension.h"
#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"

using testing::ElementsAreArray;

namespace fxcodec {
namespace {

constexpr uint32_t kClearCode = 256;
constexpr uint32_t kEndOfDataCode = 257;

void AppendCode(std::vector<uint8_t>* output,
                uint32_t code,
                uint8_t code_length,
                size_t* bit_pos) {
  for (uint8_t bit = 0; bit < code_length; ++bit) {
    if (*bit_pos % 8 == 0) {
      output->push_back(0);
    }
    const uint8_t shift = code_length - bit - 1;
    if ((code & (1u << shift)) != 0) {
      output->back() |= 1u << (7 - *bit_pos % 8);
    }
    ++*bit_pos;
  }
}

std::vector<uint8_t> PackLiteralCodes(pdfium::span<const uint8_t> literals,
                                      bool early_change,
                                      bool append_end_of_data) {
  std::vector<uint8_t> output;
  size_t bit_pos = 0;
  uint8_t code_length = 9;
  uint32_t current_code = 0;
  bool has_old_code = false;
  AppendCode(&output, kClearCode, code_length, &bit_pos);
  for (uint8_t literal : literals) {
    AppendCode(&output, literal, code_length, &bit_pos);
    if (has_old_code && current_code + (early_change ? 1u : 0u) != 4094) {
      ++current_code;
      if (current_code + (early_change ? 1u : 0u) == 254) {
        code_length = 10;
      } else if (current_code + (early_change ? 1u : 0u) == 766) {
        code_length = 11;
      } else if (current_code + (early_change ? 1u : 0u) == 1790) {
        code_length = 12;
      }
    }
    has_old_code = true;
  }
  if (append_end_of_data) {
    AppendCode(&output, kEndOfDataCode, code_length, &bit_pos);
  }
  return output;
}

TEST(LZWReferenceTest, DecodesClearAndEndOfDataCodes) {
  const std::array<uint8_t, 3> kLiterals = {'A', 'B', 'C'};
  const std::vector<uint8_t> input = PackLiteralCodes(kLiterals, true, true);

  DataAndBytesConsumed result =
      FlateModule::LZWDecodeReference(input, /*early_change=*/true);
  EXPECT_THAT(result.data, ElementsAreArray(kLiterals));
  EXPECT_EQ(input.size(), result.bytes_consumed);
}

TEST(LZWReferenceTest, PreservesEarlyChangeCodeWidthTransitions) {
  std::vector<uint8_t> literals(300);
  for (size_t i = 0; i < literals.size(); ++i) {
    literals[i] = static_cast<uint8_t>(i % 255);
  }

  for (bool early_change : {false, true}) {
    const std::vector<uint8_t> input =
        PackLiteralCodes(literals, early_change, true);
    DataAndBytesConsumed result =
        FlateModule::LZWDecodeReference(input, early_change);
    EXPECT_THAT(result.data, ElementsAreArray(literals));
    EXPECT_EQ(input.size(), result.bytes_consumed);
  }
}

TEST(LZWReferenceTest, PreservesTruncatedAndMalformedBehavior) {
  const std::array<uint8_t, 1> kLiteral = {'A'};
  const std::vector<uint8_t> truncated =
      PackLiteralCodes(kLiteral, true, /*append_end_of_data=*/false);
  DataAndBytesConsumed truncated_result =
      FlateModule::LZWDecodeReference(truncated, /*early_change=*/true);
  EXPECT_THAT(truncated_result.data, ElementsAreArray(kLiteral));
  EXPECT_EQ(truncated.size(), truncated_result.bytes_consumed);

  const std::vector<uint8_t> malformed = {0x81, 0x00};
  DataAndBytesConsumed malformed_result =
      FlateModule::LZWDecodeReference(malformed, /*early_change=*/true);
  EXPECT_THAT(malformed_result.data, ElementsAreArray(DataVector<uint8_t>()));
  EXPECT_EQ(FX_INVALID_OFFSET, malformed_result.bytes_consumed);
}

TEST(LZWRustParityTest, MatchesReferenceForNormalAndMalformedInput) {
  std::vector<uint8_t> literals(300);
  for (size_t i = 0; i < literals.size(); ++i) {
    literals[i] = static_cast<uint8_t>(i % 255);
  }

  for (bool early_change : {false, true}) {
    const std::vector<uint8_t> input =
        PackLiteralCodes(literals, early_change, true);
    DataAndBytesConsumed reference =
        FlateModule::LZWDecodeReference(input, early_change);
    DataAndBytesConsumed candidate =
        RustCodecAdapter::LZWDecode(input, early_change);
    EXPECT_EQ(reference.bytes_consumed, candidate.bytes_consumed);
    EXPECT_EQ(reference.data, candidate.data);
  }

  const std::vector<uint8_t> malformed = {0x81, 0x00};
  DataAndBytesConsumed reference =
      FlateModule::LZWDecodeReference(malformed, /*early_change=*/true);
  DataAndBytesConsumed candidate =
      RustCodecAdapter::LZWDecode(malformed, /*early_change=*/true);
  EXPECT_EQ(reference.bytes_consumed, candidate.bytes_consumed);
  EXPECT_EQ(reference.data, candidate.data);
}

}  // namespace
}  // namespace fxcodec
