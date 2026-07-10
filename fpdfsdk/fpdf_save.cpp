// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "public/fpdf_save.h"

#include <stdint.h>

#include <optional>
#include <utility>
#include <vector>

#include "build/build_config.h"
#include "constants/catalog.h"
#include "core/fpdfapi/edit/cpdf_creator.h"
#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_document.h"
#include "core/fpdfapi/parser/cpdf_reference.h"
#include "core/fpdfapi/parser/cpdf_stream_acc.h"
#include "core/fpdfapi/parser/cpdf_string.h"
#include "core/fxcrt/fx_extension.h"
#include "core/fxcrt/mask.h"
#include "core/fxcrt/stl_util.h"
#include "fpdfsdk/cpdfsdk_filewriteadapter.h"
#include "fpdfsdk/cpdfsdk_helpers.h"
#include "public/fpdf_edit.h"


static_assert(FPDF_INCREMENTAL == CPDF_Creator::CreateFlags::kIncremental);
static_assert(FPDF_NO_INCREMENTAL == CPDF_Creator::CreateFlags::kNoOriginal);
static_assert(FPDF_REMOVE_SECURITY_DEPRECATED ==
              CPDF_Creator::CreateFlags::kRemoveSecurityDeprecated);
static_assert(FPDF_REMOVE_SECURITY ==
              CPDF_Creator::CreateFlags::kRemoveSecurity);
static_assert(FPDF_SUBSET_NEW_FONTS ==
              CPDF_Creator::CreateFlags::kSubsetNewFonts);

namespace {


bool DoDocSave(FPDF_DOCUMENT document,
               FPDF_FILEWRITE* file_write,
               FPDF_DWORD flags,
               std::optional<int> version) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return false;
  }


  CPDF_Creator file_maker(
      doc, pdfium::MakeRetain<CPDFSDK_FileWriteAdapter>(file_write));
  bool create_result = file_maker.Create(
      Mask<CPDF_Creator::CreateFlags>::FromUnderlyingUnchecked(
          static_cast<uint32_t>(flags)),
      version.value_or(0));


  return create_result;
}

}  // namespace

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_SaveAsCopy(FPDF_DOCUMENT document,
                                                    FPDF_FILEWRITE* file_write,
                                                    FPDF_DWORD flags) {
  return DoDocSave(document, file_write, flags, {});
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_SaveWithVersion(FPDF_DOCUMENT document,
                     FPDF_FILEWRITE* file_write,
                     FPDF_DWORD flags,
                     int fileVersion) {
  return DoDocSave(document, file_write, flags, fileVersion);
}
