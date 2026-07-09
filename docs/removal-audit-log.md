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
| planned | XFA runtime | XFA-related declarations in `fpdfview.h`, `fpdf_formfill.h` | `xfa/`, `fpdfsdk/fpdfxfa/`, `fxjs/xfa/`, XFA-only codec paths | XFA is an interactive form runtime and is outside the static-document scope. | Build without XFA targets; render ordinary non-XFA PDFs; ensure XFA headers are not exported. |
| planned | AcroForm filling and widget interaction | `fpdf_formfill.h`, `fpdf_fwlevent.h`, form-specific APIs in `fpdf_annot.h` | `fpdfsdk/formfiller/`, `fpdfsdk/pwl/`, widget focus/event/timer paths | The project will not support filling forms, widget interaction, or viewer-style input handling. | Static rendering still works; non-widget annotations still render; removed form headers no longer compile for callers. |
| planned | Embedded JavaScript execution | V8-related declarations in `fpdfview.h`, `fpdf_javascript.h` if not retained for static inspection | V8-backed `fxjs` implementation, V8 deps, JS samples/tests | JavaScript is interactive runtime behavior and should not execute inside pdfium-light. | Build graph has no V8 dependency; retained APIs do not initialize or execute JS. |
| planned | JavaScript action/event callbacks | document/page/form action APIs tied to form handles and JS platform callbacks | `CPDFSDK_FormFillEnvironment` action execution and JS platform callback paths | Action execution exists to support interactive documents and embedding callbacks. | Opening/rendering a static PDF does not require JS runtime or form environment. |
| planned | Form-specific annotation helpers | `FPDFAnnot_*` APIs requiring `FPDF_FORMHANDLE`, form flags, form options, checkbox state, form additional actions | form-field helpers in `fpdfsdk/fpdf_annot.cpp` and related tests | Ordinary annotations remain in scope, but form field inspection and manipulation do not. | Highlight, freetext, ink, link, stamp, text, geometric, and redact annotations still compile and test. |
| planned | XFA barcode support | none intended | `fxbarcode/` if only referenced by XFA after pruning | Barcode generation is only needed for XFA form rendering in this checkout. | No retained target depends on `fxbarcode`. |
| planned | V8 sample and form sample | sample-only | `samples/simple_with_v8.cc`, formfill sample code in `samples/simple_no_v8.c` | Samples should reflect the supported light API surface. | Remaining sample demonstrates static load/render/edit/save without form environment. |
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

