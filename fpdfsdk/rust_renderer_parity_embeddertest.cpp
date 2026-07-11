// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <stdint.h>

#include <string>
#include <string_view>
#include <vector>

#include "core/fpdfapi/render/rust/rust_render_adapter.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/fx_safe_types.h"
#include "core/fxcrt/span.h"
#include "core/fxge/agg/rust/rust_agg_adapter.h"
#include "fpdfsdk/cpdfsdk_renderpage.h"
#include "public/fpdfview.h"
#include "testing/embedder_test.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

struct RenderCorpusCase {
  std::string_view name;
  std::string_view fixture;
  int page_index;
  int flags;
};

constexpr RenderCorpusCase kRenderCorpus[] = {
#define RUST_RENDER_CASE(name, fixture, page_index, flags) \
  {#name, fixture, page_index, flags},
#include "testing/resources/rust_renderer_corpus.inc"
#undef RUST_RENDER_CASE
};

pdfium::span<const uint8_t> BitmapBytes(FPDF_BITMAP bitmap) {
  FX_SAFE_SIZE_T byte_count = FPDFBitmap_GetStride(bitmap);
  byte_count *= FPDFBitmap_GetHeight(bitmap);
  return UNSAFE_BUFFERS(pdfium::span(
      static_cast<const uint8_t*>(FPDFBitmap_GetBuffer(bitmap)),
      byte_count.ValueOrDie()));
}

void ExpectExactBitmapParity(FPDF_BITMAP reference, FPDF_BITMAP candidate) {
  ASSERT_TRUE(reference);
  ASSERT_TRUE(candidate);
  ASSERT_EQ(FPDFBitmap_GetWidth(reference), FPDFBitmap_GetWidth(candidate));
  ASSERT_EQ(FPDFBitmap_GetHeight(reference), FPDFBitmap_GetHeight(candidate));
  ASSERT_EQ(FPDFBitmap_GetFormat(reference), FPDFBitmap_GetFormat(candidate));
  ASSERT_EQ(FPDFBitmap_GetStride(reference), FPDFBitmap_GetStride(candidate));
  EXPECT_EQ(BitmapBytes(reference), BitmapBytes(candidate));
}

class RustRendererParityEmbedderTest
    : public EmbedderTest,
      public testing::WithParamInterface<RenderCorpusCase> {};

TEST_P(RustRendererParityEmbedderTest, CandidateMatchesCppReferenceExactly) {
  const RenderCorpusCase& test_case = GetParam();
  ASSERT_TRUE(OpenDocument(std::string(test_case.fixture)));
  ScopedPage page = LoadScopedPage(test_case.page_index);
  ASSERT_TRUE(page);

  ScopedFPDFBitmap reference;
  std::vector<uint8_t> reference_trace;
  std::vector<uint8_t> reference_agg_trace;
  {
    ScopedRenderImplementationForTesting implementation(
        RenderImplementationForTesting::kCppReference);
    pdfium::rust::ScopedRenderTraceForTesting trace(&reference_trace);
    fxge::ScopedAggTraceForTesting agg_trace(&reference_agg_trace);
    reference = RenderLoadedPageWithFlags(page.get(), test_case.flags);
  }

  ScopedFPDFBitmap candidate;
  std::vector<uint8_t> candidate_trace;
  std::vector<uint8_t> candidate_agg_trace;
  {
    ScopedRenderImplementationForTesting implementation(
        RenderImplementationForTesting::kCandidate);
    pdfium::rust::ScopedRenderTraceForTesting trace(&candidate_trace);
    fxge::ScopedAggTraceForTesting agg_trace(&candidate_agg_trace);
    candidate = RenderLoadedPageWithFlags(page.get(), test_case.flags);
  }

  ExpectExactBitmapParity(reference.get(), candidate.get());
  ASSERT_FALSE(reference_trace.empty());
  EXPECT_EQ(reference_trace, candidate_trace);
  EXPECT_EQ(reference_agg_trace, candidate_agg_trace);
  if (test_case.name == "ManyRectangles") {
    EXPECT_FALSE(reference_agg_trace.empty());
  }
  if (test_case.name == "DashedLines") {
    EXPECT_TRUE(fxge::AggTraceHasDashValuesForTesting(reference_agg_trace));
  }
  if (test_case.name == "Rectangles") {
    EXPECT_TRUE(fxge::AggTraceHasPathCommandsForTesting(reference_agg_trace));
    EXPECT_TRUE(fxge::AggTraceHasPathDrawPlansForTesting(reference_agg_trace));
  }
}

INSTANTIATE_TEST_SUITE_P(
    RustMigrationCorpus,
    RustRendererParityEmbedderTest,
    testing::ValuesIn(kRenderCorpus),
    [](const testing::TestParamInfo<RenderCorpusCase>& info) {
      return std::string(info.param.name);
    });

}  // namespace
