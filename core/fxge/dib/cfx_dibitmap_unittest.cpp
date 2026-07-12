// Copyright 2018 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cfx_dibitmap.h"

#include <stdint.h>

#include <array>

#include "core/fxcrt/data_vector.h"
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

TEST(CFXDIBitmapTest, RustPitchAndSizeMatchesCppReference) {
  static constexpr std::array<FXDIB_Format, 8> kFormats = {
      FXDIB_Format::kInvalid,   FXDIB_Format::k1bppRgb,
      FXDIB_Format::k8bppRgb,  FXDIB_Format::kBgr,
      FXDIB_Format::kBgrx,     FXDIB_Format::k1bppMask,
      FXDIB_Format::k8bppMask, FXDIB_Format::kBgra,
  };
  static constexpr std::array<int, 9> kWidths = {
      -1, 0, 1, 7, 31, 32, 33, 536870908, 1073747000};
  static constexpr std::array<int, 7> kHeights = {
      -1, 0, 1, 4, 63, 1024, 1048576};
  static constexpr std::array<uint32_t, 8> kPitches = {
      0, 1, 4, 32, 400, 455, 2147484000u, UINT32_MAX};

  for (const FXDIB_Format format : kFormats) {
    for (const int width : kWidths) {
      for (const int height : kHeights) {
        for (const uint32_t pitch : kPitches) {
          std::optional<CFX_DIBitmap::PitchAndSize> reference;
          {
            fxge::ScopedRustDibImplementationForTesting implementation(false);
            reference = CFX_DIBitmap::CalculatePitchAndSize(width, height,
                                                            format, pitch);
          }
          const auto candidate = fxge::RustBlendAdapter::CalculatePitchAndSize(
              width, height, format, pitch);
          ASSERT_EQ(reference.has_value(), candidate.has_value())
              << "format=" << static_cast<int>(format) << " width=" << width
              << " height=" << height << " pitch=" << pitch;
          if (reference.has_value()) {
            EXPECT_EQ(reference->pitch, (*candidate)[0]);
            EXPECT_EQ(reference->size, (*candidate)[1]);
          }
        }
      }
    }
  }
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

TEST(CFXDIBitmapTest, RustBgraConvertFormatMatchesCppReference) {
  struct SourceCase {
    FXDIB_Format format;
    bool custom_palette;
  };
  static constexpr std::array<SourceCase, 6> kSources = {
      SourceCase{FXDIB_Format::k8bppMask, false},
      SourceCase{FXDIB_Format::k8bppRgb, false},
      SourceCase{FXDIB_Format::k8bppRgb, true},
      SourceCase{FXDIB_Format::kBgr, false},
      SourceCase{FXDIB_Format::kBgrx, false},
      SourceCase{FXDIB_Format::kBgra, false},
  };
  for (const auto& source_case : kSources) {
    auto reference = CreatePatternedBitmap(source_case.format);
    auto candidate = CreatePatternedBitmap(source_case.format);
    ASSERT_TRUE(reference);
    ASSERT_TRUE(candidate);
    if (source_case.custom_palette) {
      const int index = source_case.format == FXDIB_Format::k1bppRgb ? 1 : 173;
      reference->SetPaletteArgb(index, 0x8042a7e1);
      candidate->SetPaletteArgb(index, 0x8042a7e1);
    }
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      ASSERT_TRUE(reference->ConvertFormat(FXDIB_Format::kBgra));
    }
    ASSERT_TRUE(candidate->ConvertFormat(FXDIB_Format::kBgra));
    EXPECT_THAT(candidate->GetBuffer(),
                ElementsAreArray(reference->GetBuffer()))
        << "source_format=" << static_cast<int>(source_case.format)
        << " custom_palette=" << source_case.custom_palette;
  }
}

TEST(CFXDIBitmapTest, RustExpand1bppMaskMatchesCppReference) {
  auto reference = CreatePatternedBitmap(FXDIB_Format::k8bppMask, 9, 3);
  auto candidate = CreatePatternedBitmap(FXDIB_Format::k8bppMask, 9, 3);
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  static constexpr uint32_t kSourcePitch = 4;
  DataVector<uint8_t> source(kSourcePitch * 3);
  for (size_t index = 0; index < source.size(); ++index) {
    source[index] = static_cast<uint8_t>(index * 53 + 7);
  }
  {
    fxge::ScopedRustDibImplementationForTesting implementation(false);
    reference->Populate8bbpMaskFrom1bppSpan(source, kSourcePitch);
  }
  candidate->Populate8bbpMaskFrom1bppSpan(source, kSourcePitch);
  EXPECT_THAT(candidate->GetBuffer(),
              ElementsAreArray(reference->GetBuffer()));
}

TEST(CFXDIBitmapTest, RustPopulateFromSpanMatchesCppReference) {
  for (const uint32_t source_pitch : {19u, 29u}) {
    auto reference = CreatePatternedBitmap(FXDIB_Format::kBgr, 7, 3);
    auto candidate = CreatePatternedBitmap(FXDIB_Format::kBgr, 7, 3);
    ASSERT_TRUE(reference);
    ASSERT_TRUE(candidate);
    DataVector<uint8_t> source(source_pitch * 3);
    for (size_t index = 0; index < source.size(); ++index) {
      source[index] = static_cast<uint8_t>(index * 31 + 11);
    }
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference->PopulateFromSpan(source, source_pitch);
    }
    candidate->PopulateFromSpan(source, source_pitch);
    EXPECT_THAT(candidate->GetBuffer(),
                ElementsAreArray(reference->GetBuffer()))
        << "source_pitch=" << source_pitch;
  }
}

TEST(CFXDIBitmapTest, RustEqualFormatTransferMatchesCppReference) {
  static constexpr std::array<FXDIB_Format, 7> kFormats = {
      FXDIB_Format::k1bppMask, FXDIB_Format::k1bppRgb,
      FXDIB_Format::k8bppMask, FXDIB_Format::k8bppRgb,
      FXDIB_Format::kBgr,      FXDIB_Format::kBgrx,
      FXDIB_Format::kBgra,
  };
  struct TransferCase {
    int source_left;
    int source_top;
  };
  static constexpr std::array<TransferCase, 2> kCases = {
      TransferCase{2, 1}, TransferCase{-2, -1}};

  for (const FXDIB_Format format : kFormats) {
    for (const auto& transfer : kCases) {
      auto source = CreatePatternedBitmap(format, 11, 5);
      auto reference = CreatePatternedBitmap(format, 9, 4);
      auto candidate = CreatePatternedBitmap(format, 9, 4);
      ASSERT_TRUE(source);
      ASSERT_TRUE(reference);
      ASSERT_TRUE(candidate);
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        ASSERT_TRUE(reference->TransferBitmap(
            7, 3, source, transfer.source_left, transfer.source_top));
      }
      ASSERT_TRUE(candidate->TransferBitmap(
          7, 3, source, transfer.source_left, transfer.source_top));
      EXPECT_THAT(candidate->GetBuffer(),
                  ElementsAreArray(reference->GetBuffer()))
          << "format=" << static_cast<int>(format)
          << " source_left=" << transfer.source_left
          << " source_top=" << transfer.source_top;
    }
  }
}

TEST(CFXDIBitmapTest, RustOneBppMaskCompositeMatchesCppReference) {
  struct CompositeCase {
    int destination_left;
    int destination_top;
    int width;
    int height;
    int source_left;
    int source_top;
  };
  static constexpr std::array<CompositeCase, 3> kCases = {
      CompositeCase{2, 1, 7, 3, 1, 1},
      CompositeCase{-2, -1, 9, 4, 1, 0},
      CompositeCase{3, 0, 8, 3, -2, 1},
  };
  for (const auto& composite : kCases) {
    auto source = CreatePatternedBitmap(FXDIB_Format::k1bppMask, 11, 5);
    auto reference = CreatePatternedBitmap(FXDIB_Format::k1bppRgb, 9, 4);
    auto candidate = CreatePatternedBitmap(FXDIB_Format::k1bppRgb, 9, 4);
    ASSERT_TRUE(source);
    ASSERT_TRUE(reference);
    ASSERT_TRUE(candidate);
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference->CompositeOneBPPMask(
          composite.destination_left, composite.destination_top,
          composite.width, composite.height, source, composite.source_left,
          composite.source_top);
    }
    candidate->CompositeOneBPPMask(
        composite.destination_left, composite.destination_top, composite.width,
        composite.height, source, composite.source_left, composite.source_top);
    EXPECT_THAT(candidate->GetBuffer(),
                ElementsAreArray(reference->GetBuffer()))
        << "destination_left=" << composite.destination_left
        << " destination_top=" << composite.destination_top
        << " source_left=" << composite.source_left
        << " source_top=" << composite.source_top;
  }
}

TEST(CFXDIBitmapTest, RustCompositeRectMatchesCppReferenceAcrossFormats) {
  struct FormatCase {
    FXDIB_Format format;
    bool custom_palette;
  };
  static constexpr std::array<FormatCase, 8> kFormats = {
      FormatCase{FXDIB_Format::k1bppMask, false},
      FormatCase{FXDIB_Format::k1bppRgb, false},
      FormatCase{FXDIB_Format::k1bppRgb, true},
      FormatCase{FXDIB_Format::k8bppMask, false},
      FormatCase{FXDIB_Format::k8bppRgb, false},
      FormatCase{FXDIB_Format::kBgr, false},
      FormatCase{FXDIB_Format::kBgrx, false},
      FormatCase{FXDIB_Format::kBgra, false},
  };
  struct RectCase {
    int left;
    int top;
    int width;
    int height;
  };
  static constexpr std::array<RectCase, 3> kRects = {
      RectCase{1, 1, 5, 2},
      RectCase{-2, -1, 6, 4},
      RectCase{7, 2, 9, 4},
  };
  static constexpr std::array<uint32_t, 4> kColors = {
      0x00000000, 0xffffffff, 0xff4287e1, 0x8042a7e1};

  for (const auto& format_case : kFormats) {
    for (const auto& rect : kRects) {
      for (const uint32_t color : kColors) {
        auto reference = CreatePatternedBitmap(format_case.format, 9, 4);
        auto candidate = CreatePatternedBitmap(format_case.format, 9, 4);
        ASSERT_TRUE(reference);
        ASSERT_TRUE(candidate);
        if (format_case.custom_palette) {
          reference->SetPaletteArgb(1, 0x8042a7e1);
          candidate->SetPaletteArgb(1, 0x8042a7e1);
        }
        bool reference_result;
        {
          fxge::ScopedRustDibImplementationForTesting implementation(false);
          reference_result = reference->CompositeRect(
              rect.left, rect.top, rect.width, rect.height, color);
        }
        const bool candidate_result = candidate->CompositeRect(
            rect.left, rect.top, rect.width, rect.height, color);
        ASSERT_EQ(reference_result, candidate_result);
        EXPECT_THAT(candidate->GetBuffer(),
                    ElementsAreArray(reference->GetBuffer()))
            << "format=" << static_cast<int>(format_case.format)
            << " custom_palette=" << format_case.custom_palette
            << " left=" << rect.left << " top=" << rect.top
            << " width=" << rect.width << " height=" << rect.height
            << " color=" << color;
      }
    }
  }
}

TEST(CFXDIBitmapTest, RustCloneAlphaMaskMatchesCppReference) {
  auto source = CreatePatternedBitmap(FXDIB_Format::kBgra, 7, 3);
  ASSERT_TRUE(source);
  RetainPtr<CFX_DIBitmap> reference;
  {
    fxge::ScopedRustDibImplementationForTesting implementation(false);
    reference = source->CloneAlphaMask();
  }
  auto candidate = source->CloneAlphaMask();
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  ASSERT_EQ(reference->GetFormat(), candidate->GetFormat());
  ASSERT_EQ(reference->GetWidth(), candidate->GetWidth());
  ASSERT_EQ(reference->GetHeight(), candidate->GetHeight());
  for (int row = 0; row < reference->GetHeight(); ++row) {
    for (int column = 0; column < reference->GetWidth(); ++column) {
      EXPECT_EQ(reference->GetScanline(row)[column],
                candidate->GetScanline(row)[column])
          << "row=" << row << " column=" << column;
    }
  }
}

TEST(CFXDIBitmapTest, RustCopyMatchesCppReferenceAcrossFormats) {
  struct SourceCase {
    FXDIB_Format format;
    bool custom_palette;
  };
  static constexpr std::array<SourceCase, 9> kSources = {
      SourceCase{FXDIB_Format::k1bppMask, false},
      SourceCase{FXDIB_Format::k1bppRgb, false},
      SourceCase{FXDIB_Format::k1bppRgb, true},
      SourceCase{FXDIB_Format::k8bppMask, false},
      SourceCase{FXDIB_Format::k8bppRgb, false},
      SourceCase{FXDIB_Format::k8bppRgb, true},
      SourceCase{FXDIB_Format::kBgr, false},
      SourceCase{FXDIB_Format::kBgrx, false},
      SourceCase{FXDIB_Format::kBgra, false},
  };
  for (const auto& source_case : kSources) {
    auto source = CreatePatternedBitmap(source_case.format, 7, 3);
    ASSERT_TRUE(source);
    if (source_case.custom_palette) {
      const int index =
          source_case.format == FXDIB_Format::k1bppRgb ? 1 : 173;
      source->SetPaletteArgb(index, 0x8042a7e1);
    }
    auto reference = pdfium::MakeRetain<CFX_DIBitmap>();
    auto candidate = pdfium::MakeRetain<CFX_DIBitmap>();
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      ASSERT_TRUE(reference->Copy(source));
    }
    ASSERT_TRUE(candidate->Copy(source));
    ASSERT_EQ(reference->GetFormat(), candidate->GetFormat());
    ASSERT_EQ(reference->GetWidth(), candidate->GetWidth());
    ASSERT_EQ(reference->GetHeight(), candidate->GetHeight());
    EXPECT_THAT(candidate->GetBuffer(),
                ElementsAreArray(reference->GetBuffer()))
        << "format=" << static_cast<int>(source_case.format)
        << " custom_palette=" << source_case.custom_palette;
    ASSERT_EQ(reference->GetPaletteSpan().size(),
              candidate->GetPaletteSpan().size());
    for (size_t index = 0; index < reference->GetPaletteSpan().size();
         ++index) {
      EXPECT_EQ(reference->GetPaletteSpan()[index],
                candidate->GetPaletteSpan()[index]);
    }
  }
}
