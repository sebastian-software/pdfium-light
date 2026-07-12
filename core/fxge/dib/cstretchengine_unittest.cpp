// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cstretchengine.h"

#include <array>
#include <utility>

#include "core/fpdfapi/page/cpdf_dib.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_number.h"
#include "core/fpdfapi/parser/cpdf_stream.h"
#include "core/fxge/dib/fx_dib.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

// Discovered experimentally
constexpr uint32_t kTooBigSrcLen = 20;
constexpr uint32_t kTooBigDestLen = 32 * 1024 * 1024 + 1;

uint32_t PixelWeightSum(const CStretchEngine::PixelWeight* weights) {
  uint32_t sum = 0;
  for (int i = weights->src_start_; i <= weights->src_end_; ++i) {
    sum += weights->GetWeightForPosition(i);
  }
  return sum;
}

void ExecuteOneStretchTest(int32_t dest_width,
                           int32_t src_width,
                           const FXDIB_ResampleOptions& options) {
  static constexpr uint32_t kExpectedSum = CStretchEngine::kFixedPointOne;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(dest_width, 0, dest_width, src_width, 0,
                                     src_width, options));
  for (int32_t i = 0; i < dest_width; ++i) {
    EXPECT_EQ(kExpectedSum, PixelWeightSum(table.GetPixelWeight(i)))
        << "for { " << src_width << ", " << dest_width << " } at " << i;
  }
}

void ExecuteOneReversedStretchTest(int32_t dest_width,
                                   int32_t src_width,
                                   const FXDIB_ResampleOptions& options) {
  static constexpr uint32_t kExpectedSum = CStretchEngine::kFixedPointOne;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(-dest_width, 0, dest_width, src_width, 0,
                                     src_width, options));
  for (int32_t i = 0; i < dest_width; ++i) {
    EXPECT_EQ(kExpectedSum, PixelWeightSum(table.GetPixelWeight(i)))
        << "for { " << src_width << ", " << dest_width << " } at " << i
        << " (reversed)";
  }
}

void ExecuteStretchTests(const FXDIB_ResampleOptions& options) {
  // Can't test everything, few random values chosen.
  static constexpr int32_t kDestWidths[] = {1, 2, 337, 512, 808, 2550};
  static constexpr int32_t kSrcWidths[] = {1, 2, 187, 256, 809, 1110};
  for (int32_t src_width : kSrcWidths) {
    for (int32_t dest_width : kDestWidths) {
      ExecuteOneStretchTest(dest_width, src_width, options);
      ExecuteOneReversedStretchTest(dest_width, src_width, options);
    }
  }
}

}  // namespace

TEST(CStretchEngine, OverflowInCtor) {
  FX_RECT clip_rect;
  RetainPtr<CPDF_Dictionary> dict_obj = pdfium::MakeRetain<CPDF_Dictionary>();
  dict_obj->SetNewFor<CPDF_Number>("Width", 71000);
  dict_obj->SetNewFor<CPDF_Number>("Height", 12500);
  RetainPtr<CPDF_Stream> stream =
      pdfium::MakeRetain<CPDF_Stream>(std::move(dict_obj));
  auto dib_source = pdfium::MakeRetain<CPDF_DIB>(nullptr, stream);
  EXPECT_FALSE(dib_source->Load());  // Fail to load due to dimensions.
  CStretchEngine engine(nullptr, FXDIB_Format::k8bppRgb, 500, 500, clip_rect,
                        dib_source, FXDIB_ResampleOptions());
  EXPECT_TRUE(engine.GetResampleOptionsForTest().bInterpolateBilinear);
  EXPECT_FALSE(engine.GetResampleOptionsForTest().bHalftone);
  EXPECT_FALSE(engine.GetResampleOptionsForTest().bNoSmoothing);
  EXPECT_FALSE(engine.GetResampleOptionsForTest().bLossy);
}

TEST(CStretchEngine, WeightRounding) {
  FXDIB_ResampleOptions options;
  ExecuteStretchTests(options);
}

TEST(CStretchEngine, WeightRoundingNoSmoothing) {
  FXDIB_ResampleOptions options;
  options.bNoSmoothing = true;
  ExecuteStretchTests(options);
}

TEST(CStretchEngine, WeightRoundingBilinear) {
  FXDIB_ResampleOptions options;
  options.bInterpolateBilinear = true;
  ExecuteStretchTests(options);
}

TEST(CStretchEngine, WeightRoundingNoSmoothingBilinear) {
  FXDIB_ResampleOptions options;
  options.bNoSmoothing = true;
  options.bInterpolateBilinear = true;
  ExecuteStretchTests(options);
}

TEST(CStretchEngine, ZeroLengthSrc) {
  FXDIB_ResampleOptions options;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(100, 0, 100, 0, 0, 0, options));
}

TEST(CStretchEngine, ZeroLengthSrcNoSmoothing) {
  FXDIB_ResampleOptions options;
  options.bNoSmoothing = true;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(100, 0, 100, 0, 0, 0, options));
}

TEST(CStretchEngine, ZeroLengthSrcBilinear) {
  FXDIB_ResampleOptions options;
  options.bInterpolateBilinear = true;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(100, 0, 100, 0, 0, 0, options));
}

TEST(CStretchEngine, ZeroLengthSrcNoSmoothingBilinear) {
  FXDIB_ResampleOptions options;
  options.bNoSmoothing = true;
  options.bInterpolateBilinear = true;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(100, 0, 100, 0, 0, 0, options));
}

TEST(CStretchEngine, ZeroLengthDest) {
  FXDIB_ResampleOptions options;
  CStretchEngine::WeightTable table;
  ASSERT_TRUE(table.CalculateWeights(0, 0, 0, 100, 0, 100, options));
}

TEST(CStretchEngine, TooManyWeights) {
  FXDIB_ResampleOptions options;
  CStretchEngine::WeightTable table;
  ASSERT_FALSE(table.CalculateWeights(kTooBigDestLen, 0, kTooBigDestLen,
                                      kTooBigSrcLen, 0, kTooBigSrcLen,
                                      options));
}

TEST(CStretchEngine, RustWeightTableMatchesCppReferenceExactly) {
  struct WeightCase {
    int destination_length;
    int destination_minimum;
    int destination_maximum;
    int source_length;
    int source_minimum;
    int source_maximum;
  };
  static constexpr std::array<WeightCase, 10> kCases = {
      WeightCase{1, 0, 1, 1, 0, 1},
      WeightCase{2, 0, 2, 7, 0, 7},
      WeightCase{5, 0, 5, 2, 0, 2},
      WeightCase{13, 2, 11, 19, 3, 17},
      WeightCase{-13, 0, 13, 19, 0, 19},
      WeightCase{-7, 1, 6, 23, 2, 21},
      WeightCase{100, 0, 100, 0, 0, 0},
      WeightCase{0, 0, 0, 100, 0, 100},
      WeightCase{5, 4, 3, 7, 0, 7},
      WeightCase{17, 0, 17, 3, 1, 2},
  };
  for (const bool no_smoothing : {false, true}) {
    for (const bool bilinear : {false, true}) {
      FXDIB_ResampleOptions options;
      options.bNoSmoothing = no_smoothing;
      options.bInterpolateBilinear = bilinear;
      for (const auto& test_case : kCases) {
        CStretchEngine::WeightTable reference;
        bool reference_result;
        {
          fxge::ScopedRustDibImplementationForTesting implementation(false);
          reference_result = reference.CalculateWeights(
              test_case.destination_length, test_case.destination_minimum,
              test_case.destination_maximum, test_case.source_length,
              test_case.source_minimum, test_case.source_maximum, options);
        }
        CStretchEngine::WeightTable candidate;
        const bool candidate_result = candidate.CalculateWeights(
            test_case.destination_length, test_case.destination_minimum,
            test_case.destination_maximum, test_case.source_length,
            test_case.source_minimum, test_case.source_maximum, options);
        ASSERT_EQ(reference_result, candidate_result)
            << "destination_length=" << test_case.destination_length
            << " source_length=" << test_case.source_length
            << " no_smoothing=" << no_smoothing
            << " bilinear=" << bilinear;
        if (!reference_result || test_case.destination_length == 0) {
          continue;
        }
        for (int pixel = test_case.destination_minimum;
             pixel < test_case.destination_maximum; ++pixel) {
          const auto* reference_weight = reference.GetPixelWeight(pixel);
          const auto* candidate_weight = candidate.GetPixelWeight(pixel);
          ASSERT_EQ(reference_weight->src_start_, candidate_weight->src_start_)
              << "pixel=" << pixel;
          ASSERT_EQ(reference_weight->src_end_, candidate_weight->src_end_)
              << "pixel=" << pixel;
          for (int position = reference_weight->src_start_;
               position <= reference_weight->src_end_; ++position) {
            EXPECT_EQ(reference_weight->GetWeightForPosition(position),
                      candidate_weight->GetWeightForPosition(position))
                << "pixel=" << pixel << " position=" << position
                << " destination_length=" << test_case.destination_length
                << " source_length=" << test_case.source_length
                << " no_smoothing=" << no_smoothing
                << " bilinear=" << bilinear;
          }
        }
      }
    }
  }
}
