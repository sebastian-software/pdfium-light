// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <stdint.h>

#include <array>

#include "core/fxcrt/data_vector.h"
#include "core/fxge/dib/cfx_scanlinecompositor.h"
#include "core/fxge/dib/blend.h"
#include "core/fxge/dib/fx_dib.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace fxge {
namespace {

constexpr size_t kChannelPairCount = 256 * 256;

TEST(RustBlendParityTest, EverySeparableModeAndChannelPairMatchesCpp) {
  std::array<uint8_t, kChannelPairCount> backdrop = {};
  std::array<uint8_t, kChannelPairCount> source = {};
  for (size_t index = 0; index < kChannelPairCount; ++index) {
    backdrop[index] = static_cast<uint8_t>(index / 256);
    source[index] = static_cast<uint8_t>(index % 256);
  }

  for (int mode_value = static_cast<int>(BlendMode::kNormal);
       mode_value <= static_cast<int>(BlendMode::kExclusion); ++mode_value) {
    const auto mode = static_cast<BlendMode>(mode_value);
    auto candidate = RustBlendAdapter::BlendChannels(mode, backdrop, source);
    ASSERT_TRUE(candidate.has_value());
    ASSERT_EQ(kChannelPairCount, candidate->size());
    for (size_t index = 0; index < kChannelPairCount; ++index) {
      ASSERT_EQ(Blend(mode, backdrop[index], source[index]),
                candidate.value()[index])
          << "mode=" << mode_value << " backdrop=" << int{backdrop[index]}
          << " source=" << int{source[index]};
    }
  }
}

TEST(RustBlendParityTest, RejectsMismatchedInputLengths) {
  const std::array<uint8_t, 1> backdrop = {0};
  const std::array<uint8_t, 2> source = {0, 1};
  EXPECT_FALSE(RustBlendAdapter::BlendChannels(BlendMode::kNormal, backdrop,
                                               source)
                   .has_value());
}

TEST(RustBlendParityTest, RejectsNonSeparableModes) {
  const std::array<uint8_t, 1> channel = {0};
  EXPECT_FALSE(RustBlendAdapter::BlendChannels(BlendMode::kHue, channel, channel)
                   .has_value());
}

TEST(RustBlendParityTest, BgraRowsMatchCppReferenceWithAndWithoutClip) {
  constexpr size_t kPixelCount = 257;
  DataVector<uint8_t> source(kPixelCount * 4);
  DataVector<uint8_t> initial_destination(kPixelCount * 4);
  DataVector<uint8_t> clip(kPixelCount);
  for (size_t pixel = 0; pixel < kPixelCount; ++pixel) {
    for (size_t channel = 0; channel < 4; ++channel) {
      source[pixel * 4 + channel] =
          static_cast<uint8_t>((pixel * 67 + channel * 43) % 256);
      initial_destination[pixel * 4 + channel] =
          static_cast<uint8_t>((pixel * 29 + channel * 97) % 256);
    }
    clip[pixel] = static_cast<uint8_t>((pixel * 53) % 256);
  }

  for (int mode_value = static_cast<int>(BlendMode::kNormal);
       mode_value <= static_cast<int>(BlendMode::kExclusion); ++mode_value) {
    const auto mode = static_cast<BlendMode>(mode_value);
    for (bool rgb_byte_order : {false, true}) {
      for (bool use_clip : {false, true}) {
        CFX_ScanlineCompositor compositor;
        ASSERT_TRUE(compositor.Init(FXDIB_Format::kBgra, FXDIB_Format::kBgra,
                                    /*src_palette=*/{}, /*mask_color=*/0, mode,
                                    rgb_byte_order));
        const pdfium::span<const uint8_t> clip_span =
            use_clip ? pdfium::span<const uint8_t>(clip)
                     : pdfium::span<const uint8_t>();
        DataVector<uint8_t> reference = initial_destination;
        {
          ScopedRustDibImplementationForTesting implementation(
              /*use_candidate=*/false);
          compositor.CompositeRgbBitmapLine(
              reference, source, static_cast<int>(kPixelCount), clip_span);
        }
        DataVector<uint8_t> candidate = initial_destination;
        {
          ScopedRustDibImplementationForTesting implementation(
              /*use_candidate=*/true);
          compositor.CompositeRgbBitmapLine(
              candidate, source, static_cast<int>(kPixelCount), clip_span);
        }
        EXPECT_EQ(reference, candidate)
            << "mode=" << mode_value << " rgb_byte_order=" << rgb_byte_order
            << " use_clip=" << use_clip;
      }
    }
  }
}

TEST(RustBlendParityTest, BgraToBgrRowsMatchCppReference) {
  constexpr size_t kPixelCount = 257;
  DataVector<uint8_t> source(kPixelCount * 4);
  DataVector<uint8_t> clip(kPixelCount);
  for (size_t index = 0; index < source.size(); ++index) {
    source[index] = static_cast<uint8_t>((index * 71) % 256);
  }
  for (size_t pixel = 0; pixel < clip.size(); ++pixel) {
    clip[pixel] = static_cast<uint8_t>((pixel * 47) % 256);
  }

  for (FXDIB_Format format : {FXDIB_Format::kBgr, FXDIB_Format::kBgrx}) {
    const size_t components = GetCompsFromFormat(format);
    DataVector<uint8_t> initial_destination(kPixelCount * components);
    for (size_t index = 0; index < initial_destination.size(); ++index) {
      initial_destination[index] = static_cast<uint8_t>((index * 31) % 256);
    }
    for (int mode_value = static_cast<int>(BlendMode::kNormal);
         mode_value <= static_cast<int>(BlendMode::kExclusion); ++mode_value) {
      const auto mode = static_cast<BlendMode>(mode_value);
      for (bool rgb_byte_order : {false, true}) {
        for (bool use_clip : {false, true}) {
          CFX_ScanlineCompositor compositor;
          ASSERT_TRUE(compositor.Init(format, FXDIB_Format::kBgra,
                                      /*src_palette=*/{}, /*mask_color=*/0,
                                      mode, rgb_byte_order));
          const pdfium::span<const uint8_t> clip_span =
              use_clip ? pdfium::span<const uint8_t>(clip)
                       : pdfium::span<const uint8_t>();
          DataVector<uint8_t> reference = initial_destination;
          {
            ScopedRustDibImplementationForTesting implementation(false);
            compositor.CompositeRgbBitmapLine(
                reference, source, static_cast<int>(kPixelCount), clip_span);
          }
          DataVector<uint8_t> candidate = initial_destination;
          {
            ScopedRustDibImplementationForTesting implementation(true);
            compositor.CompositeRgbBitmapLine(
                candidate, source, static_cast<int>(kPixelCount), clip_span);
          }
          EXPECT_EQ(reference, candidate)
              << "format=" << static_cast<int>(format)
              << " mode=" << mode_value
              << " rgb_byte_order=" << rgb_byte_order
              << " use_clip=" << use_clip;
        }
      }
    }
  }
}

TEST(RustBlendParityTest, BgraToByteRowsMatchCppReference) {
  constexpr size_t kPixelCount = 257;
  DataVector<uint8_t> source(kPixelCount * 4);
  DataVector<uint8_t> initial_destination(kPixelCount);
  DataVector<uint8_t> clip(kPixelCount);
  for (size_t index = 0; index < source.size(); ++index) {
    source[index] = static_cast<uint8_t>((index * 83) % 256);
  }
  for (size_t pixel = 0; pixel < kPixelCount; ++pixel) {
    initial_destination[pixel] = static_cast<uint8_t>((pixel * 37) % 256);
    clip[pixel] = static_cast<uint8_t>((pixel * 61) % 256);
  }

  for (FXDIB_Format format :
       {FXDIB_Format::k8bppRgb, FXDIB_Format::k8bppMask}) {
    for (int mode_value = static_cast<int>(BlendMode::kNormal);
         mode_value <= static_cast<int>(BlendMode::kLast); ++mode_value) {
      const auto mode = static_cast<BlendMode>(mode_value);
      for (bool use_clip : {false, true}) {
        CFX_ScanlineCompositor compositor;
        ASSERT_TRUE(compositor.Init(format, FXDIB_Format::kBgra,
                                    /*src_palette=*/{}, /*mask_color=*/0, mode,
                                    /*bRgbByteOrder=*/false));
        const pdfium::span<const uint8_t> clip_span =
            use_clip ? pdfium::span<const uint8_t>(clip)
                     : pdfium::span<const uint8_t>();
        DataVector<uint8_t> reference = initial_destination;
        {
          ScopedRustDibImplementationForTesting implementation(false);
          compositor.CompositeRgbBitmapLine(
              reference, source, static_cast<int>(kPixelCount), clip_span);
        }
        DataVector<uint8_t> candidate = initial_destination;
        {
          ScopedRustDibImplementationForTesting implementation(true);
          compositor.CompositeRgbBitmapLine(
              candidate, source, static_cast<int>(kPixelCount), clip_span);
        }
        EXPECT_EQ(reference, candidate)
            << "format=" << static_cast<int>(format)
            << " mode=" << mode_value << " use_clip=" << use_clip;
      }
    }
  }
}

TEST(RustBlendParityTest, OpaqueRowsMatchCppReferenceAcrossFormats) {
  constexpr size_t kPixelCount = 257;
  DataVector<uint8_t> clip(kPixelCount);
  for (size_t pixel = 0; pixel < clip.size(); ++pixel) {
    clip[pixel] = static_cast<uint8_t>((pixel * 73) % 256);
  }

  for (FXDIB_Format source_format :
       {FXDIB_Format::kBgr, FXDIB_Format::kBgrx}) {
    const size_t source_components = GetCompsFromFormat(source_format);
    DataVector<uint8_t> source(kPixelCount * source_components);
    for (size_t index = 0; index < source.size(); ++index) {
      source[index] = static_cast<uint8_t>((index * 89) % 256);
    }
    for (FXDIB_Format destination_format :
         {FXDIB_Format::k8bppRgb, FXDIB_Format::k8bppMask,
          FXDIB_Format::kBgr, FXDIB_Format::kBgrx, FXDIB_Format::kBgra}) {
      const size_t destination_components =
          GetCompsFromFormat(destination_format);
      DataVector<uint8_t> initial_destination(kPixelCount *
                                               destination_components);
      for (size_t index = 0; index < initial_destination.size(); ++index) {
        initial_destination[index] = static_cast<uint8_t>((index * 41) % 256);
      }
      for (int mode_value = static_cast<int>(BlendMode::kNormal);
           mode_value <= static_cast<int>(BlendMode::kLast); ++mode_value) {
        const auto mode = static_cast<BlendMode>(mode_value);
        const bool byte_order_allowed =
            destination_format != FXDIB_Format::k8bppRgb &&
            destination_format != FXDIB_Format::k8bppMask;
        for (bool rgb_byte_order : {false, true}) {
          if (rgb_byte_order && !byte_order_allowed) {
            continue;
          }
          for (bool use_clip : {false, true}) {
            CFX_ScanlineCompositor compositor;
            ASSERT_TRUE(compositor.Init(
                destination_format, source_format, /*src_palette=*/{},
                /*mask_color=*/0, mode, rgb_byte_order));
            const pdfium::span<const uint8_t> clip_span =
                use_clip ? pdfium::span<const uint8_t>(clip)
                         : pdfium::span<const uint8_t>();
            DataVector<uint8_t> reference = initial_destination;
            {
              ScopedRustDibImplementationForTesting implementation(false);
              compositor.CompositeRgbBitmapLine(
                  reference, source, static_cast<int>(kPixelCount), clip_span);
            }
            DataVector<uint8_t> candidate = initial_destination;
            {
              ScopedRustDibImplementationForTesting implementation(true);
              compositor.CompositeRgbBitmapLine(
                  candidate, source, static_cast<int>(kPixelCount), clip_span);
            }
            EXPECT_EQ(reference, candidate)
                << "source_format=" << static_cast<int>(source_format)
                << " destination_format="
                << static_cast<int>(destination_format)
                << " mode=" << mode_value
                << " rgb_byte_order=" << rgb_byte_order
                << " use_clip=" << use_clip;
          }
        }
      }
    }
  }
}

TEST(RustBlendParityTest, MaskRowsMatchCppReferenceAcrossFormats) {
  constexpr int kPixelCount = 257;
  constexpr uint32_t kMaskColor = 0xAD6535D3;
  DataVector<uint8_t> clip(kPixelCount);
  for (size_t pixel = 0; pixel < clip.size(); ++pixel) {
    clip[pixel] = static_cast<uint8_t>((pixel * 79) % 256);
  }

  for (FXDIB_Format source_format :
       {FXDIB_Format::k8bppMask, FXDIB_Format::k1bppMask}) {
    const int source_left = source_format == FXDIB_Format::k1bppMask ? 3 : 0;
    const size_t source_size = source_format == FXDIB_Format::k1bppMask
                                   ? (source_left + kPixelCount + 7) / 8
                                   : kPixelCount;
    DataVector<uint8_t> source(source_size);
    for (size_t index = 0; index < source.size(); ++index) {
      source[index] = static_cast<uint8_t>((index * 97 + 13) % 256);
    }
    for (FXDIB_Format destination_format :
         {FXDIB_Format::k8bppRgb, FXDIB_Format::k8bppMask,
          FXDIB_Format::kBgr, FXDIB_Format::kBgrx, FXDIB_Format::kBgra}) {
      const size_t destination_components =
          GetCompsFromFormat(destination_format);
      DataVector<uint8_t> initial_destination(kPixelCount *
                                               destination_components);
      for (size_t index = 0; index < initial_destination.size(); ++index) {
        initial_destination[index] = static_cast<uint8_t>((index * 43) % 256);
      }
      for (int mode_value = static_cast<int>(BlendMode::kNormal);
           mode_value <= static_cast<int>(BlendMode::kLast); ++mode_value) {
        const auto mode = static_cast<BlendMode>(mode_value);
        const bool byte_order_allowed =
            destination_format != FXDIB_Format::k8bppRgb &&
            destination_format != FXDIB_Format::k8bppMask;
        for (bool rgb_byte_order : {false, true}) {
          if (rgb_byte_order && !byte_order_allowed) {
            continue;
          }
          for (bool use_clip : {false, true}) {
            CFX_ScanlineCompositor compositor;
            ASSERT_TRUE(compositor.Init(destination_format, source_format,
                                        /*src_palette=*/{}, kMaskColor, mode,
                                        rgb_byte_order));
            const pdfium::span<const uint8_t> clip_span =
                use_clip ? pdfium::span<const uint8_t>(clip)
                         : pdfium::span<const uint8_t>();
            DataVector<uint8_t> reference = initial_destination;
            DataVector<uint8_t> candidate = initial_destination;
            {
              ScopedRustDibImplementationForTesting implementation(false);
              if (source_format == FXDIB_Format::k1bppMask) {
                compositor.CompositeBitMaskLine(reference, source, source_left,
                                                kPixelCount, clip_span);
              } else {
                compositor.CompositeByteMaskLine(reference, source, kPixelCount,
                                                 clip_span);
              }
            }
            {
              ScopedRustDibImplementationForTesting implementation(true);
              if (source_format == FXDIB_Format::k1bppMask) {
                compositor.CompositeBitMaskLine(candidate, source, source_left,
                                                kPixelCount, clip_span);
              } else {
                compositor.CompositeByteMaskLine(candidate, source, kPixelCount,
                                                 clip_span);
              }
            }
            EXPECT_EQ(reference, candidate)
                << "source_format=" << static_cast<int>(source_format)
                << " destination_format="
                << static_cast<int>(destination_format)
                << " mode=" << mode_value
                << " rgb_byte_order=" << rgb_byte_order
                << " use_clip=" << use_clip;
          }
        }
      }
    }
  }
}

}  // namespace
}  // namespace fxge
