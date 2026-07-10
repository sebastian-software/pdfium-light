// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stdint.h>
#include <stddef.h>

#include <array>
#include <vector>

#include "core/fpdfapi/parser/fpdf_parser_decode.h"
#include "core/fxcodec/basic/basicmodule.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace fxcodec {
namespace {

void ExpectDecodeParity(DataAndBytesConsumed reference,
                        DataAndBytesConsumed candidate) {
  EXPECT_EQ(reference.bytes_consumed, candidate.bytes_consumed);
  EXPECT_EQ(reference.data, candidate.data);
}

std::vector<uint8_t> MakeIncrementingInput(size_t length) {
  std::vector<uint8_t> input(length);
  for (size_t i = 0; i < input.size(); ++i) {
    input[i] = static_cast<uint8_t>(i);
  }
  return input;
}

std::vector<uint8_t> MakeAlternatingInput(size_t length) {
  std::vector<uint8_t> input(length);
  for (size_t i = 0; i < input.size(); ++i) {
    input[i] = i % 2 == 0 ? 1 : 2;
  }
  return input;
}

TEST(RustCodecParityTest, EncodersMatchCppReference) {
  const std::array<std::vector<uint8_t>, 9> kInputs = {
      std::vector<uint8_t>{},
      std::vector<uint8_t>{0},
      std::vector<uint8_t>{1},
      std::vector<uint8_t>{1, 2, 3, 4},
      std::vector<uint8_t>{0, 0, 0, 0, 1, 2, 3, 4},
      std::vector<uint8_t>{1, 1, 1, 1, 2, 3, 4, 4, 4},
      std::vector<uint8_t>(260, 42),
      MakeIncrementingInput(300),
      MakeAlternatingInput(300),
  };

  for (const auto& input : kInputs) {
    EXPECT_EQ(BasicModule::A85EncodeReference(input),
              BasicModule::A85Encode(input));
    EXPECT_EQ(BasicModule::RunLengthEncodeReference(input),
              BasicModule::RunLengthEncode(input));
  }
}

TEST(RustCodecParityTest, DecodersMatchCppReferenceForNormalAndMalformedInput) {
  const std::array<std::vector<uint8_t>, 12> kA85Inputs = {
      std::vector<uint8_t>{},
      std::vector<uint8_t>{'~', '>'},
      std::vector<uint8_t>{'F', 'C', 'f', 'N', '8', '~', '>'},
      std::vector<uint8_t>{'F', 'C', 'f', 'N', '8', 'v', 'w'},
      std::vector<uint8_t>{'z', '~', '>'},
      std::vector<uint8_t>{'z', '!', '!', '!', '!', '!', '~', '>'},
      std::vector<uint8_t>{'1', '2', 'A'},
      std::vector<uint8_t>{'\t', ' ', 'F', 'C', '\r', '\n', 'f', 'N',
                           '8', ' ', '~', '>'},
      std::vector<uint8_t>{'~'},
      std::vector<uint8_t>{'>'},
      std::vector<uint8_t>{0xff},
      std::vector<uint8_t>{'F', 'C', 'f', 'N', '8', '~', '>', 'F'},
  };
  for (const auto& input : kA85Inputs) {
    ExpectDecodeParity(A85DecodeReference(input), A85Decode(input));
  }

  const std::array<std::vector<uint8_t>, 10> kHexInputs = {
      std::vector<uint8_t>{},
      std::vector<uint8_t>{'>'},
      std::vector<uint8_t>{'4', '8', '6', '9', '>'},
      std::vector<uint8_t>{'4', 'f', '6'},
      std::vector<uint8_t>{'4', ' ', 'g', 'F', '\n', '6', '@'},
      std::vector<uint8_t>{'\t', '4', '8', '\r', '6', '9', ' ', '>'},
      std::vector<uint8_t>{'4', '8', '>', '4', '8'},
      std::vector<uint8_t>{'x', 'y', 'z'},
      std::vector<uint8_t>{0xff, '4', '8'},
      std::vector<uint8_t>{'4', '8', '6', '9', '>', 0xff},
  };
  for (const auto& input : kHexInputs) {
    ExpectDecodeParity(HexDecodeReference(input), HexDecode(input));
  }

  const std::array<std::vector<uint8_t>, 8> kRunLengthInputs = {
      std::vector<uint8_t>{},
      std::vector<uint8_t>{128},
      std::vector<uint8_t>{0, 'a', 128},
      std::vector<uint8_t>{255, 'a', 128},
      std::vector<uint8_t>{2, 'a', 'b', 'c', 128},
      std::vector<uint8_t>{2, 'a'},
      std::vector<uint8_t>{255},
      std::vector<uint8_t>{127, 'a', 128},
  };
  for (const auto& input : kRunLengthInputs) {
    ExpectDecodeParity(RunLengthDecodeReference(input),
                       RunLengthDecode(input));
  }
}

}  // namespace
}  // namespace fxcodec
