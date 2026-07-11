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
| `360e1fbb5` Phase 1 CMYK slice | 8,678 | 1,867 | 241 | 741 | 6,570 | 264,368 | 3.28% | 0.97% | Prior surfaces plus Adobe CMYK scalar conversion and batch CMYK-to-BGR image rows; generated lookup data excluded from authored behavior |
| `19ed48ba3` Phase 1 bitmap-alpha slice | 8,807 | 1,996 | 241 | 839 | 6,570 | 264,700 | 3.33% | 1.04% | Prior surfaces plus BGRA red-from-alpha, opaque-alpha, mask multiplication, and constant-alpha multiplication over complete bitmaps |
| `5e7dc096b` Phase 1 bitmap-clear slice | 8,864 | 2,053 | 241 | 875 | 6,570 | 264,856 | 3.35% | 1.07% | Prior surfaces plus whole-bitmap clearing across 1-/8-bpp mask/RGB, BGR, BGRx, and BGRA formats |
| `ecaa53bbb` Phase 1 color-scale slice | 8,903 | 2,092 | 241 | 897 | 6,570 | 264,944 | 3.36% | 1.09% | Prior surfaces plus BGR/BGRx/BGRA integer grayscale conversion over complete bitmaps |
| `bdb469824` Phase 1 bitmap-geometry slice | 8,972 | 2,161 | 241 | 923 | 6,570 | 265,085 | 3.38% | 1.12% | Prior surfaces plus checked pitch and allocation-size calculation for every retained bitmap format |
| `88d1de27d` Phase 1 bitmap-population slice | 9,070 | 2,259 | 241 | 975 | 6,570 | 265,288 | 3.42% | 1.17% | Prior surfaces plus 1-bpp-to-8-bpp mask expansion and pitch-aware external-span population |
| `95b339296` Phase 1 equal-format transfer slice | 9,214 | 2,403 | 241 | 1,029 | 6,570 | 265,536 | 3.47% | 1.24% | Prior surfaces plus equal-format 1-bpp bit-range and 8/24/32-bpp row transfers |
| `32153fe46` Phase 1 overlap-geometry slice | 9,378 | 2,567 | 241 | 1,090 | 6,570 | 265,843 | 3.53% | 1.33% | Prior surfaces plus checked source/destination overlap and optional clip normalization |
| `bf269cbd2` Phase 1 palette-primitives slice | 9,488 | 2,677 | 241 | 1,146 | 6,570 | 266,099 | 3.57% | 1.38% | Prior surfaces plus default palette generation, default ARGB lookup, and exact custom-palette search |
| `c5999179d` Phase 1 buffer-conversion slice | 9,697 | 2,886 | 241 | 1,181 | 6,570 | 266,479 | 3.64% | 1.49% | Prior surfaces plus mask, indexed, grayscale, BGR, BGRx, and BGRA row conversion with default and custom palettes |
| `66a4f3d15` Phase 1 1-bpp mask-composite slice | 9,781 | 2,970 | 241 | 1,207 | 6,570 | 266,640 | 3.67% | 1.53% | Prior surfaces plus clipped 1-bpp mask OR compositing with preserved surrounding and unset destination bits |

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

Adobe CMYK interpolation is active through the same boundary for scalar colors
and complete image rows. `testing/tools/generate_rust_cmyk_table.py` reproduces
the 6,561-entry Rust lookup table from the retained C++ oracle; the generated
file is reported as generated Rust rather than authored behavior. The focused
gate compares 456,976 interpolation-boundary combinations plus a 256-pixel
batch corpus that verifies PDFium's historical BGR output order.

BGRA bitmap alpha mutations also cross the boundary once per complete bitmap.
Rust walks explicit width, height, and pitch values without allocating and
leaves row padding untouched. Same-process tests compare red-from-alpha,
opaque-alpha, mask multiplication for BGRA and converted BGRx input, and
constant-alpha multiplication against the retained C++ loops.

Whole-bitmap clearing preserves the format-specific treatment of row padding:
1-/8-bpp images and equal-channel BGR fill padding, while non-gray BGR and
four-component formats write active pixels only. The focused differential
matrix covers seven formats and transparent, gray, and colored ARGB values.

Color-scale conversion handles three- and four-component bitmap rows in one
FFI call, preserves alpha/x bytes and padding, and uses the same integer
11/59/30 weights as `FXRGB2GRAY`. Six focused cases cover BGR, BGRx, and BGRA
with both public mode values against the retained C++ loop.

Bitmap pitch and allocation-size calculation uses checked `u32` arithmetic in
Rust before any allocation. The differential matrix covers all eight format
values, invalid and boundary dimensions, caller-supplied pitches, alignment,
short-pitch rejection, and multiplication overflow in 4,032 combinations.

Bitmap population expands packed 1-bpp mask rows and copies arbitrary external
row pitches into zeroed destination storage through one FFI call per bitmap.
The differential cases cover non-byte-aligned widths, preserved destination
padding for bit expansion, and source pitches both below and above destination
pitch for generic population.

Equal-format transfer keeps source objects and row access in C++ while Rust
owns each copied row. Multi-byte rows preserve the reference `memcpy`
non-overlap contract; 1-bpp rows retain arbitrary source/destination bit
offsets, surrounding bits, and left-to-right alias order. Fourteen focused
cases cover seven formats and both positive and clipped negative source
offsets.

One-bit mask compositing now crosses the same row boundary after C++ normalizes
source and destination overlap. Rust ORs only set source bits into arbitrary
destination offsets, preserving unset pixels, surrounding bits, and exact
left-to-right alias order. Three same-process cases cover ordinary and clipped
negative source and destination coordinates.

The shared overlap helper now performs source bounds, destination bounds,
optional clip intersection, coordinate rebasing, and checked `i32` overflow in
Rust before transfers or compositing begin. A deterministic 1,026-case corpus
compares normal, negative, clipped, empty, and overflow-near geometry directly
with the retained C++ helper.

Palette storage remains a C++ `DataVector`, while Rust fills default 1-bpp and
8-bpp ARGB entries, resolves default entries, and searches exact custom colors.
Focused tests cover every default entry, full generated palettes, mutation of
custom entries, and successful custom lookup through bitmap clearing.

Buffer conversion now keeps bitmap allocation and palette ownership in C++
while Rust converts complete rows between the retained 1-/8-/24-/32-bpp mask,
indexed, grayscale, BGR, BGRx, and BGRA representations. The boundary preserves
palette propagation, destination alpha/x bytes, and the reference's unsupported
1-bpp-to-ARGB contract. Differential cases cover default and custom palettes,
all supported source widths, and 8-bpp, BGR, and BGRA destinations.

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
