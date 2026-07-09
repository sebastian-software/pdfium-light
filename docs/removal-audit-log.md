# Removal Audit Log

This log records substantial feature removals from pdfium-light. It should be
updated when code, public APIs, tests, samples, or dependencies are removed.

The log is not a replacement for Git history. Its job is to make the product
scope decision visible: what was removed, why it was removed, and what retained
behavior was checked afterward.

## Status Legend

- `planned`: identified for removal, not yet removed.
- `in-progress`: removal has started but is not fully validated.
- `removed`: code or API has been removed and validation is recorded.
- `deferred`: removal is desired but blocked by a dependency or product
  decision.
- `kept`: considered for removal but intentionally retained.

## Audit Entries

| Status | Area | Public API | Implementation / dependency | Reason | Validation expected |
| --- | --- | --- | --- | --- | --- |
| in-progress | Light build/API boundary | Light export manifest excludes `fpdf_formfill.h` and `fpdf_fwlevent.h`; `fpdf_javascript.h` is removed; `fpdf_annot.h` neither includes formfill nor exposes form helpers to light consumers | `fpdfsdk` still directly depends on the non-executing `fxjs` stub, `formfiller`, and `pwl`; the top-level target retains the latter two until #3–#4 land | Establish a compile-time public boundary before removing monolithic implementation paths. | Build `//:pdfium_light_public_headers_test`; confirm the light manifest contains no direct interactive header. |
| removed | XFA runtime | XFA APIs remain unavailable to light consumers; remaining formfill-only declarations are removed with #4 | `xfa/`, `fpdfsdk/fpdfxfa/`, `fxjs/xfa/`, and XFA-only test/fuzzer sources removed; `pdf_enable_xfa=true` is rejected | XFA is an interactive form runtime and is outside the static-document scope. | Light header compile passes; `pdf_enable_xfa=true` is rejected; retained build files have no XFA target dependency. |
| planned | AcroForm filling and widget interaction | `fpdf_formfill.h`, `fpdf_fwlevent.h`, form-specific APIs in `fpdf_annot.h` | `fpdfsdk/formfiller/`, `fpdfsdk/pwl/`, widget focus/event/timer paths | The project will not support filling forms, widget interaction, or viewer-style input handling. | Static rendering still works; non-widget annotations still render; removed form headers no longer compile for callers. |
| removed | Embedded JavaScript execution | V8 declarations and `fpdf_javascript.h` removed | V8-backed `fxjs` implementation, JS API implementation, JS/V8 tests, and V8 sample removed; only the non-executing Formfill stub remains until #4 | JavaScript is interactive runtime behavior and should not execute inside pdfium-light. | Light header compile passes; build files contain no V8 target or dependency; retained startup no longer initializes a JS runtime. |
| planned | JavaScript action/event callbacks | document/page/form action APIs tied to form handles and JS platform callbacks | `CPDFSDK_FormFillEnvironment` action execution and JS platform callback paths | Action execution exists to support interactive documents and embedding callbacks. | Opening/rendering a static PDF does not require JS runtime or form environment. |
| planned | Form-specific annotation helpers | `FPDFAnnot_*` APIs requiring `FPDF_FORMHANDLE`, form flags, form options, checkbox state, form additional actions | form-field helpers in `fpdfsdk/fpdf_annot.cpp` and related tests | Ordinary annotations remain in scope, but form field inspection and manipulation do not. | Highlight, freetext, ink, link, stamp, text, geometric, and redact annotations still compile and test. |
| removed | XFA barcode support | none intended | `fxbarcode/` and its XFA-only fuzzing coverage removed | Barcode generation is only needed for XFA form rendering in this checkout. | No retained target depends on `fxbarcode`. |
| in-progress | V8 sample and form sample | sample-only | `samples/simple_with_v8.cc` removed; formfill sample code in `samples/simple_no_v8.c` remains until #4 | Samples should reflect the supported light API surface. | Remaining sample demonstrates static load/render/edit/save without form environment. |
| kept | Core PDF image codecs | n/a | JPEG, JPX/OpenJPEG, JBIG2, Flate, Fax, ICC/LCMS paths | These are required for broad static PDF rendering and are not interactive features. | Rendering corpus continues to cover image-heavy PDFs. |
| kept | Ordinary annotations | non-form parts of `fpdf_annot.h` | `CPDF_Annot`, non-widget annotation rendering/editing | Highlights, freetext, ink, links, stamps, comments, and redact marks are static document content. | Annotation rendering and inspection tests remain. |
| kept | Windows x64 support | n/a | Windows-specific build/render paths that are not 32-bit-only | Windows x64 remains a supported target. | Windows x64 build remains in validation matrix. |
| planned | Windows x86/32-bit support | n/a | 32-bit-only build config, artifacts, tests, and docs where separable | Windows 32-bit is not a target platform. | Windows x64 and probe Windows arm64 are unaffected. |
| planned | macOS x64/Intel support | n/a | macOS x64-only artifacts, tests, and docs where separable | macOS Intel is not a target platform. | macOS arm64 remains in validation matrix. |

## Redaction Audit Requirements

Real redaction is in scope and must be tracked separately from removals. Any
redaction implementation entry should record:

- content types handled: text, image, path, form XObject, annotation;
- whether partial intersections are handled or reported as unsupported;
- proof that redacted text is not extractable after save;
- render-before/render-after validation;
- known unsupported cases.
