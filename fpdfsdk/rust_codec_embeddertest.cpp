// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "public/cpp/fpdf_scopers.h"
#include "public/fpdf_save.h"
#include "public/fpdf_text.h"
#include "testing/embedder_test.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class RustCodecEmbedderTest : public EmbedderTest {};

void ExpectRustCodecPage(FPDF_PAGE page, int expected_char_count) {
  ScopedFPDFBitmap bitmap = EmbedderTest::RenderPage(page);
  ASSERT_TRUE(bitmap);

  ScopedFPDFTextPage text_page(FPDFText_LoadPage(page));
  ASSERT_TRUE(text_page);
  EXPECT_EQ(expected_char_count, FPDFText_CountChars(text_page.get()));
}

TEST_F(RustCodecEmbedderTest, FiltersDecodeRenderAndSurviveSaveReload) {
  ASSERT_TRUE(OpenDocument("rust_codec_filters.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ExpectRustCodecPage(page.get(), 18);

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ExpectRustCodecPage(saved_page.get(), 18);
}

TEST_F(RustCodecEmbedderTest, AsciiHexDecodeRendersAndSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("rust_ascii_hex_filter.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ExpectRustCodecPage(page.get(), 18);

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ExpectRustCodecPage(saved_page.get(), 18);
}

TEST_F(RustCodecEmbedderTest, LzwDecodeRendersAndSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("rust_lzw_filter.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ExpectRustCodecPage(page.get(), 15);

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ExpectRustCodecPage(saved_page.get(), 15);
}

TEST_F(RustCodecEmbedderTest, PngPredictorRendersAndSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("rust_png_predictor_filter.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ExpectRustCodecPage(page.get(), 15);

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ExpectRustCodecPage(saved_page.get(), 15);
}

TEST_F(RustCodecEmbedderTest, FaxImageRendersAndSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("pixel/transfer_function.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ASSERT_TRUE(RenderPage(page.get()));

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ASSERT_TRUE(RenderPage(saved_page.get()));
}

}  // namespace
