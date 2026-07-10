// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxcodec/flate/flatemodule.h"
#include "core/fxcodec/rust/rust_codec_adapter.h"

#include <stdint.h>

#include <array>

#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"

using testing::ElementsAreArray;

namespace fxcodec {
namespace {

TEST(PNGPredictorReferenceTest, PreservesAllPredictorTagsAcrossRows) {
  const std::array<uint8_t, 16> kInput = {
      1, 1, 1, 1,
      2, 4, 4, 4,
      3, 2, 2, 2,
      4, 1, 1, 1,
  };
  const std::array<uint8_t, 12> kExpected = {
      1, 2, 3,
      5, 6, 7,
      4, 7, 9,
      5, 8, 10,
  };

  std::optional<DataVector<uint8_t>> result =
      FlateModule::PNGPredictorReference(1, 8, 3, kInput);
  ASSERT_TRUE(result.has_value());
  EXPECT_THAT(result.value(), ElementsAreArray(kExpected));
}

TEST(PNGPredictorReferenceTest, PreservesPartialAndInvalidInputBehavior) {
  const std::array<uint8_t, 3> kPartialInput = {0, 1, 2};
  const std::array<uint8_t, 2> kExpected = {1, 2};
  std::optional<DataVector<uint8_t>> partial_result =
      FlateModule::PNGPredictorReference(1, 8, 3, kPartialInput);
  ASSERT_TRUE(partial_result.has_value());
  EXPECT_THAT(partial_result.value(), ElementsAreArray(kExpected));

  EXPECT_FALSE(FlateModule::PNGPredictorReference(0, 8, 3, kPartialInput)
                   .has_value());
}

TEST(TIFFPredictorReferenceTest, PreservesOneEightAndSixteenBitBehavior) {
  DataVector<uint8_t> one_bit = {0x40};
  ASSERT_TRUE(FlateModule::TIFFPredictorReference(1, 1, 4, &one_bit));
  EXPECT_THAT(one_bit, ElementsAreArray(std::array<uint8_t, 1>{0x70}));

  DataVector<uint8_t> eight_bit = {1, 1, 1, 1};
  ASSERT_TRUE(FlateModule::TIFFPredictorReference(1, 8, 4, &eight_bit));
  EXPECT_THAT(eight_bit,
              ElementsAreArray(std::array<uint8_t, 4>{1, 2, 3, 4}));

  DataVector<uint8_t> sixteen_bit = {0, 1, 0, 1};
  ASSERT_TRUE(FlateModule::TIFFPredictorReference(1, 16, 2, &sixteen_bit));
  EXPECT_THAT(sixteen_bit,
              ElementsAreArray(std::array<uint8_t, 4>{0, 1, 0, 2}));
}

TEST(TIFFPredictorReferenceTest, RejectsInvalidGeometry) {
  DataVector<uint8_t> input = {1, 2, 3};
  EXPECT_FALSE(FlateModule::TIFFPredictorReference(0, 8, 3, &input));
  EXPECT_THAT(input, ElementsAreArray(std::array<uint8_t, 3>{1, 2, 3}));
}

TEST(PredictorRustParityTest, MatchesReferenceResultsAndFailureStatus) {
  const std::array<uint8_t, 16> png_input = {
      1, 1, 1, 1,
      2, 4, 4, 4,
      3, 2, 2, 2,
      4, 1, 1, 1,
  };
  std::optional<DataVector<uint8_t>> png_reference =
      FlateModule::PNGPredictorReference(1, 8, 3, png_input);
  DataAndBytesConsumed png_rust =
      RustCodecAdapter::PNGPredictor(png_input, 1, 8, 3);
  ASSERT_TRUE(png_reference.has_value());
  EXPECT_NE(FX_INVALID_OFFSET, png_rust.bytes_consumed);
  EXPECT_EQ(png_reference.value(), png_rust.data);

  const std::array<uint8_t, 3> invalid_png = {0, 1, 2};
  EXPECT_FALSE(
      FlateModule::PNGPredictorReference(0, 8, 3, invalid_png).has_value());
  EXPECT_EQ(FX_INVALID_OFFSET,
            RustCodecAdapter::PNGPredictor(invalid_png, 0, 8, 3)
                .bytes_consumed);

  DataVector<uint8_t> tiff_reference = {0, 1, 0, 1};
  ASSERT_TRUE(FlateModule::TIFFPredictorReference(1, 16, 2, &tiff_reference));
  const std::array<uint8_t, 4> tiff_input = {0, 1, 0, 1};
  DataAndBytesConsumed tiff_rust =
      RustCodecAdapter::TIFFPredictor(tiff_input, 1, 16, 2);
  EXPECT_NE(FX_INVALID_OFFSET, tiff_rust.bytes_consumed);
  EXPECT_EQ(tiff_reference, tiff_rust.data);

  const std::array<uint8_t, 3> invalid_tiff = {1, 2, 3};
  EXPECT_FALSE(
      FlateModule::TIFFPredictorReference(0, 8, 3, &tiff_reference));
  EXPECT_EQ(FX_INVALID_OFFSET,
            RustCodecAdapter::TIFFPredictor(invalid_tiff, 0, 8, 3)
                .bytes_consumed);
}

}  // namespace
}  // namespace fxcodec
