# pdfium-light Validation

This document defines the repeatable validation gate for pdfium-light changes.
The gate has two layers:

1. a local static/syntax gate that runs from this reduced repository; and
2. a full GN/Ninja gate that requires a complete Chromium/PDFium `gclient`
   checkout with `depot_tools`.

The local gate is the required minimum for removal PRs. The full gate is the
release-quality validation path once the complete toolchain is available.

Rust codec changes additionally require a checkout with
`custom_vars = {"checkout_rust": True}` before `gclient sync` and
`enable_rust=true` in GN args. See
[`rust-port.md`](rust-port.md) for the C++/Rust ABI contract and focused parity
and e-invoice regression commands.

The local gate also runs `testing/tools/rust_port_metrics.py --check` so the
Rust migration denominator and active-surface report stay available from the
reduced checkout.

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
- absence of stale macOS x86-only expectation fallbacks, suppressions, and
  golden PNGs;
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
gn gen out/light --args='enable_rust=true pdf_enable_light=true pdf_enable_v8=false pdf_enable_xfa=false is_component_build=false clang_use_chrome_plugins=false'
```

Then build the focused light validation target:

```bash
ninja -C out/light pdfium_light_validation
```

For the Rust codec port, make sure the checkout was synced with
`checkout_rust=true` and generated with `enable_rust=true`; the GN graph then
builds `//core/fxcodec/rust:pdfium_rust_codecs` as part of the retained
`fxcodec` target.

Run the zero-tolerance renderer differential corpus explicitly with:

```bash
out/light/pdfium_embeddertests \
  --gtest_filter='RustMigrationCorpus/RustRendererParityEmbedderTest.*'
```

The comparison includes bitmap dimensions, format, stride, and all allocated
bytes. Platform expectation tolerances do not apply to this same-process gate.

For Rust render-plan, AGG path, and glyph-planning changes, build and run the
native Rust test targets before the renderer corpus:

```bash
ninja -C out/light \
  pdfium_rust_render_unittests \
  pdfium_rust_agg_unittests \
  pdfium_rust_glyph_unittests
out/light/pdfium_rust_render_unittests
out/light/pdfium_rust_agg_unittests
out/light/pdfium_rust_glyph_unittests
```

The renderer corpus additionally compares the exact render-command trace and
the AGG path, draw-plan, stroke-matrix, dash-decision, dash-value, and phase
traces plus every glyph bitmap cache-key word, checked bitmap-origin plan,
device-origin rounding decision, aggregated glyph bounds, bitmap lookup action,
FreeType glyph load plan, PDF text dispatch decision, and active text-pattern
decision plus text-matrix availability and the executed pattern/path/normal
text backend. AGG and FreeType remain the retained raster and font backends;
these tests validate the Rust-owned planning and adapter boundaries around
them. The harness opens independent documents for Oracle and Candidate so
glyph/font cache state cannot hide or manufacture trace differences.

For Rust parser changes, build the native parser tests, the C++ object-snapshot
corpus, and the retained parser fuzzer source:

```bash
ninja -C out/light \
  pdfium_rust_parser_unittests \
  pdfium_unittests \
  testing/fuzzers:pdf_parser_fuzzer_src \
  testing/fuzzers:pdf_parser_fuzzer_corpus
out/light/pdfium_rust_parser_unittests
out/light/pdfium_unittests \
  --gtest_filter='ParserTest.RustCandidateMatchesCppCrossRefObjectSnapshots:SimpleParserTest.*'
out/light/pdf_parser_fuzzer_corpus
```

The versioned corpus in `testing/resources/rust_parser_corpus.inc` compares
parse status, rebuild decisions, trailer object number, and every
cross-reference object entry for normal, defaulted, unknown, and truncated
streams. `pdf_parser_fuzzer` performs allocation-free token and consumed-offset
differential checks before sending the same bounded input through the public
in-memory document parser. The standalone corpus executable runs the same
fuzzer entry point over every versioned input, including public Oracle/Candidate
load errors, document metadata, and bounded page geometry. `pdfium_all` must
continue to compile the fuzzer source.

Run the bounded parser corpus under AddressSanitizer as the Phase 6 resource
gate. `out/light-rust-asan/args.gn` uses the normal light arguments plus
`is_asan = true`:

```bash
gn gen out/light-rust-asan
ninja -C out/light-rust-asan testing/fuzzers:pdf_parser_fuzzer_corpus
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 \
  out/light-rust-asan/pdf_parser_fuzzer_corpus
```

For Rust DIB changes, build and run the native Rust target and the exhaustive
C++/Rust blend and row parity cases:

```bash
ninja -C out/light pdfium_rust_dib_unittests
out/light/pdfium_rust_dib_unittests
out/light/pdfium_unittests \
  --gtest_filter='RustBlendParityTest.*:ScanlineCompositorTest.CompositeRgbBitmapLineBgra*'
```

The BGRA row parity test covers every separable mode with clip enabled and
disabled and with both byte-order settings. It selects C++ and Rust explicitly
in the same process; the ordinary compositor tests execute the production Rust
default.

For the broader retained test/fuzzer set, run:

```bash
ninja -C out/light pdfium_all
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
