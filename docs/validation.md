# pdfium-light Validation

This document defines the repeatable validation gate for pdfium-light changes.
The gate has two layers:

1. a local static/syntax gate that runs from this reduced repository; and
2. a full GN/Ninja gate that requires a complete Chromium/PDFium `gclient`
   checkout with `depot_tools`.

The local gate is the required minimum for removal PRs. The full gate is the
release-quality validation path once the complete toolchain is available.

## Local reduced-checkout gate

Run this from the repository root:

```bash
python3 testing/tools/validate_light.py
```

The command checks:

- the retained public header manifest in `BUILD.gn`;
- absence of removed public headers such as `fpdf_formfill.h`,
  `fpdf_fwlevent.h`, and `fpdf_javascript.h`;
- absence of removed form-fill, JavaScript, XFA, and document-action symbols
  from exported light headers;
- retained build target names for `pdfium`, `pdfium_unittests`,
  `pdfium_embeddertests`, `pdfium_light_validation`, `pdfium_all`, and
  `pdfium_light_public_headers_test`;
- absence of the removed `pdfium_test` and `fxjs` build targets from retained
  top-level build files;
- source-level smoke coverage markers for render, text, edit/save,
  annotation, and redaction behavior;
- syntax-only compilation of the retained public header compile test and the
  static sample.

Use this narrower command only when a compiler is not available and you need to
verify the repository structure:

```bash
python3 testing/tools/validate_light.py --skip-syntax
```

## Full depot_tools gate

In a complete `gclient` checkout with `gn`, `ninja`, and the Chromium build
metadata available, generate a light build with:

```bash
gn gen out/light --args='pdf_enable_light=true pdf_enable_v8=false pdf_enable_xfa=false is_component_build=false clang_use_chrome_plugins=false'
```

Then build the focused light validation target:

```bash
ninja -C out/light pdfium_light_validation
```

For the broader retained test/fuzzer set, run:

```bash
ninja -C out/light pdfium pdfium_all
```

Run the retained test binaries:

```bash
out/light/pdfium_unittests
out/light/pdfium_embeddertests
```

`pdfium_light_validation` intentionally covers the light library, retained
public header compile test, `pdfium_unittests`, and `pdfium_embeddertests`.
`pdfium_all` adds `pdfium_diff` and retained fuzzers. The legacy viewer-style
`pdfium_test` binary is not part of pdfium-light.

## Smoke coverage map

| Capability | Required coverage | Current source |
| --- | --- | --- |
| Retained public headers | Compile the public header set a light consumer receives, without implementation-only defines. | `testing/light_api_headers_test.cc` via `pdfium_light_public_headers_test` |
| Static rendering | Render ordinary pages and compare bitmap expectations. | `fpdfsdk/fpdf_view_embeddertest.cpp`, including `RenderHelloWorldWithFlags` and `RenderManyRectanglesWithFlags` |
| Progressive rendering | Exercise retained progressive render entry points without a form environment. | `core/fpdfapi/render/fpdf_progressive_render_embeddertest.cpp` |
| Text extraction | Load text pages, extract text, count chars, search, and inspect text geometry. | `fpdfsdk/fpdf_text_embeddertest.cpp` |
| Edit/save remove | Remove page objects, regenerate content, save, reload, render, and confirm removed resources stay gone. | `fpdfsdk/fpdf_edit_embeddertest.cpp`, including `RemoveTextObject` |
| Edit/save insert | Insert page objects, regenerate content, save, reload, and verify ordering/rendering. | `fpdfsdk/fpdf_edit_embeddertest.cpp`, including `InsertPageObjectAndSave` and `InsertObjectAtIndexPersistsOrder` |
| Ordinary annotation rendering | Render non-interactive annotations with `FPDF_ANNOT`. | `fpdfsdk/fpdf_view_embeddertest.cpp`, including `RenderAnnotationWithPrintingFlag` |
| Ordinary annotation inspection | Count annotations and inspect retained annotation subtypes without form handles. | `fpdfsdk/fpdf_text_embeddertest.cpp`, including `AnnotLinks`; `testing/light_api_headers_test.cc` |
| Real redaction | Apply redactions, save/reload, compare rendering, and verify removed text is no longer extractable. | `fpdfsdk/fpdf_edit_embeddertest.cpp`, including `ApplyRedactionsRemovesCoveredTextAndSaves` |
| Unsafe redaction rejection | Reject unsafe partial intersections without mutating content. | `fpdfsdk/fpdf_edit_embeddertest.cpp`, including `ApplyRedactionsRejectsPartialTextIntersection` |
| Removed API absence | Fail if removed headers or removed public tokens return to the exported light surface. | `testing/tools/validate_light.py` |

## Platform validation

Use the supported platform matrix in
[`docs/platform-support.md`](platform-support.md) when recording validation
evidence. Platform-specific commands should use the same `pdf_enable_light`,
`pdf_enable_v8=false`, and `pdf_enable_xfa=false` arguments shown above unless
the platform note documents a stricter requirement.
