// Copyright 2015 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TESTING_EMBEDDER_TEST_H_
#define TESTING_EMBEDDER_TEST_H_

#include <stdint.h>

#include <fstream>
#include <map>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

#include "build/build_config.h"
#include "core/fxcrt/bytestring.h"
#include "core/fxcrt/span.h"
#include "core/fxcrt/unowned_ptr.h"
#include "public/fpdf_ext.h"
#include "public/fpdf_save.h"
#include "public/fpdfview.h"
#include "testing/gtest/include/gtest/gtest.h"

class TestLoader;

// The loading time of the CFGAS_FontMgr is linear in the number of times it is
// loaded. So, if a test suite has a lot of tests that need a font manager they
// can end up executing very, very slowly.

// Helper macro for common equality assertions with a fixed tolerance of 0.001.
#define EXPECT_NEAR_THREE_PLACES(a, b) EXPECT_NEAR((a), (b), 0.001)

// This class is used to load a PDF document, and then run programatic
// API tests against it.
class EmbedderTest : public ::testing::Test,
                     public UNSUPPORT_INFO,
                     public FPDF_FILEWRITE {
 public:
  enum class LinearizeOption { kDefaultLinearize, kMustLinearize };

  class Delegate {
   public:
    virtual ~Delegate() = default;

    // Equivalent to UNSUPPORT_INFO::FSDK_UnSupport_Handler().
    virtual void UnsupportedHandler(int type) {}

  };

  class ScopedSavedDoc {
   public:
    ScopedSavedDoc();
    explicit ScopedSavedDoc(EmbedderTest* test);
    ScopedSavedDoc(EmbedderTest* test, const ByteString& password);
    ScopedSavedDoc(const ScopedSavedDoc&) = delete;
    ScopedSavedDoc& operator=(const ScopedSavedDoc&) = delete;
    ScopedSavedDoc(ScopedSavedDoc&&) noexcept;
    ScopedSavedDoc& operator=(ScopedSavedDoc&&) noexcept;
    ~ScopedSavedDoc();

    FPDF_DOCUMENT get() { return doc_; }

    explicit operator bool() const { return !!doc_; }

   private:
    UnownedPtr<EmbedderTest> test_;
    FPDF_DOCUMENT doc_;
  };

  class ScopedPageBase {
   public:
    FPDF_PAGE get() { return page_; }

    explicit operator bool() const { return !!page_; }

   protected:
    ScopedPageBase(EmbedderTest* test, FPDF_PAGE page);
    ~ScopedPageBase();

    UnownedPtr<EmbedderTest> test_;
    FPDF_PAGE page_;
  };

  class ScopedPage : public ScopedPageBase {
   public:
    ScopedPage();
    ScopedPage(EmbedderTest* test, int page_index);
    ScopedPage(const ScopedPage&) = delete;
    ScopedPage& operator=(const ScopedPage&) = delete;
    ScopedPage(ScopedPage&&) noexcept;
    ScopedPage& operator=(ScopedPage&&) noexcept;
    ~ScopedPage();
  };

  class ScopedSavedPage : public ScopedPageBase {
   public:
    ScopedSavedPage();
    ScopedSavedPage(EmbedderTest* test, int page_index);
    ScopedSavedPage(const ScopedSavedPage&) = delete;
    ScopedSavedPage& operator=(const ScopedSavedPage&) = delete;
    ScopedSavedPage(ScopedSavedPage&&) noexcept;
    ScopedSavedPage& operator=(ScopedSavedPage&&) noexcept;
    ~ScopedSavedPage();
  };

  EmbedderTest();
  ~EmbedderTest() override;

  void SetUp() override;
  void TearDown() override;

  Delegate* GetDelegate() { return delegate_; }
  void SetDelegate(Delegate* delegate) {
    delegate_ = delegate ? delegate : default_delegate_.get();
  }


  FPDF_DOCUMENT document() const { return document_.get(); }
  FPDF_DOCUMENT saved_document() const { return saved_document_.get(); }

  // Create an empty document.
  void CreateEmptyDocument();

  // Alias retained for tests that predate the removal of the form-fill runtime.
  void CreateEmptyDocumentWithoutFormFillEnvironment();

  // Open the document specified by `filename`, or return false on failure.
  // The `filename` is relative to the test data directory where we store all
  // the test files. `password` can be an empty string if the file is not
  // password protected.
  virtual bool OpenDocumentWithOptions(const std::string& filename,
                                       const ByteString& password,
                                       LinearizeOption linearize_option);

  // Variants provided for convenience.
  bool OpenDocument(const std::string& filename);
  bool OpenDocumentLinearized(const std::string& filename);
  bool OpenDocumentWithPassword(const std::string& filename,
                                const ByteString& password);

  // Close the document from a previous OpenDocument() call. This happens
  // automatically at tear-down, and is usually not explicitly required,
  // unless testing multiple documents or duplicate destruction.
  void CloseDocument();

  // Retained as a no-op for tests that predate JavaScript removal.
  void DoOpenActions();

  // Determine the page numbers present in the document.
  int GetFirstPageNum();
  int GetPageCount();

  // Load a specific page of the open document with a given non-negative
  // `page_index`. On success, return a ScopedPage with the page handle. On failure, return an empty ScopedPage.
  // The caller needs to let the ScopedPage go out of scope to properly unload
  // the page, and must do so before the page's document and `this` get
  // destroyed.
  // The caller cannot call this for a `page_index` if it already obtained and
  // holds the page handle for that page.
  ScopedPage LoadScopedPage(int page_index);

  // Prefer LoadScopedPage() above.
  //
  // Load a specific page of the open document with a given non-negative
  // `page_index`. On success, return a page handle. On failure, return nullptr.
  // The caller does not own the returned page handle, but must call
  // UnloadPage() on it when done.
  // The caller cannot call this for a `page_index` if it already obtained and
  // holds the page handle for that page.
  FPDF_PAGE LoadPage(int page_index);

  // Alias retained after form events were removed.
  FPDF_PAGE LoadPageNoEvents(int page_index);

  // Release the resources for a `page` obtained from LoadPage(). Further use of `page` is prohibited after calling this.
  void UnloadPage(FPDF_PAGE page);

  // Alias retained after form events were removed.
  void UnloadPageNoEvents(FPDF_PAGE page);

  // RenderLoadedPageWithFlags() with no flags.
  ScopedFPDFBitmap RenderLoadedPage(FPDF_PAGE page);

  // Convert `page` loaded via LoadPage() into a bitmap with the specified page
  // rendering `flags`.
  //
  // See public/fpdfview.h for a list of page rendering flags.
  ScopedFPDFBitmap RenderLoadedPageWithFlags(FPDF_PAGE page, int flags);

  // RenderSavedPageWithFlags() with no flags.
  ScopedFPDFBitmap RenderSavedPage(FPDF_PAGE page);

  // Convert `page` loaded via LoadSavedPage() into a bitmap with the specified
  // page rendering `flags`.
  //
  // See public/fpdfview.h for a list of page rendering flags.
  ScopedFPDFBitmap RenderSavedPageWithFlags(FPDF_PAGE page, int flags);

  // Convert `page` into a bitmap with the specified page rendering `flags`.
  static ScopedFPDFBitmap RenderPageWithFlags(FPDF_PAGE page, int flags);

  // Simplified form of RenderPageWithFlags() with no handle and no flags.
  static ScopedFPDFBitmap RenderPage(FPDF_PAGE page);

#if BUILDFLAG(IS_WIN)
  // Convert `page` into EMF with the specified page rendering `flags`.
  static std::vector<uint8_t> RenderPageWithFlagsToEmf(FPDF_PAGE page,
                                                       int flags);

  // Get the PostScript data from `emf_data`.
  static std::string GetPostScriptFromEmf(pdfium::span<const uint8_t> emf_data);
#endif  // BUILDFLAG(IS_WIN)

  // Return bytes for each of the FPDFBitmap_* format types.
  static int BytesPerPixelForFormat(int format);

 protected:
  using PageNumberToHandleMap = std::map<int, FPDF_PAGE>;

  // Return the hash of only the pixels in `bitmap`. i.e. Excluding the gap, if
  // any, at the end of a row where the stride is larger than width * bpp.
  static std::string HashBitmap(FPDF_BITMAP bitmap);

  // For debugging purposes.
  // Write `bitmap` as a PNG to `filename`.
  static void WriteBitmapToPng(FPDF_BITMAP bitmap, const std::string& filename);

  // Check `bitmap` matches `expectation_png_name`, where `expectation_png_name`
  // is the name of testing/resources/embedder_tests/expectation_png_name.png.
  static void CompareBitmap(FPDF_BITMAP bitmap,
                            std::string_view expectation_png_name);

  // Like CompareBitmap(), except instead of just adding ".png" to
  // `expectation_png_name`, this method will look for the expectation PNG using
  // several suffixes in order: "_agg_$os.png", "_agg.png", "_$os.png".
  //
  // For example, with "hello_world" on macOS, the list of
  // expectation PNGs are:
  // - hello_world_agg_mac.png
  // - hello_world_agg.png
  // - hello_world_mac.png
  // - hello_world.png
  //
  // This is similar to the behavior in testing/tools/pngdiffer.py.
  //
  // `max_pixel_per_channel_delta` can optionally be set to tolerate minor pixel
  // discrepancies. The default is exact matching.
  static void CompareBitmapWithExpectationSuffix(
      FPDF_BITMAP bitmap,
      std::string_view expectation_png_name,
      int max_pixel_per_channel_delta = 0);

  // Same as `CompareBitmapWithExpectationSuffix()`, but automatically
  // applies platform-specific tolerance.
  static void CompareBitmapWithFuzzyExpectationSuffix(
      FPDF_BITMAP bitmap,
      std::string_view expectation_png_name);

  void ClearString() { data_string_.clear(); }
  const std::string& GetString() const { return data_string_; }

  static int GetBlockFromString(void* param,
                                unsigned long pos,
                                unsigned char* buf,
                                unsigned long size);

  // See comments in the respective non-Saved versions of these methods.
  ScopedSavedDoc OpenScopedSavedDocument();
  ScopedSavedDoc OpenScopedSavedDocumentWithPassword(
      const ByteString& password);
  ScopedSavedPage LoadScopedSavedPage(int page_index);
  FPDF_PAGE LoadSavedPage(int page_index);
  void CloseSavedPage(FPDF_PAGE page);

  // See comments for CompareBitmap() and CompareBitmapWithExpectationSuffix()
  // above.
  void VerifySavedRendering(FPDF_PAGE page,
                            std::string_view expectation_png_name);
  void VerifySavedRenderingWithExpectationSuffix(
      FPDF_PAGE page,
      std::string_view expectation_png_name);
  void VerifySavedRenderingWithFuzzyExpectationSuffix(
      FPDF_PAGE page,
      std::string_view expectation_png_name);
  void VerifySavedDocument(std::string_view expectation_png_name);
  void VerifySavedDocumentWithExpectationSuffix(
      std::string_view expectation_png_name);
  void VerifySavedDocumentWithFuzzyExpectationSuffix(
      std::string_view expectation_png_name);

#ifndef NDEBUG
  // For debugging purposes.
  // While open, write any data that gets passed to WriteBlockCallback() to
  // `filename`. This is typically used to capture data from FPDF_SaveAsCopy()
  // calls.
  void OpenPDFFileForWrite(const std::string& filename);
  void ClosePDFFileForWrite();
#endif

 private:
  static int WriteBlockCallback(FPDF_FILEWRITE* pFileWrite,
                                const void* data,
                                unsigned long size);

  // Helper method for the methods below.
  static int GetPageNumberForPage(const PageNumberToHandleMap& page_map,
                                  FPDF_PAGE page);
  // Find `page` inside `page_map_` and return the associated page number, or -1
  // if `page` cannot be found.
  int GetPageNumberForLoadedPage(FPDF_PAGE page) const;

  // Same as GetPageNumberForLoadedPage(), but with `saved_page_map_`.
  int GetPageNumberForSavedPage(FPDF_PAGE page) const;

  // Helpers for opening saved documents. These are intended for internal use
  // only. Callers should use the Scoped methods that manage lifetime
  // automatically.
  FPDF_DOCUMENT OpenSavedDocument();
  FPDF_DOCUMENT OpenSavedDocumentWithPassword(const ByteString& password);

  // Closes a document opened via OpenSavedDocument(). This must only be invoked
  // in documents opened by the helpers above. This is intended for internal use
  // in the destructor of the scoped methods.
  void CloseSavedDocument();

  void UnloadPageCommon(FPDF_PAGE page);
  FPDF_PAGE LoadPageCommon(int page_index);

  ScopedFPDFBitmap VerifySavedRenderingCommon(FPDF_PAGE page);
  ScopedFPDFBitmap VerifySavedDocumentCommon();

  std::unique_ptr<Delegate> default_delegate_;
  Delegate* delegate_;


  // must outlive `loader_`.
  std::vector<uint8_t> file_contents_;
  std::unique_ptr<TestLoader> loader_;
  ScopedFPDFDocument document_;
  PageNumberToHandleMap page_map_;

  ScopedFPDFDocument saved_document_;
  PageNumberToHandleMap saved_page_map_;

  std::string data_string_;
  std::string saved_document_file_data_;
  std::ofstream filestream_;
};

#endif  // TESTING_EMBEDDER_TEST_H_
