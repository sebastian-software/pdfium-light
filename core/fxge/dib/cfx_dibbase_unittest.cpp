// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cfx_dibbase.h"

#include <array>
#include <limits>
#include <optional>

#include "core/fxcrt/fx_coordinates.h"
#include "core/fxge/dib/cfx_dibitmap.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

struct Input {
  CFX_Point src_top_left;
  CFX_Size src_size;
  CFX_Point dest_top_left;
  CFX_Size overlap_size;
};

struct Output {
  CFX_Point src_top_left;
  CFX_Point dest_top_left;
  CFX_Size overlap_size;
};

void RunOverlapRectTest(const CFX_DIBitmap* bitmap,
                        const Input& input,
                        const Output* expected_output) {
  // Initialize in-out parameters.
  int src_left = input.src_top_left.x;
  int src_top = input.src_top_left.y;
  int dest_left = input.dest_top_left.x;
  int dest_top = input.dest_top_left.y;
  int overlap_width = input.overlap_size.width;
  int overlap_height = input.overlap_size.height;

  bool success = bitmap->GetOverlapRect(
      dest_left, dest_top, overlap_width, overlap_height, input.src_size.width,
      input.src_size.height, src_left, src_top,
      /*pClipRgn=*/nullptr);
  if (success == !expected_output) {
    ADD_FAILURE();
    return;
  }

  if (expected_output) {
    EXPECT_EQ(expected_output->src_top_left.x, src_left);
    EXPECT_EQ(expected_output->src_top_left.y, src_top);
    EXPECT_EQ(expected_output->dest_top_left.x, dest_left);
    EXPECT_EQ(expected_output->dest_top_left.y, dest_top);
    EXPECT_EQ(expected_output->overlap_size.width, overlap_width);
    EXPECT_EQ(expected_output->overlap_size.height, overlap_height);
  }
}

RetainPtr<CFX_DIBitmap> CreateConversionBitmap(FXDIB_Format format,
                                               bool custom_palette) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  if (!bitmap->Create(7, 3, format)) {
    return nullptr;
  }
  auto buffer = bitmap->GetWritableBuffer();
  for (size_t index = 0; index < buffer.size(); ++index) {
    buffer[index] = static_cast<uint8_t>(index * 61 + 23);
  }
  if (custom_palette) {
    const int index = format == FXDIB_Format::k1bppRgb ? 1 : 173;
    bitmap->SetPaletteArgb(index, 0x8042a7e1);
  }
  return bitmap;
}

}  // namespace

TEST(CFXDIBBaseTest, GetOverlapRectTrivialOverlap) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{0, 0},
                        /*overlap_size=*/{400, 300}};
  const Output kExpectedOutput = {/*src_top_left=*/{0, 0},
                                  /*dest_top_left=*/{0, 0},
                                  /*overlap_size=*/{400, 300}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapNoLimit) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{35, 41}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{123, 137},
                        /*overlap_size=*/{200, 100}};
  const Output kExpectedOutput = {/*src_top_left=*/{35, 41},
                                  /*dest_top_left=*/{123, 137},
                                  /*overlap_size=*/{200, 100}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapLimitedBySource) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{141, 154}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{35, 41},
                        /*overlap_size=*/{270, 160}};
  const Output kExpectedOutput = {/*src_top_left=*/{141, 154},
                                  /*dest_top_left=*/{35, 41},
                                  /*overlap_size=*/{259, 146}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapLimitedByDestination) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{35, 41}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{123, 137},
                        /*overlap_size=*/{280, 170}};
  const Output kExpectedOutput = {/*src_top_left=*/{35, 41},
                                  /*dest_top_left=*/{123, 137},
                                  /*overlap_size=*/{277, 163}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectBadInputs) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kEmptyInputs[] = {
      // Empty source rect.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{0, 0},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Empty overlap size.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{0, 0}},
      // Source out of bounds on x-axis.
      {/*src_top_left=*/{-400, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
  };
  for (const Input& input : kEmptyInputs) {
    RunOverlapRectTest(bitmap.Get(), input, /*expected_output=*/nullptr);
  }

  const Input kOutOfBoundInputs[] = {
      // Source out of bounds on x-axis.
      {/*src_top_left=*/{400, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Source out of bounds on y-axis.
      {/*src_top_left=*/{0, 300}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Source out of bounds on y-axis.
      {/*src_top_left=*/{0, -300}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on x-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{-400, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on x-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{400, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on y-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, -300},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on y-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 300},
       /*overlap_size=*/{400, 300}},
  };
  for (const Input& input : kOutOfBoundInputs) {
    RunOverlapRectTest(bitmap.Get(), input, /*expected_output=*/nullptr);
  }
}

TEST(CFXDIBBaseTest, RustOverlapRectMatchesCppReferenceCorpus) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  ASSERT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  for (int index = 0; index < 1026; ++index) {
    int dest_left = (index * 97 % 901) - 450;
    int dest_top = (index * 53 % 701) - 350;
    int width = 1 + index * 37 % 500;
    int height = 1 + index * 29 % 400;
    const int src_width = 250 + index * 11 % 350;
    const int src_height = 200 + index * 17 % 300;
    int src_left = (index * 71 % 801) - 400;
    int src_top = (index * 43 % 601) - 300;
    if (index == 1024) {
      src_left = std::numeric_limits<int>::max() - 10;
      width = 100;
    } else if (index == 1025) {
      dest_left = std::numeric_limits<int>::min() + 5;
      src_left = std::numeric_limits<int>::max() - 5;
      width = 20;
    }
    const std::optional<FX_RECT> clip =
        index % 3 == 0
            ? std::optional<FX_RECT>(FX_RECT(-20 + index % 40,
                                             -10 + index % 30,
                                             200 + index % 250,
                                             150 + index % 200))
            : std::nullopt;

    int reference_dest_left = dest_left;
    int reference_dest_top = dest_top;
    int reference_width = width;
    int reference_height = height;
    int reference_src_left = src_left;
    int reference_src_top = src_top;
    bool reference_success;
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference_success = bitmap->GetOverlapRect(
          reference_dest_left, reference_dest_top, reference_width,
          reference_height, src_width, src_height, reference_src_left,
          reference_src_top, clip ? &clip.value() : nullptr);
    }
    const auto candidate = fxge::RustBlendAdapter::GetOverlapRect(
        bitmap->GetWidth(), bitmap->GetHeight(), dest_left, dest_top, width,
        height, src_width, src_height, src_left, src_top, clip.has_value(),
        clip ? clip->left : 0, clip ? clip->top : 0, clip ? clip->right : 0,
        clip ? clip->bottom : 0);
    ASSERT_EQ(reference_success, candidate.has_value()) << "index=" << index;
    if (reference_success) {
      EXPECT_EQ(reference_dest_left, (*candidate)[0]);
      EXPECT_EQ(reference_dest_top, (*candidate)[1]);
      EXPECT_EQ(reference_width, (*candidate)[2]);
      EXPECT_EQ(reference_height, (*candidate)[3]);
      EXPECT_EQ(reference_src_left, (*candidate)[4]);
      EXPECT_EQ(reference_src_top, (*candidate)[5]);
    }
  }
}

TEST(CFXDIBBaseTest, RustDefaultPaletteMatchesCppReference) {
  for (const FXDIB_Format format :
       {FXDIB_Format::k1bppRgb, FXDIB_Format::k8bppRgb}) {
    auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
    ASSERT_TRUE(bitmap->Create(4, 3, format));
    const int palette_size = format == FXDIB_Format::k1bppRgb ? 2 : 256;
    for (int index = 0; index < palette_size; ++index) {
      uint32_t reference;
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        reference = bitmap->GetPaletteArgb(index);
      }
      EXPECT_EQ(reference, bitmap->GetPaletteArgb(index))
          << "format=" << static_cast<int>(format) << " index=" << index;
    }
  }
}

TEST(CFXDIBBaseTest, RustPaletteBuildMatchesCppReference) {
  for (const FXDIB_Format format :
       {FXDIB_Format::k1bppRgb, FXDIB_Format::k8bppRgb}) {
    auto reference = pdfium::MakeRetain<CFX_DIBitmap>();
    auto candidate = pdfium::MakeRetain<CFX_DIBitmap>();
    ASSERT_TRUE(reference->Create(4, 3, format));
    ASSERT_TRUE(candidate->Create(4, 3, format));
    const int index = format == FXDIB_Format::k1bppRgb ? 1 : 173;
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference->SetPaletteArgb(index, 0x8042a7e1);
    }
    candidate->SetPaletteArgb(index, 0x8042a7e1);
    ASSERT_EQ(reference->GetPaletteSpan().size(),
              candidate->GetPaletteSpan().size());
    for (size_t entry = 0; entry < reference->GetPaletteSpan().size();
         ++entry) {
      EXPECT_EQ(reference->GetPaletteSpan()[entry],
                candidate->GetPaletteSpan()[entry])
          << "format=" << static_cast<int>(format) << " entry=" << entry;
    }
  }
}

TEST(CFXDIBBaseTest, RustCustomPaletteLookupMatchesCppReference) {
  for (const FXDIB_Format format :
       {FXDIB_Format::k1bppRgb, FXDIB_Format::k8bppRgb}) {
    auto reference = pdfium::MakeRetain<CFX_DIBitmap>();
    auto candidate = pdfium::MakeRetain<CFX_DIBitmap>();
    ASSERT_TRUE(reference->Create(7, 3, format));
    ASSERT_TRUE(candidate->Create(7, 3, format));
    const int index = format == FXDIB_Format::k1bppRgb ? 1 : 173;
    static constexpr uint32_t kColor = 0x8042a7e1;
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference->SetPaletteArgb(index, kColor);
      reference->Clear(kColor);
    }
    candidate->SetPaletteArgb(index, kColor);
    candidate->Clear(kColor);
    ASSERT_EQ(reference->GetBuffer().size(), candidate->GetBuffer().size());
    for (size_t byte = 0; byte < reference->GetBuffer().size(); ++byte) {
      EXPECT_EQ(reference->GetBuffer()[byte], candidate->GetBuffer()[byte])
          << "format=" << static_cast<int>(format) << " byte=" << byte;
    }
  }
}

TEST(CFXDIBBaseTest, RustConvertBufferMatchesCppReference) {
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
    auto source =
        CreateConversionBitmap(source_case.format, source_case.custom_palette);
    ASSERT_TRUE(source);
    for (const FXDIB_Format destination_format :
         {FXDIB_Format::k8bppRgb, FXDIB_Format::kBgr}) {
      if (destination_format == source_case.format) {
        continue;
      }
      if (destination_format == FXDIB_Format::k8bppRgb &&
          source->GetBPP() > 8) {
        continue;
      }
      RetainPtr<CFX_DIBitmap> reference;
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        reference = source->ConvertTo(destination_format);
      }
      auto candidate = source->ConvertTo(destination_format);
      ASSERT_EQ(static_cast<bool>(reference), static_cast<bool>(candidate));
      ASSERT_TRUE(reference);
      ASSERT_EQ(reference->GetFormat(), candidate->GetFormat());
      const size_t active_row_bytes =
          static_cast<size_t>(reference->GetWidth()) *
          GetCompsFromFormat(reference->GetFormat());
      for (int row = 0; row < reference->GetHeight(); ++row) {
        for (size_t byte = 0; byte < active_row_bytes; ++byte) {
          EXPECT_EQ(reference->GetScanline(row)[byte],
                    candidate->GetScanline(row)[byte])
              << "source_format=" << static_cast<int>(source_case.format)
              << " custom_palette=" << source_case.custom_palette
              << " destination_format=" << static_cast<int>(destination_format)
              << " row=" << row << " byte=" << byte;
        }
      }
      ASSERT_EQ(reference->GetPaletteSpan().size(),
                candidate->GetPaletteSpan().size());
      for (size_t index = 0; index < reference->GetPaletteSpan().size();
           ++index) {
        EXPECT_EQ(reference->GetPaletteSpan()[index],
                  candidate->GetPaletteSpan()[index]);
      }
    }
  }
}

TEST(CFXDIBBaseTest, OneBppClipHandlesMissingTrailingWord) {
  auto source = CreateConversionBitmap(FXDIB_Format::k1bppRgb, false);
  ASSERT_TRUE(source);
  fxge::ScopedRustDibImplementationForTesting implementation(false);
  auto clipped = source->ClipTo(FX_RECT(1, 0, 6, source->GetHeight()));
  ASSERT_TRUE(clipped);
  ASSERT_EQ(5, clipped->GetWidth());
  ASSERT_EQ(source->GetHeight(), clipped->GetHeight());
  for (int row = 0; row < clipped->GetHeight(); ++row) {
    for (int column = 0; column < clipped->GetWidth(); ++column) {
      const int source_column = column + 1;
      const bool source_bit =
          (source->GetScanline(row)[source_column / 8] &
           (1 << (7 - source_column % 8))) != 0;
      const bool clipped_bit =
          (clipped->GetScanline(row)[column / 8] &
           (1 << (7 - column % 8))) != 0;
      EXPECT_EQ(source_bit, clipped_bit)
          << "row=" << row << " column=" << column;
    }
  }
}

TEST(CFXDIBBaseTest, RustClipMatchesCppReferenceAcrossFormats) {
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
  static constexpr std::array<FX_RECT, 4> kRects = {
      FX_RECT(0, 0, 7, 3),
      FX_RECT(1, 0, 6, 3),
      FX_RECT(2, 1, 7, 3),
      FX_RECT(-2, -1, 5, 2),
  };
  for (const auto& source_case : kSources) {
    auto source =
        CreateConversionBitmap(source_case.format, source_case.custom_palette);
    ASSERT_TRUE(source);
    for (const auto& rect : kRects) {
      RetainPtr<CFX_DIBitmap> reference;
      {
        fxge::ScopedRustDibImplementationForTesting implementation(false);
        reference = source->ClipTo(rect);
      }
      auto candidate = source->ClipTo(rect);
      ASSERT_EQ(static_cast<bool>(reference), static_cast<bool>(candidate));
      ASSERT_TRUE(reference);
      ASSERT_EQ(reference->GetFormat(), candidate->GetFormat());
      ASSERT_EQ(reference->GetWidth(), candidate->GetWidth());
      ASSERT_EQ(reference->GetHeight(), candidate->GetHeight());
      const size_t active_row_bytes =
          (static_cast<size_t>(reference->GetWidth()) * reference->GetBPP() +
           7) /
          8;
      for (int row = 0; row < reference->GetHeight(); ++row) {
        for (size_t byte = 0; byte < active_row_bytes; ++byte) {
          EXPECT_EQ(reference->GetScanline(row)[byte],
                    candidate->GetScanline(row)[byte])
              << "format=" << static_cast<int>(source_case.format)
              << " custom_palette=" << source_case.custom_palette
              << " rect=" << rect.left << ',' << rect.top << ',' << rect.right
              << ',' << rect.bottom << " row=" << row << " byte=" << byte;
        }
      }
      ASSERT_EQ(reference->GetPaletteSpan().size(),
                candidate->GetPaletteSpan().size());
      for (size_t index = 0; index < reference->GetPaletteSpan().size();
           ++index) {
        EXPECT_EQ(reference->GetPaletteSpan()[index],
                  candidate->GetPaletteSpan()[index]);
      }
    }
  }
}

TEST(CFXDIBBaseTest, RustStretchMatchesCppReferenceAcrossFormats) {
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
  struct DestinationCase {
    int width;
    int height;
  };
  static constexpr std::array<DestinationCase, 4> kDestinations = {
      DestinationCase{11, 5},
      DestinationCase{3, 2},
      DestinationCase{-9, 4},
      DestinationCase{10, -5},
  };
  enum class OptionCase { kDefault, kNoSmoothing, kBilinear };
  for (const auto& source_case : kSources) {
    auto source =
        CreateConversionBitmap(source_case.format, source_case.custom_palette);
    ASSERT_TRUE(source);
    if (source_case.format == FXDIB_Format::k1bppRgb &&
        source_case.custom_palette) {
      source->SetPaletteArgb(1, 0xff42a7e1);
    }
    for (const auto& destination : kDestinations) {
      for (const OptionCase option_case :
           {OptionCase::kDefault, OptionCase::kNoSmoothing,
            OptionCase::kBilinear}) {
        FXDIB_ResampleOptions options;
        options.bNoSmoothing = option_case == OptionCase::kNoSmoothing;
        options.bInterpolateBilinear = option_case == OptionCase::kBilinear;
        RetainPtr<CFX_DIBitmap> reference;
        {
          fxge::ScopedRustDibImplementationForTesting implementation(false);
          reference = source->StretchTo(destination.width, destination.height,
                                        options, nullptr);
        }
        auto candidate = source->StretchTo(destination.width, destination.height,
                                           options, nullptr);
        ASSERT_EQ(static_cast<bool>(reference), static_cast<bool>(candidate));
        ASSERT_TRUE(reference);
        ASSERT_EQ(reference->GetFormat(), candidate->GetFormat());
        ASSERT_EQ(reference->GetWidth(), candidate->GetWidth());
        ASSERT_EQ(reference->GetHeight(), candidate->GetHeight());
        const size_t active_row_bytes =
            static_cast<size_t>(reference->GetWidth()) * reference->GetBPP() /
            8;
        for (int row = 0; row < reference->GetHeight(); ++row) {
          for (size_t byte = 0; byte < active_row_bytes; ++byte) {
            EXPECT_EQ(reference->GetScanline(row)[byte],
                      candidate->GetScanline(row)[byte])
                << "source_format=" << static_cast<int>(source_case.format)
                << " custom_palette=" << source_case.custom_palette
                << " destination=" << destination.width << 'x'
                << destination.height << " option="
                << static_cast<int>(option_case) << " row=" << row
                << " byte=" << byte;
          }
        }
        ASSERT_EQ(reference->GetPaletteSpan().size(),
                  candidate->GetPaletteSpan().size());
        for (size_t index = 0; index < reference->GetPaletteSpan().size();
             ++index) {
          EXPECT_EQ(reference->GetPaletteSpan()[index],
                    candidate->GetPaletteSpan()[index]);
        }
      }
    }
  }
}

TEST(CFXDIBBaseTest, RustSwapXYMatchesCppReferenceAcrossFormats) {
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
    auto source =
        CreateConversionBitmap(source_case.format, source_case.custom_palette);
    ASSERT_TRUE(source);
    for (bool flip_x : {false, true}) {
      for (bool flip_y : {false, true}) {
        RetainPtr<CFX_DIBitmap> reference;
        {
          fxge::ScopedRustDibImplementationForTesting implementation(false);
          reference = source->SwapXY(flip_x, flip_y);
        }
        auto candidate = source->SwapXY(flip_x, flip_y);
        ASSERT_TRUE(reference);
        ASSERT_TRUE(candidate);
        EXPECT_EQ(reference->GetFormat(), candidate->GetFormat());
        EXPECT_EQ(reference->GetWidth(), candidate->GetWidth());
        EXPECT_EQ(reference->GetHeight(), candidate->GetHeight());
        EXPECT_EQ(reference->GetPitch(), candidate->GetPitch());
        EXPECT_EQ(reference->GetBuffer(), candidate->GetBuffer())
            << "source_format=" << static_cast<int>(source_case.format)
            << " custom_palette=" << source_case.custom_palette
            << " flip_x=" << flip_x << " flip_y=" << flip_y;
        EXPECT_EQ(reference->GetPaletteSpan(), candidate->GetPaletteSpan());
      }
    }
  }
}
