// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <stdint.h>

#include <string>
#include <string_view>

#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/fx_safe_types.h"
#include "core/fxcrt/span.h"
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
  {
    ScopedRenderImplementationForTesting implementation(
        RenderImplementationForTesting::kCppReference);
    reference = RenderLoadedPageWithFlags(page.get(), test_case.flags);
  }

  ScopedFPDFBitmap candidate;
  {
    ScopedRenderImplementationForTesting implementation(
        RenderImplementationForTesting::kCandidate);
    candidate = RenderLoadedPageWithFlags(page.get(), test_case.flags);
  }

  ExpectExactBitmapParity(reference.get(), candidate.get());
}

INSTANTIATE_TEST_SUITE_P(
    RustMigrationCorpus,
    RustRendererParityEmbedderTest,
    testing::ValuesIn(kRenderCorpus),
    [](const testing::TestParamInfo<RenderCorpusCase>& info) {
      return std::string(info.param.name);
    });

}  // namespace
