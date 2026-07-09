// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// This target deliberately compiles the headers that a pdfium-light consumer
// receives, without FPDF_IMPLEMENTATION. Do not add interactive headers here.

#include "public/cpp/fpdf_deleters.h"
#include "public/cpp/fpdf_scopers.h"
#include "public/fpdf_annot.h"
#include "public/fpdf_attachment.h"
#include "public/fpdf_catalog.h"
#include "public/fpdf_dataavail.h"
#include "public/fpdf_doc.h"
#include "public/fpdf_edit.h"
#include "public/fpdf_ext.h"
#include "public/fpdf_flatten.h"
#include "public/fpdf_ppo.h"
#include "public/fpdf_progressive.h"
#include "public/fpdf_save.h"
#include "public/fpdf_searchex.h"
#include "public/fpdf_signature.h"
#include "public/fpdf_structtree.h"
#include "public/fpdf_sysfontinfo.h"
#include "public/fpdf_text.h"
#include "public/fpdf_thumbnail.h"
#include "public/fpdf_transformpage.h"
#include "public/fpdfview.h"

void CompileStaticAnnotationSurface(FPDF_PAGE page,
                                    FPDF_ANNOTATION annotation,
                                    FS_RECTF* rect,
                                    unsigned int* color_component) {
  FPDF_ANNOTATION created = FPDFPage_CreateAnnot(page, FPDF_ANNOT_HIGHLIGHT);
  FPDFAnnot_SetColor(annotation, FPDFANNOT_COLORTYPE_Color,
                     /*R=*/0, /*G=*/0, /*B=*/0, /*A=*/255);
  FPDFAnnot_GetColor(annotation, FPDFANNOT_COLORTYPE_Color, color_component,
                     color_component, color_component, color_component);
  FPDFAnnot_SetRect(annotation, rect);
  FPDFAnnot_GetRect(annotation, rect);
  FPDFAnnot_IsSupportedSubtype(FPDF_ANNOT_HIGHLIGHT);
  FPDFAnnot_IsSupportedSubtype(FPDF_ANNOT_FREETEXT);
  FPDFAnnot_IsSupportedSubtype(FPDF_ANNOT_INK);
  FPDFAnnot_IsSupportedSubtype(FPDF_ANNOT_WIDGET);
  FPDFPage_CloseAnnot(created);
}

void CompileRedactionSurface(FPDF_PAGE page, const FS_RECTF* rects) {
  int result = FPDFPage_ApplyRedactions(page, rects, 1);
  (void)result;
}

int main() {
  return 0;
}
