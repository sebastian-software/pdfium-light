// Copyright 2018 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cfx_dibitmap.h"

#include <stdint.h>

#include "core/fxcrt/fx_coordinates.h"
#include "core/fxcrt/span.h"
#include "core/fxge/dib/fx_dib.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

using ::testing::ElementsAre;
using ::testing::ElementsAreArray;

RetainPtr<CFX_DIBitmap> CreatePatternedBitmap(FXDIB_Format format,
                                              int width = 7,
                                              int height = 3) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  if (!bitmap->Create(width, height, format)) {
    return nullptr;
  }
  auto buffer = bitmap->GetWritableBuffer();
  for (size_t index = 0; index < buffer.size(); ++index) {
    buffer[index] = static_cast<uint8_t>((index * 73 + 19) % 256);
  }
  return bitmap;
}

}  // namespace

TEST(CFXDIBitmapTest, Create) {
  auto pBitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_FALSE(pBitmap->Create(400, 300, FXDIB_Format::kInvalid));

  pBitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(pBitmap->Create(400, 300, FXDIB_Format::k1bppRgb));
}

TEST(CFXDIBitmapTest, CalculatePitchAndSizeGood) {
  // Simple case with no provided pitch.
  std::optional<CFX_DIBitmap::PitchAndSize> result =
      CFX_DIBitmap::CalculatePitchAndSize(100, 200, FXDIB_Format::kBgra, 0);
  ASSERT_TRUE(result.has_value());
  EXPECT_EQ(400u, result.value().pitch);
  EXPECT_EQ(80000u, result.value().size);

  // Simple case with no provided pitch and different format.
  result =
      CFX_DIBitmap::CalculatePitchAndSize(100, 200, FXDIB_Format::k8bppRgb, 0);
  ASSERT_TRUE(result.has_value());
  EXPECT_EQ(100u, result.value().pitch);
  EXPECT_EQ(20000u, result.value().size);

  // Simple case with provided pitch matching width * bpp.
  result =
      CFX_DIBitmap::CalculatePitchAndSize(100, 200, FXDIB_Format::kBgra, 400);
  ASSERT_TRUE(result.has_value());
  EXPECT_EQ(400u, result.value().pitch);
  EXPECT_EQ(80000u, result.value().size);

  // Simple case with provided pitch, where pitch exceeds width * bpp.
  result =
      CFX_DIBitmap::CalculatePitchAndSize(100, 200, FXDIB_Format::kBgra, 455);
  ASSERT_TRUE(result.has_value());
  EXPECT_EQ(455u, result.value().pitch);
  EXPECT_EQ(91000u, result.value().size);
}

TEST(CFXDIBitmapTest, CalculatePitchAndSizeBad) {
  // Bad width / height.
  static const CFX_Size kBadDimensions[] = {
      {0, 0},   {-1, -1}, {-1, 0},   {0, -1},
      {0, 200}, {100, 0}, {-1, 200}, {100, -1},
  };
  for (const auto& dimension : kBadDimensions) {
    EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
        dimension.width, dimension.height, FXDIB_Format::kBgra, 0));
    EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
        dimension.width, dimension.height, FXDIB_Format::kBgra, 1));
  }

  // Bad format.
  EXPECT_FALSE(
      CFX_DIBitmap::CalculatePitchAndSize(100, 200, FXDIB_Format::kInvalid, 0));
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
      100, 200, FXDIB_Format::kInvalid, 800));

  // Width too wide for claimed pitch.
  EXPECT_FALSE(
      CFX_DIBitmap::CalculatePitchAndSize(101, 200, FXDIB_Format::kBgra, 400));

  // Overflow cases with calculated pitch.
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(1073747000, 1,
                                                   FXDIB_Format::kBgra, 0));
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(1048576, 1024,
                                                   FXDIB_Format::kBgra, 0));
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(4194304, 1024,
                                                   FXDIB_Format::k8bppRgb, 0));

  // Overflow cases with provided pitch.
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
      2147484000u, 1, FXDIB_Format::kBgra, 2147484000u));
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
      1048576, 1024, FXDIB_Format::kBgra, 4194304));
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(
      4194304, 1024, FXDIB_Format::k8bppRgb, 4194304));
}

TEST(CFXDIBitmapTest, CalculatePitchAndSizeBoundary) {
  // Test boundary condition for pitch overflow.
  std::optional<CFX_DIBitmap::PitchAndSize> result =
      CFX_DIBitmap::CalculatePitchAndSize(536870908, 4, FXDIB_Format::k8bppRgb,
                                          0);
  ASSERT_TRUE(result);
  EXPECT_EQ(536870908u, result.value().pitch);
  EXPECT_EQ(2147483632u, result.value().size);
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(536870909, 4,
                                                   FXDIB_Format::k8bppRgb, 0));

  // Test boundary condition for size overflow.
  result = CFX_DIBitmap::CalculatePitchAndSize(68174084, 63,
                                               FXDIB_Format::k8bppRgb, 0);
  ASSERT_TRUE(result);
  EXPECT_EQ(68174084u, result.value().pitch);
  EXPECT_EQ(4294967292u, result.value().size);
  EXPECT_FALSE(CFX_DIBitmap::CalculatePitchAndSize(68174085, 63,
                                                   FXDIB_Format::k8bppRgb, 0));
}

TEST(CFXDIBitmapTest, GetScanlineAsWith24Bpp) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  ASSERT_TRUE(bitmap->Create(3, 3, FXDIB_Format::kBgr));
  EXPECT_EQ(3, bitmap->GetWidth());
  EXPECT_EQ(12u, bitmap->GetPitch());

  EXPECT_EQ(36u, bitmap->GetBuffer().size());
  EXPECT_EQ(12u, bitmap->GetScanline(0).size());
  EXPECT_EQ(3u, bitmap->GetScanlineAs<FX_BGR_STRUCT<uint8_t>>(0).size());

  EXPECT_EQ(36u, bitmap->GetWritableBuffer().size());
  EXPECT_EQ(12u, bitmap->GetWritableScanline(0).size());
  EXPECT_EQ(3u,
            bitmap->GetWritableScanlineAs<FX_BGR_STRUCT<uint8_t>>(0).size());
}

TEST(CFXDIBitmapTest, RustSetRedFromAlphaMatchesCppReference) {
  auto reference = CreatePatternedBitmap(FXDIB_Format::kBgra);
  auto candidate = CreatePatternedBitmap(FXDIB_Format::kBgra);
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  {
    fxge::ScopedRustDibImplementationForTesting implementation(false);
    reference->SetRedFromAlpha();
  }
  candidate->SetRedFromAlpha();
  EXPECT_THAT(candidate->GetBuffer(),
              ElementsAreArray(reference->GetBuffer()));
}

TEST(CFXDIBitmapTest, RustSetOpaqueAlphaMatchesCppReference) {
  auto reference = CreatePatternedBitmap(FXDIB_Format::kBgra);
  auto candidate = CreatePatternedBitmap(FXDIB_Format::kBgra);
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  {
    fxge::ScopedRustDibImplementationForTesting implementation(false);
    reference->SetUniformOpaqueAlpha();
  }
  candidate->SetUniformOpaqueAlpha();
  EXPECT_THAT(candidate->GetBuffer(),
              ElementsAreArray(reference->GetBuffer()));
}

TEST(CFXDIBitmapTest, RustMultiplyAlphaMaskMatchesCppReference) {
  auto mask = CreatePatternedBitmap(FXDIB_Format::k8bppMask);
  ASSERT_TRUE(mask);
  for (const FXDIB_Format format :
       {FXDIB_Format::kBgra, FXDIB_Format::kBgrx}) {
    auto reference = CreatePatternedBitmap(format);
    auto candidate = CreatePatternedBitmap(format);
    ASSERT_TRUE(reference);
    ASSERT_TRUE(candidate);
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      ASSERT_TRUE(reference->MultiplyAlphaMask(mask));
    }
    ASSERT_TRUE(candidate->MultiplyAlphaMask(mask));
    EXPECT_EQ(reference->GetFormat(), candidate->GetFormat());
    EXPECT_THAT(candidate->GetBuffer(),
                ElementsAreArray(reference->GetBuffer()));
  }
}

TEST(CFXDIBitmapTest, RustMultiplyAlphaMatchesCppReference) {
  auto reference = CreatePatternedBitmap(FXDIB_Format::kBgra);
  auto candidate = CreatePatternedBitmap(FXDIB_Format::kBgra);
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  {
    fxge::ScopedRustDibImplementationForTesting implementation(false);
    ASSERT_TRUE(reference->MultiplyAlpha(0.37f));
  }
  ASSERT_TRUE(candidate->MultiplyAlpha(0.37f));
  EXPECT_THAT(candidate->GetBuffer(),
              ElementsAreArray(reference->GetBuffer()));
}

TEST(CFXDIBitmapTest, RustClearMatchesCppReferenceAcrossFormats) {
  static constexpr std::array<FXDIB_Format, 7> kFormats = {
      FXDIB_Format::k1bppMask, FXDIB_Format::k1bppRgb,
      FXDIB_Format::k8bppMask, FXDIB_Format::k8bppRgb,
      FXDIB_Format::kBgr,      FXDIB_Format::kBgrx,
      FXDIB_Format::kBgra,
  };
  static constexpr std::array<uint32_t, 3> kColors = {
      0x00000000, 0xff5a5a5a, 0x8012a7e4};
  for (const FXDIB_Format format : kFormats) {
    for (const uint32_t color : kColors) {
      auto reference = CreatePatternedBitmap(format);
      auto candidate = CreatePatternedBitmap(format);
      ASSERT_TRUE(reference);
      ASSERT_TRUE(candidate);
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        reference->Clear(color);
      }
      candidate->Clear(color);
      EXPECT_THAT(candidate->GetBuffer(),
                  ElementsAreArray(reference->GetBuffer()))
          << "format=" << static_cast<int>(format) << " color=" << color;
    }
  }
}

TEST(CFXDIBitmapTest, RustColorScaleMatchesCppReferenceAcrossFormats) {
  for (const FXDIB_Format format :
       {FXDIB_Format::kBgr, FXDIB_Format::kBgrx, FXDIB_Format::kBgra}) {
    for (const bool is_white_on_black : {false, true}) {
      auto reference = CreatePatternedBitmap(format);
      auto candidate = CreatePatternedBitmap(format);
      ASSERT_TRUE(reference);
      ASSERT_TRUE(candidate);
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        reference->ConvertColorScale(is_white_on_black);
      }
      candidate->ConvertColorScale(is_white_on_black);
      EXPECT_THAT(candidate->GetBuffer(),
                  ElementsAreArray(reference->GetBuffer()))
          << "format=" << static_cast<int>(format)
          << " is_white_on_black=" << is_white_on_black;
    }
  }
}
