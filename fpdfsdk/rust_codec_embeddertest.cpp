// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <vector>

#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_document.h"
#include "core/fpdfapi/parser/cpdf_stream.h"
#include "core/fpdfapi/parser/cpdf_stream_acc.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/data_vector.h"
#include "fpdfsdk/cpdfsdk_helpers.h"
#include "public/cpp/fpdf_scopers.h"
#include "public/fpdf_save.h"
#include "public/fpdf_text.h"
#include "testing/embedder_test.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class RustCodecEmbedderTest : public EmbedderTest {};

struct ParserObjectGraphSnapshot {
  std::vector<ByteString> catalog_keys;
  std::vector<ByteString> page_keys;
  ByteString catalog_type;
  ByteString page_type;
  CFX_FloatRect media_box;
  DataVector<uint8_t> filtered_content;

  bool operator==(const ParserObjectGraphSnapshot&) const = default;
};

void AppendFilteredContent(const CPDF_Object* object,
                           DataVector<uint8_t>* output) {
  if (!object) {
    return;
  }
  RetainPtr<const CPDF_Object> direct = object->GetDirect();
  if (!direct) {
    return;
  }
  if (const CPDF_Stream* stream = direct->AsStream()) {
    auto access =
        pdfium::MakeRetain<CPDF_StreamAcc>(pdfium::WrapRetain(stream));
    access->LoadAllDataFiltered();
    output->insert(output->end(), access->GetSpan().begin(),
                   access->GetSpan().end());
    return;
  }
  if (const CPDF_Array* array = direct->AsArray()) {
    for (size_t index = 0; index < array->size(); ++index) {
      AppendFilteredContent(array->GetObjectAt(index).Get(), output);
    }
  }
}

ParserObjectGraphSnapshot CaptureObjectGraph(FPDF_DOCUMENT document) {
  CPDF_Document* pdf_document = CPDFDocumentFromFPDFDocument(document);
  CHECK(pdf_document);
  const CPDF_Dictionary* catalog = pdf_document->GetRoot();
  CHECK(catalog);
  RetainPtr<const CPDF_Dictionary> page = pdf_document->GetPageDictionary(0);
  CHECK(page);
  ParserObjectGraphSnapshot snapshot{
      .catalog_keys = catalog->GetKeys(),
      .page_keys = page->GetKeys(),
      .catalog_type = catalog->GetNameFor("Type"),
      .page_type = page->GetNameFor("Type"),
      .media_box = page->GetRectFor("MediaBox"),
  };
  AppendFilteredContent(page->GetObjectFor("Contents").Get(),
                        &snapshot.filtered_content);
  return snapshot;
}

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

TEST_F(RustCodecEmbedderTest, ParserObjectGraphSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("hello_world.pdf"));
  ScopedPage page = LoadScopedPage(0);
  ASSERT_TRUE(page);
  ExpectRustCodecPage(page.get(), 30);
  const ParserObjectGraphSnapshot original = CaptureObjectGraph(document());
  EXPECT_EQ("Catalog", original.catalog_type);
  EXPECT_EQ("Page", original.page_type);
  EXPECT_FALSE(original.catalog_keys.empty());
  EXPECT_FALSE(original.page_keys.empty());
  EXPECT_FALSE(original.filtered_content.empty());

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ScopedSavedPage saved_page = LoadScopedSavedPage(0);
  ASSERT_TRUE(saved_page);
  ExpectRustCodecPage(saved_page.get(), 30);
  EXPECT_EQ(original, CaptureObjectGraph(saved_document.get()));
}

}  // namespace
