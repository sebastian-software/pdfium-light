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

int main() {
  return 0;
}
