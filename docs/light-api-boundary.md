# pdfium-light API and build boundary

`pdf_enable_light=true` is the default build mode for this repository. It
defines `PDFIUM_LIGHT` for consumers and makes the exported header manifest
describe the static-document API. Set it to `false` only while comparing a
transition with upstream-compatible PDFium; it is not a supported
pdfium-light release mode.

## Exported headers

The light manifest exports the C and C++ helper headers for document loading,
rendering, text extraction, page-object editing, saving, static inspection,
ordinary annotations, and page transforms. It includes:

- `fpdfview.h`, `fpdf_text.h`, `fpdf_edit.h`, `fpdf_save.h`, and
  `fpdf_transformpage.h`;
- `fpdf_annot.h`, `fpdf_attachment.h`, `fpdf_catalog.h`, `fpdf_doc.h`,
  `fpdf_ext.h`, `fpdf_flatten.h`, `fpdf_ppo.h`, `fpdf_signature.h`, and
  `fpdf_structtree.h` for static document inspection and editing;
- `fpdf_searchex.h`, `fpdf_sysfontinfo.h`, and `fpdf_thumbnail.h` where they
  support static search or font configuration;
- `public/cpp/fpdf_deleters.h` and `public/cpp/fpdf_scopers.h`.

`fpdf_dataavail.h`, `fpdf_progressive.h`, `fpdf_formfill.h`,
`fpdf_fwlevent.h`, and `fpdf_javascript.h` have been removed. `fpdf_annot.h`
contains only the ordinary, non-widget annotation API.
The retained annotation surface covers text notes, freetext, highlights,
underlines, strikeouts, squigglies, ink, links, stamps, geometric annotations,
attachments, and redact marks. Widget annotations are not supported for
creation, and `FPDF_ANNOT` rendering passes `bShowWidget=false`.

`fpdf_edit.h` also exposes conservative real redaction through
`FPDFPage_ApplyRedactions()`. The first supported policy removes only whole
text, path, and image page objects that are fully covered by a supplied
page-space rectangle; partial intersections and nested form/shading content
return explicit error codes instead of producing overlay-only output.

`//:pdfium_light_public_headers_test` compiles the full retained header set as
an external consumer would, without `FPDF_IMPLEMENTATION`.

## Removal seams

The retained `fpdfsdk` source set implements static document APIs only. The
interactive form-fill environment, widget event layer, PWL controls, and
JavaScript callback paths are absent from the build graph.

- #5 removes the JS/V8 branch.
- #3 removed the XFA branch, including `xfa/`, `fpdfsdk/fpdfxfa`,
  `fxjs/xfa`, and XFA-only barcode support. `pdf_enable_xfa=true` now fails
  configuration explicitly.
- #4 removes Formfill/PWL, widget interaction, and form-specific annotation
  APIs while keeping ordinary annotations.

The audit log records removal status and validation evidence. A header hidden
from this manifest is not evidence that its implementation has been deleted.
