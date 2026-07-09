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
| removed | Light build/API boundary | `fpdf_formfill.h`, `fpdf_fwlevent.h`, `fpdf_javascript.h`, and form-specific annotation APIs are removed | retained targets contain only static document code | Establish and enforce a compile-time public boundary before removing monolithic implementation paths. | Light header compile passes; retained build files contain no direct interactive header or target. |
| removed | XFA runtime | XFA APIs remain unavailable to light consumers; remaining formfill-only declarations are removed with #4 | `xfa/`, `fpdfsdk/fpdfxfa/`, `fxjs/xfa/`, and XFA-only test/fuzzer sources removed; `pdf_enable_xfa=true` is rejected | XFA is an interactive form runtime and is outside the static-document scope. | Light header compile passes; `pdf_enable_xfa=true` is rejected; retained build files have no XFA target dependency. |
| removed | AcroForm filling and widget interaction | `fpdf_formfill.h`, `fpdf_fwlevent.h`, form-specific annotation APIs, and form handle types removed | `fpdfsdk/formfiller/`, `fpdfsdk/pwl/`, form-fill environment, widget event/focus/timer paths, and form tests removed | The project will not support filling forms, widget interaction, or viewer-style input handling. | Light header compile passes; retained build files have no Formfill/PWL target or header dependency. |
| removed | Embedded JavaScript execution | V8 declarations and `fpdf_javascript.h` removed | V8-backed implementation, the temporary `fxjs` stub, JS API implementation, JS/V8 tests, and V8 sample removed | JavaScript is interactive runtime behavior and should not execute inside pdfium-light. | Light header compile passes; build files contain no V8 or `fxjs` target/dependency; retained startup no longer initializes a JS runtime. |
| removed | JavaScript action/event callbacks | document/page/form action APIs tied to form handles and JS platform callbacks remain outside the public light API | form event setup was removed from the embedder harness, progressive render tests, and reachable test/fuzzer/sample targets | Action execution exists to support interactive documents and embedding callbacks. | Static document open/render/save tests compile without a form environment or JS platform callbacks. |
| removed | Form-specific annotation helpers | `FPDFAnnot_*` APIs requiring form handles, form flags, form options, checkbox state, or form additional actions removed | form-field helpers and their direct tests removed from `fpdfsdk/fpdf_annot.cpp` | Ordinary annotations remain in scope, but form field inspection and manipulation do not. | Retained annotation calls compile as a light consumer; the implementation statically asserts that highlight, freetext, and ink are supported while widgets are not. |
| removed | XFA barcode support | none intended | `fxbarcode/` and its XFA-only fuzzing coverage removed | Barcode generation is only needed for XFA form rendering in this checkout. | No retained target depends on `fxbarcode`. |
| removed | V8 and form samples | sample-only | `samples/simple_with_v8.cc`, V8 sample include rules, and Formfill setup in `samples/simple_no_v8.c` removed | Samples should reflect the supported light API surface. | Remaining sample demonstrates static document creation without a form environment. |
| removed | Legacy test runner and V8 docs | none | `pdfium_test`, SafetyNet/V8 docs/navigation, JS/V8/XFA test resources, and form-event helper files removed from the retained tree | The light repo should not publish viewer-style or V8 setup instructions that cannot be built from the supported API. | Retained build files no longer reference `pdfium_test`, `fxjs`, V8 test environments, or form-event helper targets. |
| kept | Core PDF image codecs | n/a | JPEG, JPX/OpenJPEG, JBIG2, Flate, Fax, ICC/LCMS paths | These are required for broad static PDF rendering and are not interactive features. | Rendering corpus continues to cover image-heavy PDFs. |
| kept | Ordinary annotations | non-form parts of `fpdf_annot.h` | `CPDF_Annot`, non-widget annotation rendering/editing | Highlights, freetext, ink, links, stamps, comments, and redact marks are static document content. | Light consumer compilation covers retained annotation calls; `FPDF_ANNOT` rendering explicitly excludes widgets. |
| kept | Windows x64 support | n/a | Windows-specific build/render paths that are not 32-bit-only | Windows x64 remains a supported target. | Windows x64 remains in `docs/platform-support.md`; no shared Windows code was removed in #9. |
| removed | Windows x86/32-bit support | n/a | public docs no longer advertise Windows x86/32-bit support; shared Windows code is retained for Windows x64 and the Windows arm64 probe | Windows 32-bit is not a target platform. | `docs/platform-support.md` lists Windows x86/32-bit as out of scope; README no longer advertises it. |
| removed | macOS x64/Intel support | n/a | public docs no longer advertise macOS x64/Intel support | macOS Intel is not a target platform. | `docs/platform-support.md` lists macOS x64/Intel as out of scope; README lists macOS arm64 as the supported macOS target. |
| deferred | Windows arm64 support | n/a | no source removal; probe blocked in this checkout because `gn` and full Chromium `build/` metadata are unavailable | Windows arm64 may be viable through the Chromium/PDFium toolchain, but needs a real probe before support is decided. | `docs/platform-support.md` records the attempted command and blocker. |

## Redaction Audit Requirements

Real redaction is in scope and must be tracked separately from removals. Any
redaction implementation entry should record:

- content types handled: text, image, path, form XObject, annotation;
- whether partial intersections are handled or reported as unsupported;
- proof that redacted text is not extractable after save;
- render-before/render-after validation;
- known unsupported cases.

## Redaction Implementation Notes

| Status | Area | Public API | Implementation / dependency | Reason | Validation expected |
| --- | --- | --- | --- | --- | --- |
| implemented | Conservative whole-object redaction | `FPDFPage_ApplyRedactions()` in `fpdf_edit.h` with explicit `FPDF_REDACTION_*` result codes | Removes fully covered text, path, and image page objects; rejects partial intersections, shading, and nested form XObjects without modifying content | pdfium-light must remove underlying content and must not report success for overlay-only or unsafe redaction cases. | Embedder coverage saves and reloads a redacted document, verifies removed text is not extractable, verifies rendering matches object removal, and verifies partial text intersection returns `FPDF_REDACTION_ERROR_UNSAFE_PARTIAL_INTERSECTION`. |
