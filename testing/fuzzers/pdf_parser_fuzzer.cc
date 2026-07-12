// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stdint.h>

#include "core/fpdfapi/parser/cpdf_simple_parser.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/bytestring.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/span.h"
#include "public/fpdfview.h"

namespace {

struct DocumentSnapshot {
  int page_count;
  int file_version;
  bool has_file_version;
  unsigned long permissions;
  int security_revision;
};

DocumentSnapshot SnapshotDocument(FPDF_DOCUMENT document,
                                  bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  int file_version = 0;
  const bool has_file_version =
      FPDF_GetFileVersion(document, &file_version) != 0;
  return {
      .page_count = FPDF_GetPageCount(document),
      .file_version = file_version,
      .has_file_version = has_file_version,
      .permissions = FPDF_GetDocPermissions(document),
      .security_revision = FPDF_GetSecurityHandlerRevision(document),
  };
}

void CompareBoundaryPage(FPDF_DOCUMENT reference,
                         FPDF_DOCUMENT candidate,
                         int page_index) {
  FPDF_PAGE reference_page = nullptr;
  {
    pdfium::rust::ScopedRustParserImplementationForTesting implementation(
        false);
    reference_page = FPDF_LoadPage(reference, page_index);
  }
  FPDF_PAGE candidate_page = nullptr;
  {
    pdfium::rust::ScopedRustParserImplementationForTesting implementation(true);
    candidate_page = FPDF_LoadPage(candidate, page_index);
  }
  CHECK_EQ(!!reference_page, !!candidate_page);
  if (reference_page) {
    CHECK_EQ(FPDF_GetPageWidthF(reference_page),
             FPDF_GetPageWidthF(candidate_page));
    CHECK_EQ(FPDF_GetPageHeightF(reference_page),
             FPDF_GetPageHeightF(candidate_page));
  }
  FPDF_ClosePage(reference_page);
  FPDF_ClosePage(candidate_page);
}

}  // namespace

extern "C" int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {
  static constexpr size_t kMaxDataSize = 1024 * 1024;
  if (size > kMaxDataSize) {
    return 0;
  }

  // SAFETY: The fuzzer contract provides `size` readable bytes at `data`.
  const auto input = UNSAFE_BUFFERS(pdfium::span(data, size));
  CPDF_SimpleParser reference(input);
  CPDF_SimpleParser candidate(input);
  while (true) {
    ByteStringView reference_word;
    {
      pdfium::rust::ScopedRustParserImplementationForTesting implementation(
          false);
      reference_word = reference.GetWord();
    }
    ByteStringView candidate_word;
    {
      pdfium::rust::ScopedRustParserImplementationForTesting implementation(
          true);
      candidate_word = candidate.GetWord();
    }
    CHECK_EQ(reference_word, candidate_word);
    CHECK_EQ(reference.GetCurrentPosition(), candidate.GetCurrentPosition());
    if (reference_word.IsEmpty()) {
      break;
    }
  }

  FPDF_DOCUMENT reference_document = nullptr;
  unsigned long reference_error = 0;
  {
    pdfium::rust::ScopedRustParserImplementationForTesting implementation(
        false);
    reference_document = FPDF_LoadMemDocument64(data, size, nullptr);
    reference_error = FPDF_GetLastError();
  }
  FPDF_DOCUMENT candidate_document = nullptr;
  unsigned long candidate_error = 0;
  {
    pdfium::rust::ScopedRustParserImplementationForTesting implementation(true);
    candidate_document = FPDF_LoadMemDocument64(data, size, nullptr);
    candidate_error = FPDF_GetLastError();
  }
  CHECK_EQ(!!reference_document, !!candidate_document);
  CHECK_EQ(reference_error, candidate_error);
  if (reference_document) {
    const DocumentSnapshot reference_snapshot =
        SnapshotDocument(reference_document, false);
    const DocumentSnapshot candidate_snapshot =
        SnapshotDocument(candidate_document, true);
    CHECK_EQ(reference_snapshot.page_count, candidate_snapshot.page_count);
    CHECK_EQ(reference_snapshot.file_version, candidate_snapshot.file_version);
    CHECK_EQ(reference_snapshot.has_file_version,
             candidate_snapshot.has_file_version);
    CHECK_EQ(reference_snapshot.permissions, candidate_snapshot.permissions);
    CHECK_EQ(reference_snapshot.security_revision,
             candidate_snapshot.security_revision);
    if (reference_snapshot.page_count > 0) {
      CompareBoundaryPage(reference_document, candidate_document, 0);
      if (reference_snapshot.page_count > 1) {
        CompareBoundaryPage(reference_document, candidate_document,
                            reference_snapshot.page_count - 1);
      }
    }
  }
  FPDF_CloseDocument(reference_document);
  FPDF_CloseDocument(candidate_document);
  return 0;
}
