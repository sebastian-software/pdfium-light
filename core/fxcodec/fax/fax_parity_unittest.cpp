// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxcodec/fax/faxmodule.h"
#include "core/fxcodec/rust/rust_codec_adapter.h"

#include <stdint.h>

#include <array>
#include <memory>
#include <string_view>

#include "core/fxcodec/scanlinedecoder.h"
#include "core/fxcrt/data_vector.h"
#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"

using testing::ElementsAreArray;

namespace fxcodec {
namespace {

DataVector<uint8_t> PackBits(std::string_view bits) {
  DataVector<uint8_t> data((bits.size() + 7) / 8);
  for (size_t index = 0; index < bits.size(); ++index) {
    if (bits[index] == '1') {
      data[index / 8] |= 1 << (7 - index % 8);
    }
  }
  return data;
}

std::unique_ptr<ScanlineDecoder> CreateFaxReferenceDecoder(
    pdfium::span<const uint8_t> data,
    int height,
    int encoding,
    bool end_of_line,
    bool byte_align,
    bool black_is_1) {
  return FaxModule::CreateDecoderReference(data, /*width=*/8, height,
                                           encoding, end_of_line, byte_align,
                                           black_is_1, /*columns=*/8,
                                           /*rows=*/height);
}

std::unique_ptr<ScanlineDecoder> CreateRustFaxDecoder(
    pdfium::span<const uint8_t> data,
    int height,
    int encoding,
    bool end_of_line,
    bool byte_align,
    bool black_is_1) {
  return FaxModule::CreateDecoder(data, /*width=*/8, height, encoding,
                                  end_of_line, byte_align, black_is_1,
                                  /*columns=*/8, /*rows=*/height);
}

TEST(FaxG4ReferenceTest, PreservesVerticalHorizontalAndTruncatedRows) {
  const std::array<uint8_t, 8> kTwoWhiteRows = {
      0xff, 0xff, 0xff, 0xff,
      0xff, 0xff, 0xff, 0xff,
  };
  DataVector<uint8_t> vertical_rows = PackBits("11");
  std::array<uint8_t, 8> vertical_output = {};
  EXPECT_EQ(2u, FaxModule::FaxG4Decode(vertical_rows, /*starting_bitpos=*/0,
                                       /*width=*/8, /*height=*/2,
                                       /*pitch=*/4, vertical_output));
  EXPECT_THAT(vertical_output, ElementsAreArray(kTwoWhiteRows));

  const std::array<uint8_t, 4> kBlackRow = {0, 0xff, 0xff, 0xff};
  DataVector<uint8_t> horizontal_row = PackBits("00100110101000101");
  std::array<uint8_t, 4> horizontal_output = {};
  EXPECT_EQ(17u, FaxModule::FaxG4Decode(horizontal_row, /*starting_bitpos=*/0,
                                        /*width=*/8, /*height=*/1,
                                        /*pitch=*/4, horizontal_output));
  EXPECT_THAT(horizontal_output, ElementsAreArray(kBlackRow));

  DataVector<uint8_t> truncated_row = PackBits("0");
  std::array<uint8_t, 4> truncated_output = {};
  EXPECT_EQ(12u, FaxModule::FaxG4Decode(truncated_row, /*starting_bitpos=*/0,
                                        /*width=*/8, /*height=*/1,
                                        /*pitch=*/4, truncated_output));
  EXPECT_THAT(truncated_output,
              ElementsAreArray(std::array<uint8_t, 4>{0xff, 0xff, 0xff,
                                                       0xff}));
}

TEST(FaxG4ReferenceTest, PreservesMakeupRunsAndPassMode) {
  // The first row is a horizontal all-black 128-pixel run: black 128 is a
  // makeup code followed by its zero-length terminator. The second row uses
  // pass mode against that non-white reference row.
  DataVector<uint8_t> input = PackBits(
      "001"      // Horizontal mode.
      "00110101" // White run length 0.
      "000011001000" // Black makeup run length 128.
      "0000110111" // Black terminator run length 0.
      "0001");  // Pass mode.
  std::array<uint8_t, 32> output = {};
  EXPECT_EQ(37u, FaxModule::FaxG4Decode(input, /*starting_bitpos=*/0,
                                        /*width=*/128, /*height=*/2,
                                        /*pitch=*/16, output));
  EXPECT_THAT(pdfium::span(output).first(16),
              ElementsAreArray(std::array<uint8_t, 16>{}));
  EXPECT_THAT(pdfium::span(output).last(16),
              ElementsAreArray(std::array<uint8_t, 16>{
                  0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                  0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
              }));
}

TEST(FaxG4RustParityTest, MatchesReferenceRowsAndEndingBitPositions) {
  DataVector<uint8_t> horizontal_row = PackBits("00100110101000101");
  std::array<uint8_t, 4> reference_output = {};
  uint32_t reference_bitpos = FaxModule::FaxG4DecodeReference(
      horizontal_row, /*starting_bitpos=*/0, /*width=*/8, /*height=*/1,
      /*pitch=*/4, reference_output);
  DataAndBytesConsumed rust_output = RustCodecAdapter::FaxG4Decode(
      horizontal_row, /*starting_bitpos=*/0, /*width=*/8, /*height=*/1,
      /*pitch=*/4);
  EXPECT_EQ(reference_bitpos, rust_output.bytes_consumed);
  EXPECT_THAT(rust_output.data, ElementsAreArray(reference_output));

  DataVector<uint8_t> truncated_row = PackBits("0");
  reference_bitpos = FaxModule::FaxG4DecodeReference(
      truncated_row, /*starting_bitpos=*/0, /*width=*/8, /*height=*/1,
      /*pitch=*/4, reference_output);
  rust_output = RustCodecAdapter::FaxG4Decode(
      truncated_row, /*starting_bitpos=*/0, /*width=*/8, /*height=*/1,
      /*pitch=*/4);
  EXPECT_EQ(reference_bitpos, rust_output.bytes_consumed);
  EXPECT_THAT(rust_output.data, ElementsAreArray(reference_output));

  DataVector<uint8_t> makeup_and_pass = PackBits(
      "00100110101000011001000000011011100001");
  std::array<uint8_t, 32> reference_makeup_output = {};
  reference_bitpos = FaxModule::FaxG4DecodeReference(
      makeup_and_pass, /*starting_bitpos=*/0, /*width=*/128, /*height=*/2,
      /*pitch=*/16, reference_makeup_output);
  rust_output = RustCodecAdapter::FaxG4Decode(
      makeup_and_pass, /*starting_bitpos=*/0, /*width=*/128, /*height=*/2,
      /*pitch=*/16);
  EXPECT_EQ(reference_bitpos, rust_output.bytes_consumed);
  EXPECT_THAT(rust_output.data, ElementsAreArray(reference_makeup_output));
}

TEST(FaxScanlineReferenceTest, PreservesGroup3AndGroup4Modes) {
  const std::array<uint8_t, 4> kWhiteRow = {0xff, 0xff, 0xff, 0xff};
  const std::array<uint8_t, 4> kBlackRow = {0, 0xff, 0xff, 0xff};

  DataVector<uint8_t> one_dimensional = PackBits("00110101000101");
  std::unique_ptr<ScanlineDecoder> one_dimensional_decoder =
      CreateFaxReferenceDecoder(one_dimensional, /*height=*/1,
                                /*encoding=*/0, /*end_of_line=*/false,
                                /*byte_align=*/false, /*black_is_1=*/false);
  ASSERT_TRUE(one_dimensional_decoder);
  EXPECT_THAT(one_dimensional_decoder->GetScanline(0),
              ElementsAreArray(kBlackRow));

  DataVector<uint8_t> mixed_one_dimensional = PackBits("110011");
  std::unique_ptr<ScanlineDecoder> mixed_one_dimensional_decoder =
      CreateFaxReferenceDecoder(mixed_one_dimensional, /*height=*/1,
                                /*encoding=*/1, /*end_of_line=*/false,
                                /*byte_align=*/false, /*black_is_1=*/false);
  ASSERT_TRUE(mixed_one_dimensional_decoder);
  EXPECT_THAT(mixed_one_dimensional_decoder->GetScanline(0),
              ElementsAreArray(kWhiteRow));

  DataVector<uint8_t> mixed_two_dimensional = PackBits("01");
  std::unique_ptr<ScanlineDecoder> mixed_two_dimensional_decoder =
      CreateFaxReferenceDecoder(mixed_two_dimensional, /*height=*/1,
                                /*encoding=*/1, /*end_of_line=*/false,
                                /*byte_align=*/false, /*black_is_1=*/false);
  ASSERT_TRUE(mixed_two_dimensional_decoder);
  EXPECT_THAT(mixed_two_dimensional_decoder->GetScanline(0),
              ElementsAreArray(kWhiteRow));
}

TEST(FaxScanlineReferenceTest, PreservesEolAlignmentAndBlackIs1Behavior) {
  const std::array<uint8_t, 4> kWhiteRow = {0xff, 0xff, 0xff, 0xff};
  DataVector<uint8_t> eol_row = PackBits("00000000000110011");
  std::unique_ptr<ScanlineDecoder> eol_decoder = CreateFaxReferenceDecoder(
      eol_row, /*height=*/1, /*encoding=*/0, /*end_of_line=*/true,
      /*byte_align=*/false, /*black_is_1=*/false);
  ASSERT_TRUE(eol_decoder);
  EXPECT_THAT(eol_decoder->GetScanline(0), ElementsAreArray(kWhiteRow));

  DataVector<uint8_t> aligned_rows = PackBits("1001100010011");
  std::unique_ptr<ScanlineDecoder> aligned_decoder = CreateFaxReferenceDecoder(
      aligned_rows, /*height=*/2, /*encoding=*/0, /*end_of_line=*/false,
      /*byte_align=*/true, /*black_is_1=*/false);
  ASSERT_TRUE(aligned_decoder);
  EXPECT_THAT(aligned_decoder->GetScanline(0), ElementsAreArray(kWhiteRow));
  EXPECT_THAT(aligned_decoder->GetScanline(1), ElementsAreArray(kWhiteRow));
  EXPECT_EQ(2u, aligned_decoder->GetSrcOffset());

  DataVector<uint8_t> inverted_row = PackBits("1");
  std::unique_ptr<ScanlineDecoder> inverted_decoder = CreateFaxReferenceDecoder(
      inverted_row, /*height=*/1, /*encoding=*/-1, /*end_of_line=*/false,
      /*byte_align=*/false, /*black_is_1=*/true);
  ASSERT_TRUE(inverted_decoder);
  EXPECT_THAT(inverted_decoder->GetScanline(0),
              ElementsAreArray(std::array<uint8_t, 4>{0, 0, 0, 0}));
}

TEST(FaxScanlineRustParityTest, MatchesReferenceLinesOffsetsAndRewind) {
  const DataVector<uint8_t> kInputs[] = {
      PackBits("00110101000101"), PackBits("110011"),
      PackBits("01"), PackBits("1001100010011"),
      PackBits("00000000000110011"), PackBits("1"),
  };
  const int kEncodings[] = {0, 1, 1, 0, 0, -1};
  const int kHeights[] = {1, 1, 1, 2, 1, 1};
  const bool kEndOfLine[] = {false, false, false, false, true, false};
  const bool kByteAlign[] = {false, false, false, true, false, false};
  const bool kBlackIs1[] = {false, false, false, false, false, true};
  for (size_t index = 0; index < std::size(kInputs); ++index) {
    std::unique_ptr<ScanlineDecoder> reference = CreateFaxReferenceDecoder(
        kInputs[index], kHeights[index], kEncodings[index],
        kEndOfLine[index], kByteAlign[index], kBlackIs1[index]);
    std::unique_ptr<ScanlineDecoder> candidate = CreateRustFaxDecoder(
        kInputs[index], kHeights[index], kEncodings[index],
        kEndOfLine[index], kByteAlign[index], kBlackIs1[index]);
    ASSERT_TRUE(reference);
    ASSERT_TRUE(candidate);
    for (int line = 0; line < kHeights[index]; ++line) {
      EXPECT_THAT(candidate->GetScanline(line),
                  ElementsAreArray(reference->GetScanline(line)));
      EXPECT_EQ(reference->GetSrcOffset(), candidate->GetSrcOffset());
    }
    EXPECT_THAT(candidate->GetScanline(0),
                ElementsAreArray(reference->GetScanline(0)));
    EXPECT_EQ(reference->GetSrcOffset(), candidate->GetSrcOffset());
  }
}

}  // namespace
}  // namespace fxcodec
