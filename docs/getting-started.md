# Getting Started with pdfium-light

[TOC]

pdfium-light is a static PDF library for rendering, text extraction, page object
editing, saving, annotations, document assembly, and conservative redaction. It
does not include embedded JavaScript execution, V8, XFA, AcroForm filling,
widget interaction, or viewer-style event callbacks.

## Prerequisites

Build or obtain a pdfium-light static library and include the public headers from
`public/`. The supported GN configuration keeps the light boundary explicit:

```gn
pdf_enable_light = true
pdf_enable_v8 = false
pdf_enable_xfa = false
is_component_build = false
```

Applications should include only the public headers for the APIs they use.
`fpdfview.h` is needed for library initialization, document loading, page
loading, rendering, and shutdown.

## Initializing the library

Initialize PDFium once before using any other API, and destroy it when the
process no longer needs PDFium.

```c
#include <string.h>
#include "fpdfview.h"

int main(void) {
  FPDF_LIBRARY_CONFIG config;
  memset(&config, 0, sizeof(config));
  config.version = 3;

  FPDF_InitLibraryWithConfig(&config);

  FPDF_DestroyLibrary();
  return 0;
}
```

The V8-related fields in older PDFium examples are intentionally not used by
pdfium-light.

## Loading a document

Use `FPDF_LoadDocument()` for a file path, `FPDF_LoadMemDocument()` for an
in-memory buffer, or `FPDF_LoadCustomDocument()` for a custom loader. Pass
`NULL` as the password when the document is not encrypted.

```c
#include <stdio.h>
#include <string.h>
#include "fpdfview.h"

int main(void) {
  FPDF_LIBRARY_CONFIG config;
  memset(&config, 0, sizeof(config));
  config.version = 3;
  FPDF_InitLibraryWithConfig(&config);

  FPDF_DOCUMENT doc = FPDF_LoadDocument("input.pdf", NULL);
  if (!doc) {
    unsigned long err = FPDF_GetLastError();
    fprintf(stderr, "Failed to load PDF: %lu\n", err);
    FPDF_DestroyLibrary();
    return 1;
  }

  FPDF_CloseDocument(doc);
  FPDF_DestroyLibrary();
  return 0;
}
```

`FPDF_GetLastError()` reports errors such as missing files, invalid PDF syntax,
password failures, unsupported security handlers, or page access failures.

## Rendering a page

Load a page, render it into a bitmap, then close the page. Form handles and
`FPDF_FFLDraw()` are not part of pdfium-light.

```c
FPDF_PAGE page = FPDF_LoadPage(doc, 0);
if (!page) {
  FPDF_CloseDocument(doc);
  FPDF_DestroyLibrary();
  return 1;
}

int width = (int)FPDF_GetPageWidthF(page);
int height = (int)FPDF_GetPageHeightF(page);
FPDF_BITMAP bitmap = FPDFBitmap_Create(width, height, 0);
FPDFBitmap_FillRect(bitmap, 0, 0, width, height, 0xFFFFFFFF);
FPDF_RenderPageBitmap(bitmap, page, 0, 0, width, height, 0, 0);

FPDFBitmap_Destroy(bitmap);
FPDF_ClosePage(page);
```

Use `FPDF_ANNOT` when you want ordinary static annotations rendered with the
page. Interactive widget handling is outside the light scope.

## Creating and saving a document

The edit and save APIs can create a new document, add pages, generate page
content, and serialize the result.

```c
#include <string.h>
#include "fpdf_edit.h"
#include "fpdf_save.h"
#include "fpdfview.h"

int main(void) {
  FPDF_LIBRARY_CONFIG config;
  memset(&config, 0, sizeof(config));
  config.version = 3;
  FPDF_InitLibraryWithConfig(&config);

  FPDF_DOCUMENT doc = FPDF_CreateNewDocument();
  FPDF_PAGE page = FPDFPage_New(doc, 0, 640.0, 480.0);
  FPDFPage_GenerateContent(page);

  FPDF_ClosePage(page);
  FPDF_CloseDocument(doc);
  FPDF_DestroyLibrary();
  return 0;
}
```

See [PDF Editing Guide](/docs/pdfium-edit-guide.md) for more editing examples
and [Light API Boundary](/docs/light-api-boundary.md) for the supported public
surface.
