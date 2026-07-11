# Rust codec port

pdfium-light keeps its public C API and static-library contract unchanged while
individual internal modules move to Rust. The first production slice is the
byte-oriented PDF filter boundary: ASCII85, ASCIIHex, LZW, PNG/TIFF predictor
transforms, and RunLength encode/decode.

## Migration telemetry

Run the reproducible source-level report from the repository root:

```bash
python3 testing/tools/rust_port_metrics.py
```

The report counts physical lines in tracked first-party source files under
`core`, `fpdfsdk`, and `public`. Rust LOC only counts `.rs` files. Authored
behavior, Rust ABI thunks, generated Rust data, and the C++ adapter are reported
separately. Rust ABI regions use paired `RUST_PORT_METRICS_BEGIN/END` markers;
generated files must carry `@generated` in their first 20 lines. The check fails
if the categories do not add back up to physical Rust LOC. The behavior share
uses authored Rust behavior over implementation LOC, excluding headers, ABI
adapters, generated Rust, and the `fpdfapi/cmaps` mapping-data tree. Both shares
are progress signals, not feature, performance, memory, or binary-size metrics.
`validate_light.py` runs the report with `--check`.

## Renderer differential gate

`RustRendererParityEmbedderTest` renders the versioned cases in
`testing/resources/rust_renderer_corpus.inc` twice in the same process: once
through the retained C++ reference and once through the production candidate
path. Width, height, bitmap format, stride, and every allocated bitmap byte must
match. There is no fuzzy or platform-specific tolerance in this gate.

Phase 0 intentionally routes the candidate to the reference implementation.
Each later rendering slice replaces behavior only inside the candidate path;
the reference selector remains test-only and unchanged until the slice passes.

| Commit | Rust LOC | Authored behavior | Rust ABI | C++ adapter | Generated Rust | Product-native LOC | Physical share | Behavior share | Activated Rust surfaces |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `5fbaaa393` | 367 | — | — | 111 | — | 253,804 | 0.14% | — | ASCII85 encode/decode; RunLength encode/decode |
| Epic #30 predictor slice | 764 | — | — | 176 | — | 254,631 | 0.30% | — | ASCII85 encode/decode; ASCIIHex decode; LZW decode; PNG/TIFF predictor transforms; RunLength encode/decode |
| Epic #30 Fax slice | 1,232 | — | — | 262 | — | 255,525 | 0.48% | — | ASCII85 encode/decode; ASCIIHex decode; LZW decode; PNG/TIFF predictor transforms; CCITT Fax Group 4 and scanline decode; RunLength encode/decode |
| `7a4eca8b4` Phase 0 | 1,236 | 1,033 | 203 | 262 | 0 | 255,791 | 0.48% | 0.54% | Same codec surfaces; renderer differential candidate bootstrapped to the retained C++ reference |
| `cffeca939` Phase 1 BGRA slice | 1,495 | 1,220 | 275 | 392 | 0 | 256,298 | 0.58% | 0.64% | Codec surfaces plus BGRA-to-BGRA normal/separable row compositing; remaining DIB formats stay on C++ |
| `5ab5543b8` Phase 1 row-compositor slice | 2,013 | 1,772 | 241 | 694 | 0 | 257,576 | 0.78% | 0.92% | Codec surfaces plus all PDF blend modes across BGRA, opaque BGR/BGRx, gray, mask, and indexed-palette row compositing |

## Toolchain

The Rust target uses Chromium's hermetic GN toolchain. A complete checkout must
enable the Rust dependency before syncing:

```python
solutions = [{"name": "pdfium", "url": "https://pdfium.googlesource.com/pdfium.git"}]
target_os = ["mac"]
custom_vars = {"checkout_rust": True}
```

Run `gclient sync`, then generate and build with the normal light arguments:

```bash
gn gen out/light-rust --args='enable_rust=true pdf_enable_light=true pdf_enable_v8=false pdf_enable_xfa=false is_component_build=false clang_use_chrome_plugins=false'
ninja -C out/light-rust pdfium_light_validation
out/light-rust/pdfium_unittests --gtest_filter='RustCodecParityTest.*:RustBlendParityTest.*'
out/light-rust/pdfium_embeddertests --gtest_filter='RustMigrationCorpus/RustRendererParityEmbedderTest.*:FPDFAttachmentEmbedderTest.FacturXAttachmentSurvivesSaveReload'
ninja -C out/light-rust pdfium_rust_dib_unittests
out/light-rust/pdfium_rust_dib_unittests
```

The reduced source checkout does not include `build/`, `gn`, or the hermetic
Rust toolchain. It can still run the documented static light gate and the
standalone Rust unit test:

```bash
rustc --edition=2021 --test core/fxcodec/rust/pdfium_rust_codecs.rs -o /tmp/pdfium_rust_codecs_tests
/tmp/pdfium_rust_codecs_tests
```

## Internal C++/Rust boundary

`core/fxcodec/rust/rust_codec_adapter.{h,cpp}` is the sole C++ caller of the
Rust crate. It passes a borrowed `uint8_t` span and receives an owned byte
buffer plus `bytes_consumed` across a C ABI. Rust remains responsible for
freeing that allocation after the adapter has copied it into `DataVector`.

The contract deliberately uses the existing `uint32_t` consumed-byte sentinel
(`UINT32_MAX`) for decoder failures. It does not introduce a public Rust API,
modify public headers, or permit Rust allocations to escape into the C++
allocator domain.

`core/fxge/dib/rust/rust_blend_adapter.{h,cpp}` is the DIB batch boundary.
Separable blend primitives can process whole channel arrays for differential
coverage. The activated row compositor accepts packed source/destination rows
and an optional clip row, mutates caller-owned output in place, and performs no
Rust heap allocation. BGRA, opaque BGR/BGRx, gray, byte/bit mask, and 1-/8-bit
indexed source rows cross FFI once per row across all retained destination
formats and all PDF blend modes. The C++ row algorithms remain available as the
same-process differential oracle while the rest of `fxge/dib` is migrated.

`ScopedRustDibImplementationForTesting` is test-only. It selects the retained
C++ row compositor or the production Rust candidate in the same process. The
production default is always the candidate.

## Parity and activation

The old C++ algorithms remain as explicitly named internal reference functions
while the Rust adapters are active in the real encoder and stream decoder
paths. `RustCodecParityTest` compares exact encoded bytes and decoder output,
failure status, and consumed-byte counts for normal, empty, truncated, and
malformed inputs. This preserves PDFium behavior rather than substituting a
more conventional codec interpretation.

`RustCodecEmbedderTest.FiltersDecodeRenderAndSurviveSaveReload` loads a PDF
whose content stream applies `ASCII85Decode` followed by `RunLengthDecode`.
It renders and extracts the decoded text before and after save/reload, proving
that the normal light document path reaches the adapters.

`RustCodecEmbedderTest.PngPredictorRendersAndSurvivesSaveReload` applies the
PNG predictor after LZW decoding. The predictor corpus also compares all PNG
filter tags, partial rows, invalid geometry, and TIFF 1-, 8-, and 16-bit rows
with the retained C++ reference functions. Flate decompression remains C++
owned in this slice.

CCITT Fax Group 4 and scanline decoding are active through the internal Rust
ABI. The retained C++ references compare Group 3/1D, mixed 2D, Group 4, EOL,
byte alignment, BlackIs1 inversion, truncation, source offsets, and rewind.
The Fax fuzzer also calls the Group 4 helper directly, while
`RustCodecEmbedderTest.FaxImageRendersAndSurvivesSaveReload` proves the real
CCITT image path before and after save/reload. The existing JBIG2 Group 4
consumer continues to call the same active helper through `FaxModule`.

The first Rust scanline slice decodes the bounded image eagerly in the
constructor, retaining the decoded rows and per-row source offsets for the
existing `ScanlineDecoder` API. This deliberately trades the C++ decoder's
O(pitch) streaming state for O(height × pitch) storage and full decode work
before the first requested row. A future streaming Rust decoder can remove
that trade-off without changing the public C API or the differential contract.

No performance claim is made by this port. Any performance decision requires a
separate benchmark against the reference implementation.

## E-invoice regression fixture

`testing/resources/zugferd_facturx_attachment.{in,pdf,xml}` is a small,
hand-authored fixture for attachment retention. It uses a PDF/A-3-style catalog
association, a `factur-x.xml` embedded file with `application/xml` metadata,
and a minimal CrossIndustryInvoice-shaped XML payload. The source files are
created in this repository for testing and are intended to be redistributable
under the repository license.

`FPDFAttachmentEmbedderTest.FacturXAttachmentSurvivesSaveReload` verifies the
attachment name, MIME description, exact XML bytes, and save/reload retention.
It does not validate an EN 16931 schema, cryptographic signatures, or complete
PDF/A conformance.
