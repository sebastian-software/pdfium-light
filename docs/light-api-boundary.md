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
- `fpdf_dataavail.h`, `fpdf_progressive.h`, `fpdf_searchex.h`,
  `fpdf_sysfontinfo.h`, and `fpdf_thumbnail.h` where they support static
  loading, rendering, search, or font configuration;
- `public/cpp/fpdf_deleters.h` and `public/cpp/fpdf_scopers.h`, excluding
  form and JavaScript helpers in light mode.

`fpdf_formfill.h` and `fpdf_fwlevent.h` are not part of the light export.
`fpdf_javascript.h` has been removed. `fpdf_annot.h` does not pull in
`fpdf_formfill.h` for a light consumer and hides its form-specific helpers. #6 removes their implementations
and the temporary implementation-only declarations after Formfill/PWL is gone,
without withdrawing ordinary annotation support.

`//:pdfium_light_public_headers_test` compiles the full retained header set as
an external consumer would, without `FPDF_IMPLEMENTATION`.

## Removal seams

The current `fpdfsdk` source set still combines static APIs with
`cpdfsdk_*` viewer code and has direct dependencies on `fxjs`, `formfiller`,
and `pwl`. The top-level `pdfium` target also depends directly on `fxjs` and
`fpdfsdk/formfiller`; when XFA is enabled it adds `fpdfsdk/fpdfxfa` and XFA
targets. These are internal transitional dependencies, not part of the light
public contract.

- #5 removes the JS/V8 branch. Only the non-executing `fxjs` stub remains
  temporarily for Formfill.
- #3 removes the XFA branch, including `fxjs/xfa` after #5.
- #4 removes Formfill/PWL and splits the viewer-dependent `fpdfsdk` code from
  retained static API implementations.
- #6 removes the form-specific implementations and implementation-only
  declarations in `fpdf_annot.h` while keeping ordinary annotations.

The audit log records removal status and validation evidence. A header hidden
from this manifest is not evidence that its implementation has been deleted.
