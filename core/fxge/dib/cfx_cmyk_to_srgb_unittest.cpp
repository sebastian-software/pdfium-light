// Copyright 2019 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cfx_cmyk_to_srgb.h"

#include <array>

#include "core/fxcrt/data_vector.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

union Float_t {
  Float_t(float num = 0.0f) : f(num) {}

  int32_t i;
  float f;
};

TEST(fxge, CMYKRounding) {
  // Testing all floats from 0.0 to 1.0 takes about 35 seconds in release
  // builds and much longer in debug builds, so just test the known-dangerous
  // range.
  static constexpr float kStartValue = 0.001f;
  static constexpr float kEndValue = 0.003f;
  FX_RGB_STRUCT<float> rgb;
  // Iterate through floats by incrementing the representation, as discussed in
  // https://randomascii.wordpress.com/2012/01/23/stupid-float-tricks-2/
  for (Float_t f = kStartValue; f.f < kEndValue; f.i++) {
    rgb = AdobeCmykToStandardRgbF(f.f, f.f, f.f, f.f);
  }
  // Check various other 'special' numbers.
  rgb = AdobeCmykToStandardRgbF(0.0f, 0.25f, 0.5f, 1.0f);
}

TEST(fxge, RustCmykMatchesCppReferenceAcrossInterpolationBoundaries) {
  static constexpr std::array<uint8_t, 26> kChannels = {
      0,   1,   31,  32,  33,  63,  64,  65,  95,  96,  97,  127, 128,
      129, 159, 160, 161, 191, 192, 193, 223, 224, 225, 254, 255, 17,
  };
  for (const uint8_t cyan : kChannels) {
    for (const uint8_t magenta : kChannels) {
      for (const uint8_t yellow : kChannels) {
        for (const uint8_t key : kChannels) {
          const auto expected = fxge::AdobeCmykToStandardRgbReferenceForTesting(
              cyan, magenta, yellow, key);
          const auto actual =
              AdobeCmykToStandardRgb(cyan, magenta, yellow, key);
          ASSERT_EQ(expected.red, actual.red)
              << "cyan=" << static_cast<int>(cyan)
              << " magenta=" << static_cast<int>(magenta)
              << " yellow=" << static_cast<int>(yellow)
              << " key=" << static_cast<int>(key);
          ASSERT_EQ(expected.green, actual.green);
          ASSERT_EQ(expected.blue, actual.blue);
        }
      }
    }
  }
}

TEST(fxge, RustCmykBatchPreservesPdfiumBgrOrder) {
  static constexpr size_t kPixelCount = 256;
  DataVector<uint8_t> source(kPixelCount * 4);
  DataVector<uint8_t> expected(kPixelCount * 3);
  for (size_t pixel = 0; pixel < kPixelCount; ++pixel) {
    const uint8_t cyan = static_cast<uint8_t>(pixel);
    const uint8_t magenta = static_cast<uint8_t>(255 - pixel);
    const uint8_t yellow = static_cast<uint8_t>(pixel * 37);
    const uint8_t key = static_cast<uint8_t>(pixel * 73);
    source[pixel * 4] = cyan;
    source[pixel * 4 + 1] = magenta;
    source[pixel * 4 + 2] = yellow;
    source[pixel * 4 + 3] = key;
    const auto rgb = fxge::AdobeCmykToStandardRgbReferenceForTesting(
        cyan, magenta, yellow, key);
    expected[pixel * 3] = rgb.blue;
    expected[pixel * 3 + 1] = rgb.green;
    expected[pixel * 3 + 2] = rgb.red;
  }

  DataVector<uint8_t> actual(expected.size());
  ASSERT_TRUE(fxge::RustBlendAdapter::ConvertCmykToBgrRow(source, actual));
  EXPECT_EQ(expected, actual);
}
