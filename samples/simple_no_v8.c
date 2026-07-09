// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Minimal pdfium-light example from C: create a static PDF page, add a page
// object, render the page once, and save the document. pdfium-light does not
// create a form-fill environment and does not execute JavaScript.

#include <stdio.h>
#include <string.h>

#include "public/fpdf_edit.h"
#include "public/fpdf_save.h"
#include "public/fpdfview.h"

typedef struct FileWriter {
  FPDF_FILEWRITE base;
  FILE* file;
} FileWriter;

static int WriteBlock(FPDF_FILEWRITE* self,
                      const void* data,
                      unsigned long size) {
  FileWriter* writer = (FileWriter*)self;
  return fwrite(data, 1, size, writer->file) == size;
}

int main(int argc, const char* argv[]) {
  const char* output_path = argc > 1 ? argv[1] : "pdfium-light-sample.pdf";

  FPDF_LIBRARY_CONFIG config;
  memset(&config, 0, sizeof(config));
  config.version = 3;
  FPDF_InitLibraryWithConfig(&config);

  FPDF_DOCUMENT doc = FPDF_CreateNewDocument();
  if (!doc) {
    FPDF_DestroyLibrary();
    return 1;
  }

  FPDF_PAGE page = FPDFPage_New(doc, 0, 640.0, 480.0);
  if (!page) {
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return 1;
  }

  FPDF_PAGEOBJECT rect = FPDFPageObj_CreateNewRect(72.0f, 72.0f, 240.0f, 120.0f);
  if (!rect || !FPDFPageObj_SetFillColor(rect, 66, 133, 244, 255) ||
      !FPDFPath_SetDrawMode(rect, FPDF_FILLMODE_ALTERNATE, 0)) {
    FPDF_ClosePage(page);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return 1;
  }
  FPDFPage_InsertObject(page, rect);

  if (!FPDFPage_GenerateContent(page)) {
    FPDF_ClosePage(page);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return 1;
  }

  FPDF_BITMAP bitmap = FPDFBitmap_Create(640, 480, 0);
  if (bitmap) {
    FPDFBitmap_FillRect(bitmap, 0, 0, 640, 480, 0xFFFFFFFF);
    FPDF_RenderPageBitmap(bitmap, page, 0, 0, 640, 480, 0, 0);
    FPDFBitmap_Destroy(bitmap);
  }

  FILE* file = fopen(output_path, "wb");
  if (!file) {
    FPDF_ClosePage(page);
    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return 1;
  }

  FileWriter writer;
  memset(&writer, 0, sizeof(writer));
  writer.base.version = 1;
  writer.base.WriteBlock = WriteBlock;
  writer.file = file;

  int ok = FPDF_SaveAsCopy(doc, &writer.base, 0);
  fclose(file);

  FPDF_ClosePage(page);
  FPDF_CloseDocument(doc);
  FPDF_DestroyLibrary();

  return ok ? 0 : 1;
}
