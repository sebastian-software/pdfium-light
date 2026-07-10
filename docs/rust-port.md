# Rust codec port

pdfium-light keeps its public C API and static-library contract unchanged while
individual internal modules move to Rust. The first production slice is the
byte-oriented PDF filter boundary: ASCII85, ASCIIHex, LZW, and RunLength
encode/decode.

## Migration telemetry

Run the reproducible source-level report from the repository root:

```bash
python3 testing/tools/rust_port_metrics.py
```

The report counts physical lines in tracked first-party source files under
`core`, `fpdfsdk`, and `public`. Rust LOC only counts `.rs` files; the C++
adapter is reported separately. This is a progress signal, not a feature,
performance, memory, or binary-size metric. `validate_light.py` runs the
report with `--check`.

| Commit | Rust LOC | C++ adapter LOC | Product-native LOC | Rust share | Activated Rust surfaces |
| --- | ---: | ---: | ---: | ---: | --- |
| `5fbaaa393` | 367 | 111 | 253,804 | 0.14% | ASCII85 encode/decode; RunLength encode/decode |

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
out/light-rust/pdfium_unittests --gtest_filter='RustCodecParityTest.*:FPDFAttachmentEmbedderTest.FacturXAttachmentSurvivesSaveReload'
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
