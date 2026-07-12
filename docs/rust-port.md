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
| `659e766e0` Phase 1 rectangle-composite slice | 10,032 | 3,221 | 241 | 1,253 | 6,570 | 267,006 | 3.76% | 1.66% | Prior surfaces plus solid rectangle compositing across 1-/8-bpp mask/RGB, BGR, BGRx, and BGRA formats |
| `28c172466` Phase 1 alpha-mask clone slice | 10,089 | 3,278 | 241 | 1,272 | 6,570 | 267,116 | 3.78% | 1.69% | Prior surfaces plus scanline-backed BGRA alpha extraction into 8-bpp masks with untouched destination padding |
| `c6bc0bb76` Phase 1 bitmap-clip slice | 10,205 | 3,394 | 241 | 1,296 | 6,570 | 267,349 | 3.82% | 1.74% | Prior surfaces plus aligned 1-/8-/24-/32-bpp clipping and native-word shifted 1-bpp clipping |
| `3708a4ab5` Phase 1 bitmap-copy slice | 10,245 | 3,434 | 241 | 1,310 | 6,570 | 267,457 | 3.83% | 1.76% | Prior surfaces plus complete scanline copies, including source padding, for every retained format |
| `64b671193` Phase 1 stretch-weight slice | 10,594 | 3,783 | 241 | 1,355 | 6,570 | 267,944 | 3.95% | 1.94% | Prior surfaces plus bounded stretch-table layout and exact nearest, bilinear, area, clipped, and mirrored fixed-point weights |
| `f1ec24da9` Phase 1 horizontal-stretch slice | 10,980 | 4,169 | 241 | 1,403 | 6,570 | 268,477 | 4.09% | 2.13% | Prior surfaces plus all six horizontal pixel transforms for mask, indexed, BGR, BGRx, and BGRA scanlines |
| `626a359b0` Phase 1 vertical-stretch slice | 11,281 | 4,470 | 241 | 1,453 | 6,570 | 268,837 | 4.20% | 2.28% | Prior surfaces plus vertical scalar/color filtering and BGRA unpremultiplication for complete destination rows |
| `2363f01d1` Phase 1 bitmap-transpose slice | 11,452 | 4,641 | 241 | 1,489 | 6,570 | 269,103 | 4.26% | 2.37% | Prior surfaces plus packed-bit and 8-/24-/32-bpp transposition with independent axis mirroring |
| `408f15262` Phase 1 alpha-transform slice | 11,645 | 4,834 | 241 | 1,529 | 6,570 | 269,383 | 4.32% | 2.46% | Prior surfaces plus fixed-matrix coordinate mapping and bilinear sampling for transformed alpha masks |
| `1eebb71c9` Phase 1 indexed-transform slice | 11,816 | 5,005 | 241 | 1,574 | 6,570 | 269,661 | 4.38% | 2.55% | Prior surfaces plus fixed-matrix bilinear indexed/grayscale sampling and exact ARGB palette resolution |
| `4cd581a61` Phase 1 color-transform slice | 12,012 | 5,201 | 241 | 1,621 | 6,570 | 269,959 | 4.45% | 2.64% | Prior surfaces plus channel-wise fixed-matrix sampling for opaque BGR/BGRx, BGRA, and retained raw CMYK transforms |
| `5322e1543` Phase 1 stretch-palette slice | 12,061 | 5,250 | 241 | 1,637 | 6,570 | 270,030 | 4.47% | 2.67% | Prior surfaces plus signed integer interpolation of opaque 1-bpp source colors into 256-entry stretch palettes |
| `a799e7b4b` Phase 1 bitmap-flip slice | 12,170 | 5,359 | 241 | 1,661 | 6,570 | 270,174 | 4.50% | 2.72% | Prior surfaces plus copy/horizontal flip rows for packed 1-bpp and 8-/24-/32-bpp Windows `FlipImage` behavior |
| `dc837a462` Phase 2 render-request slice | 12,279 | 5,468 | 241 | 1,755 | 6,570 | 270,440 | 4.54% | 2.77% | Prior surfaces plus Rust planning for public render flags, color mode, print/view usage, annotations, and device restoration |
| `da914a73c` Phase 2 object-dispatch slice | 12,355 | 5,544 | 241 | 1,808 | 6,570 | 270,607 | 4.57% | 2.81% | Prior surfaces plus Rust-owned dispatch planning for text, path, image, shading, and form page objects |
| `c78f121a5` Phase 2 object-list slice | 12,506 | 5,695 | 241 | 1,910 | 6,570 | 270,904 | 4.62% | 2.88% | Prior surfaces plus stop-object precedence, active-state filtering, exact float clip rejection, and same-process render-command traces |
| `aa5d3cdba` Phase 2 render-layer slice | 12,707 | 5,896 | 241 | 2,054 | 6,570 | 271,338 | 4.68% | 2.98% | Prior surfaces plus Rust-owned layer setup/completion planning and ordered layer iteration with stop-aware C++ callbacks |
| `08a3a00d4` Phase 3 path-paint slice | 12,824 | 6,013 | 241 | 2,135 | 6,570 | 271,558 | 4.72% | 3.04% | Prior surfaces plus Rust-owned path fill/stroke preservation, no-paint early completion, and forced-color fill-to-stroke conversion |
| `adab6e20f` Phase 3 path-matrix slice | 12,884 | 6,073 | 241 | 2,164 | 6,570 | 271,655 | 4.74% | 3.07% | Prior surfaces plus Rust-owned path matrix availability across scale, swapped axes, shear, signed zero, infinity, and NaN semantics |
| `336749474` Phase 3 fill-options slice | 13,016 | 6,205 | 241 | 2,244 | 6,570 | 271,876 | 4.79% | 3.13% | Prior surfaces plus Rust-owned fill rule, rectangle AA, path aliasing, stroke adjustment, stroke, and Type3 path-option planning |
| `b2613c9f5` Phase 3 AGG dash slice | 13,111 | 6,300 | 241 | 2,339 | 6,570 | 272,075 | 4.82% | 3.18% | Prior surfaces plus Rust-owned dash-pattern applicability at the AGG stroke boundary, including exact threshold and non-finite semantics |
| `3b1aa7950` Phase 3 AGG stroke-plan slice | 13,301 | 6,490 | 241 | 2,423 | 6,570 | 272,378 | 4.88% | 3.27% | Prior surfaces plus Rust-owned line-cap, line-join, effective-width, and miter-limit planning at the AGG stroke boundary |
| `2220a8c1b` Phase 3 AGG dash-normalization slice | 13,425 | 6,614 | 241 | 2,499 | 6,570 | 272,601 | 4.92% | 3.33% | Prior surfaces plus Rust-owned dash replacement, absolute scaling, and dash-phase normalization through an allocation-free callback boundary |
| `44b0aa456` Phase 3 AGG path-emission slice | 13,749 | 6,938 | 241 | 2,595 | 6,570 | 273,129 | 5.03% | 3.49% | Prior surfaces plus Rust-owned path iteration, matrix transformation, hard clipping, degenerate-line repair, Bezier grouping, and Move/Line/Bezier/Close emission |
| `9f7257f29` Phase 3 AGG path-draw-plan slice | 13,848 | 7,037 | 241 | 2,663 | 6,570 | 273,341 | 5.07% | 3.53% | Prior surfaces plus Rust-owned fill activation, even-odd/non-zero rule selection, stroke suppression, and zero-area/normal stroke-mode orchestration |
| `4e881dc78` Phase 3 AGG stroke-matrix slice | 14,005 | 7,194 | 241 | 2,740 | 6,570 | 273,619 | 5.12% | 3.61% | Prior surfaces plus Rust-owned identity, scale extraction, normalized stroke matrix, inverse, and path-matrix decomposition |
| `17c6e369b` Phase 4 glyph cache-key slice | 14,211 | 7,400 | 241 | 2,872 | 6,570 | 274,001 | 5.19% | 3.71% | Prior surfaces plus Rust-owned 6-, 7-, 9-, and 10-word glyph bitmap cache-key shape planning for base, substitution, and native-text variants |
| `d9705f514` Phase 4 glyph-origin slice | 14,307 | 7,496 | 241 | 2,943 | 6,570 | 274,199 | 5.22% | 3.75% | Prior surfaces plus checked Rust-owned glyph bitmap origin placement for bounding-box and draw composition offsets |
| `026b3cd97` Phase 4 device-origin slice | 14,411 | 7,600 | 241 | 3,008 | 6,570 | 274,388 | 5.25% | 3.80% | Prior surfaces plus Rust-owned LCD-floor and saturated half-away-from-zero device-origin rounding before glyph bitmap loading |
| `074c9b1b5` Phase 4 glyph-bounds slice | 14,657 | 7,846 | 241 | 3,075 | 6,570 | 274,766 | 5.33% | 3.92% | Prior surfaces plus Rust-owned borrowed glyph iteration, LCD width adjustment, checked edge construction, skip behavior, and bounding-box min/max aggregation |
| `8d94db0fa` Phase 4 bitmap-lookup slice | 14,740 | 7,929 | 241 | 3,131 | 6,570 | 274,956 | 5.36% | 3.96% | Prior surfaces plus Rust-owned invalid-glyph rejection, requested-key lookup, native-cache return, and non-native fallback/option-update planning |
| `b8ba94e27` Phase 4 path/width cache slice | 14,984 | 8,173 | 241 | 3,209 | 6,570 | 275,308 | 5.44% | 4.07% | Prior surfaces plus Rust-owned glyph-path and glyph-width cache key planning; C++ retains cache maps, font calls, and path ownership |
| `23789239e` Phase 4 FreeType load slice | 15,109 | 8,298 | 241 | 3,269 | 6,570 | 275,531 | 5.48% | 4.13% | Prior surfaces plus Rust-owned render/path load-flag and retry planning; C++ retains face state and FreeType calls |
| `803cdc455` Phase 5 text-dispatch slice | 15,219 | 8,408 | 241 | 3,330 | 6,570 | 275,724 | 5.52% | 4.19% | Prior surfaces plus Rust-owned PDF text skip, Type 3, fill, stroke, and clip dispatch; C++ retains colors, matrices, fonts, and drawing |
| `85f0e3191` Phase 5 text-pattern slice | 15,260 | 8,449 | 241 | 3,351 | 6,570 | 275,791 | 5.53% | 4.20% | Prior surfaces plus Rust-owned active fill/stroke pattern-path selection; C++ retains ARGB resolution and drawing |
| `4ce390dcd` Phase 5 text-matrix slice | 15,260 | 8,449 | 241 | 3,351 | 6,570 | 275,799 | 5.53% | 4.20% | Prior surfaces plus Rust-owned text-matrix availability validation; C++ retains matrix composition and drawing |
| `8dc87fd9e` Phase 5 text-backend slice | 15,292 | 8,481 | 241 | 3,364 | 6,570 | 275,848 | 5.54% | 4.22% | Prior surfaces plus Rust-owned normal/path text-backend route; C++ retains backend calls and arguments |
| `8ff532f47` Phase 5 text-CTM slice | 15,326 | 8,515 | 241 | 3,383 | 6,570 | 275,906 | 5.55% | 4.23% | Prior surfaces plus Rust-owned stroked-text CTM-adjustment decision; C++ retains matrix work and backend calls |
| `9286e9a6d` Phase 5 path-text options slice | 15,385 | 8,574 | 241 | 3,413 | 6,570 | 276,002 | 5.57% | 4.26% | Prior surfaces plus Rust-owned path-text fill/stroke option planning; C++ retains objects and backend calls |
| `6ca019d65` Phase 5 text-backend execution slice | 16,264 | 9,453 | 241 | 3,746 | 6,570 | 277,467 | 5.86% | 4.67% | Prior surfaces plus Rust-owned pattern/path/normal text-backend invocation; C++ retains objects, matrices, colors, and concrete backend implementation |
| `e122a59b0` Phase 6 xref-field slice | 15,439 | 8,628 | 241 | 3,468 | 6,570 | 276,122 | 5.59% | 4.29% | Prior surfaces plus Rust-owned variable-width cross-reference field reading; C++ retains stream interpretation and cross-reference ownership |
| `9dfe83b11` Phase 6 xref-type slice | 15,488 | 8,677 | 241 | 3,479 | 6,570 | 276,197 | 5.61% | 4.31% | Prior surfaces plus Rust-owned cross-reference object-type validation; C++ retains entry interpretation and table mutation |
| `64abd58c8` Phase 6 xref-entry slice | 15,528 | 8,717 | 241 | 3,494 | 6,570 | 276,271 | 5.62% | 4.33% | Prior surfaces plus Rust-owned effective cross-reference entry-type planning; C++ retains field interpretation and table mutation |
| `112e036cf` Phase 6 xref-action slice | 15,586 | 8,775 | 241 | 3,519 | 6,570 | 276,363 | 5.64% | 4.36% | Prior surfaces plus Rust-owned cross-reference free/normal/compressed/skip action planning; C++ retains table mutation |
| `0b48bf90e` Phase 6 xref-index slice | 15,631 | 8,820 | 241 | 3,539 | 6,570 | 276,433 | 5.65% | 4.38% | Prior surfaces plus Rust-owned `/Index` start/count validation; C++ retains segment storage and object graph mutation |
| `1fe71e0eb` Phase 6 xref-range slice | 15,685 | 8,874 | 241 | 3,574 | 6,570 | 276,531 | 5.67% | 4.40% | Prior surfaces plus Rust-owned cross-reference segment byte-range planning; C++ retains spans, iteration, and object graph mutation |
| `da18dac0c` Phase 6 xref-iteration slice | 15,745 | 8,934 | 241 | 3,590 | 6,570 | 276,644 | 5.69% | 4.43% | Prior surfaces plus Rust-owned ordered cross-reference segment iteration; C++ retains entry processing and object graph mutation |
| `d8a078a25` Phase 6 xref-width slice | 15,774 | 8,963 | 241 | 3,601 | 6,570 | 276,688 | 5.70% | 4.45% | Prior surfaces plus Rust-owned `/W` signed-to-unsigned field-width conversion; C++ retains array traversal and object graph mutation |
| `cb1fb95a7` Phase 6 simple-token scan slice | 15,885 | 9,074 | 241 | 3,628 | 6,570 | 276,831 | 5.74% | 4.50% | Prior surfaces plus Rust-owned simple-PDF-token whitespace and comment scanning; C++ retains the borrowed buffer and token handling |
| `50bf3eb82` Phase 6 xref-field collection slice | 15,972 | 9,161 | 241 | 3,661 | 6,570 | 276,966 | 5.77% | 4.54% | Prior surfaces plus Rust-owned complete three-field cross-reference entry collection; C++ retains table mutation and object graph ownership |
| `0c0f0c95b` Phase 6 simple-token boundary slice | 16,155 | 9,344 | 241 | 3,691 | 6,570 | 277,191 | 5.83% | 4.63% | Prior surfaces plus Rust-owned complete `CPDF_SimpleParser::GetWord()` token-range planning; C++ retains borrowed storage and `ByteStringView` ownership |
| `1453d7000` Phase 6 object-snapshot baseline | 16,155 | 9,344 | 241 | 3,714 | 6,570 | 277,302 | 5.83% | 4.62% | Same-process C++/Rust parser selection and exact error, rebuild, trailer, and cross-reference object-map snapshots for valid and truncated streams |
| `aae60a5ed` Phase 6 corpus/fuzzer baseline | 16,155 | 9,344 | 241 | 3,714 | 6,570 | 277,278 | 5.83% | 4.62% | Versioned normal, defaulted, unknown, and truncated cross-reference corpus plus retained token-differential and public document parser fuzzer |
| `e24070b45` Phase 6 xref-mutation slice | 16,364 | 9,553 | 241 | 3,765 | 6,570 | 277,614 | 5.89% | 4.72% | Prior surfaces plus Rust-owned skip/free/normal/compressed mutation orchestration; C++ retains cross-reference map storage and object lifetimes |
| `d23c49f8c` Phase 6 xref-map-size slice | 16,463 | 9,652 | 241 | 3,782 | 6,570 | 277,756 | 5.93% | 4.77% | Prior surfaces plus Rust-owned clear/truncate/ensure-last object-map sizing orchestration; C++ retains the single map storage and object lifetimes |
| `2c63f57dc` Phase 6 xref-merge-policy slice | 16,534 | 9,723 | 241 | 3,805 | 6,570 | 277,878 | 5.95% | 4.80% | Prior surfaces plus Rust-owned overlay conflict policy and object-stream flag preservation; C++ retains the single map storage and object lifetimes |
| `b425794a2` Phase 6 xref-admission slice | 16,604 | 9,793 | 241 | 3,829 | 6,570 | 277,991 | 5.97% | 4.83% | Prior surfaces plus Rust-owned normal-generation precedence and compressed-object stream guards; C++ retains the single map storage and object lifetimes |
| `00020e5e2` Phase 6 xref-storage slice | 16,767 | 9,956 | 241 | 3,898 | 6,570 | 278,243 | 6.03% | 4.91% | Rust `BTreeMap` is the production source of truth for cross-reference entries, mutation, sizing, and overlay; C++ retains the oracle map, a lazy derived view, trailers, and PDF object lifetimes |
| `ae4185d3d` Phase 6 indirect-object-index slice | 17,133 | 10,322 | 241 | 4,099 | 6,570 | 279,067 | 6.14% | 5.07% | Rust owns indirect-object numbers, recursive parse placeholders, handle indexing, replacement, deletion, last-number state, and ordered iteration; C++ retains object values through a non-object-number-keyed `RetainPtr` registry and lazy view |
| `38e8aea3e` Phase 6 PDF-number-value slice | 17,421 | 10,610 | 241 | 4,179 | 6,570 | 279,540 | 6.23% | 5.21% | Rust owns PDF number signed/unsigned/float representation, parsing, conversion, mutation, and clone state; C++ retains exact float-to-PDF text formatting and archive writes |
| `65738ff47` Phase 6 PDF-boolean-value slice | 17,504 | 10,693 | 241 | 4,218 | 6,570 | 279,758 | 6.26% | 5.24% | Rust owns PDF boolean storage, keyword mutation, and clone state; C++ retains static text selection and archive writes |
| `545ff9e45` Phase 6 PDF-reference-value slice | 17,573 | 10,762 | 241 | 4,257 | 6,570 | 279,977 | 6.28% | 5.27% | Rust owns referenced object-number storage, mutation, and clone state; C++ retains the non-owning holder pointer and object/archive adapters |
| `30549ba0c` Phase 6 PDF-array-container slice | 17,744 | 10,933 | 241 | 4,344 | 6,570 | 280,423 | 6.33% | 5.35% | Rust owns ordered array slots and mutations; C++ retains object values in an opaque-handle registry and derives the legacy iterator view |
| `d9bd92b8b` Phase 6 PDF-dictionary-container slice | 17,908 | 11,097 | 241 | 4,439 | 6,570 | 280,871 | 6.38% | 5.42% | Rust owns binary key bytes, sorted mapping, and mutations; C++ retains object values in an opaque-handle registry and derives the pooled legacy iterator view |
| `91ad4cb4e` Phase 6 byte-string-pool foundation | 18,005 | 11,194 | 241 | 4,518 | 6,570 | 281,070 | 6.41% | 5.46% | Rust owns the binary string-to-handle interning index; C++ retains shared `ByteString` buffers in a handle registry so existing copy-on-write identity remains exact |
| `71b66ee13` Phase 6 PDF-string-value slice | 18,097 | 11,286 | 241 | 4,570 | 6,570 | 281,334 | 6.43% | 5.50% | Rust owns binary string bytes and hex-mode state; C++ retains a checked `ByteString` ABI view so pool and clone buffer-sharing behavior remains exact |
| `6717f6c26` Phase 6 PDF-name-value slice | 18,097 | 11,286 | 241 | 4,570 | 6,570 | 281,423 | 6.43% | 5.50% | Rust owns binary name bytes and mutation through the shared string-state boundary; C++ retains a checked pooled `ByteString` ABI view for native encoding and clone sharing |
| `7abc76905` Phase 6 PDF-stream-memory slice | 18,155 | 11,344 | 241 | 4,605 | 6,570 | 281,637 | 6.45% | 5.52% | Rust owns complete in-memory stream bytes and size; C++ retains file-backed stream and dictionary lifetime adapters plus encode/encrypt/archive orchestration |
| `76017a44f` Phase 6 public object-graph fuzzer gate | 18,155 | 11,344 | 241 | 4,605 | 6,570 | 281,637 | 6.45% | 5.52% | Versioned inputs execute token and public document Oracle/Candidate comparisons for exact errors, metadata, page bounds, and boundary-page geometry in normal and ASan corpus runners |
| `5525cc9a3` Phase 6 object-graph save/reload gate | 18,155 | 11,344 | 241 | 4,605 | 6,570 | 281,722 | 6.44% | 5.52% | Catalog/page key order, types, MediaBox, filtered content bytes, text, and rendering remain exact after public save/reload; all six Rust codec/object-graph embedder cases pass |
| `75380056e` Phase 6 object-slot-lifetime slice | 18,194 | 11,383 | 241 | 4,627 | 6,570 | 281,778 | 6.46% | 5.54% | Rust array, dictionary, and indirect-object states decide whether removed or replaced handles still occupy any slot; C++ retains one intrusive-pointer ABI handle without a duplicate reference-count policy |
| `0b3f209ba` Phase 7 document-page-index slice | 18,341 | 11,530 | 241 | 4,708 | 6,570 | 282,125 | 6.50% | 5.60% | Rust owns the document page object-number cache, unloaded slots, lookup, resize, insertion, removal, and membership; C++ retains page-tree traversal and tree-dictionary mutation |
| `3eac0d9c0` Phase 7 page-move-planning slice | 18,414 | 11,603 | 241 | 4,732 | 6,570 | 282,204 | 6.53% | 5.64% | Rust validates public page-move cardinality, ranges, destination, and uniqueness and produces descending deletion order; C++ retains page dictionaries, extension callbacks, and tree mutation |
| `fa444d71e` Phase 7 page-tree-count slice | 18,682 | 11,871 | 241 | 4,780 | 6,570 | 282,590 | 6.61% | 5.76% | Rust owns page-tree count traversal, active-path cycle guarding, malformed-type and count normalization, checked totals, and a bounded candidate depth; C++ retains borrowed dictionaries and the exact fallback traversal |
| `53f150f66` Phase 7 page-index-traversal slice | 18,901 | 12,090 | 241 | 4,823 | 6,570 | 282,912 | 6.68% | 5.86% | Rust owns page-object-number lookup traversal, cached-prefix skipping, count shortcuts, direct-reference lookup, and the depth bound; C++ retains borrowed dictionaries and exact fallback traversal |
| `0dfd10951` Phase 7 SDK-page-range slice | 19,041 | 12,230 | 241 | 4,851 | 6,570 | 283,118 | 6.73% | 5.92% | Rust validates and expands the public import-page range grammar with exact order and duplicates; C++ retains the public wrapper, document import, and overflow oracle fallback |
| `1ff59c226` Phase 7 page-tree-mutation slice | 19,255 | 12,444 | 241 | 4,897 | 6,570 | 283,500 | 6.79% | 6.01% | Rust selects nested insertion/deletion paths, subtree skips, and active-path cycle rejection; C++ retains dictionaries, reference writes, count updates, and the bounded fallback oracle |
| `6c858cf2e` Phase 7 SDK-NUL-output slice | 19,322 | 12,511 | 241 | 4,915 | 6,570 | 283,613 | 6.81% | 6.04% | Rust owns required-length calculation and complete NUL-terminated byte-string copies for public SDK getters; C++ retains source strings, API wrappers, and checked public length conversion |
| `44e5d54fb` Phase 7 incremental-page-traversal slice | 19,763 | 12,952 | 241 | 5,034 | 6,570 | 284,343 | 6.95% | 6.24% | Rust owns the persistent traversal stack, child cursors, next-page cursor, depth state, and cache scheduling; C++ retains dictionary access and one unordered `RetainPtr` lifetime token per active Rust stack entry |
| `85aab3743` Phase 7 public-text-search slice | 20,193 | 13,382 | 241 | 5,195 | 6,570 | 285,051 | 7.08% | 6.43% | A dedicated Rust text crate owns page/word search data, forward/backward cursors, match results, whole-word, spacing, and overlap policy; C++ retains case folding, query tokenization, and text/character index mapping |
| `6f70ec2d4` Phase 7 text-query-tokenization slice | 20,258 | 13,447 | 241 | 5,185 | 6,570 | 285,113 | 7.11% | 6.46% | Rust owns leading/trailing-space markers, collapsed separators, script-boundary splitting, and right-apostrophe preservation; C++ retains only case folding and the separately selected tokenization oracle |
| `2afa6a36f` Phase 7 text-link-extraction slice | 20,677 | 13,866 | 241 | 5,294 | 6,570 | 285,739 | 7.24% | 6.64% | Rust owns extracted web/mail URLs and ranges, multiline/hyphen joining, scheme/host/IPv6/bracket trimming, and email validation; C++ retains text geometry, public wrappers, and platform alphanumeric classification |
| `0fc9dde75` Phase 7 text-index-mapping slice | 20,795 | 13,984 | 241 | 5,337 | 6,570 | 285,959 | 7.27% | 6.69% | Rust owns the visible-text segment map and both character/text index conversions; C++ retains character classification, public wrappers, and the separately selected mapping oracle |
| `d13ca6f5d` Phase 7 text-hit-testing slice | 20,915 | 14,104 | 241 | 5,379 | 6,570 | 286,176 | 7.31% | 6.75% | Rust owns character-rectangle containment, tolerance expansion, nearest-edge selection, and tie order; C++ retains character rectangles, the synchronous rectangle callback, public wrappers, and the separately selected oracle |
| `244e6f130` Phase 7 text-selection-rectangle slice | 21,087 | 14,276 | 241 | 5,456 | 6,570 | 286,525 | 7.36% | 6.82% | Rust owns selected-rectangle grouping, normalization, union, range handling, and retained public result state; C++ retains character metadata, the synchronous callback, public wrappers, and the separately selected oracle |
| `9216b82ed` Phase 7 visible-text-range slice | 21,155 | 14,344 | 241 | 5,480 | 6,570 | 286,654 | 7.38% | 6.85% | Rust owns printable range-edge scanning and visible-buffer range calculation; C++ retains the text buffer, final substring copy, public wrapper, and the separately selected oracle |
| `f9b2e8c81` Phase 7 predicate-text-assembly slice | 21,281 | 14,470 | 241 | 5,540 | 6,570 | 286,897 | 7.42% | 6.90% | Rust owns bounded/object-selected character assembly, intervening spaces, line-transition state, and CRLF insertion; C++ retains predicate evaluation, character metadata, final `WideString` conversion, public wrappers, and the separately selected oracle |
| `7178d32c6` Phase 7 temporary-line-ordering slice | 21,466 | 14,655 | 241 | 5,643 | 6,570 | 287,282 | 7.47% | 6.98% | Rust owns duplicate-space normalization, Bidi direction carry, segment reversal/forward emission, source ordering, and RTL flags; C++ retains Unicode Bidi classification, character objects, normalization callbacks, and the separately selected oracle |
| `048b8f8ee` Phase 7 character-normalization slice | 21,707 | 14,896 | 241 | 5,754 | 6,570 | 287,715 | 7.54% | 7.08% | Rust owns control/normal classification, mirror/normalization requests, ligature expansion emissions, text-append decisions, Unicode overrides, and `CharType::kPiece`; C++ retains mirror/normalization table lookup, native character values, buffer/list writes, and the separately selected oracle |
| `a3368fe93` Phase 7 text-flow-orientation slice | 21,865 | 15,054 | 241 | 5,798 | 6,570 | 287,988 | 7.59% | 7.15% | Rust owns active text-object mask construction, page-bound clamping, occupied-span fill ratios, and horizontal/vertical flow selection; C++ retains page-object geometry, the synchronous callback, and the separately selected oracle |
| `a23c711ec` Phase 7 text-object-writing-mode slice | 21,928 | 15,117 | 241 | 5,831 | 6,570 | 288,153 | 7.61% | 7.18% | Rust owns transformed-endpoint deltas, epsilon handling, vector normalization, axis thresholds, and fallback writing-mode selection; C++ retains text-object access, native matrix transforms, and the separately selected oracle |
| `6fb298ba6` Phase 7 text-object-separator slice | 22,103 | 15,292 | 241 | 5,924 | 6,570 | 288,493 | 7.66% | 7.25% | Rust owns horizontal/vertical line-end geometry, ordered width-threshold normalization, and gap-based space insertion; C++ retains text/font access, matrix transforms, hyphen policy, native output mutation, and the separately selected oracle |
| `dc960e660` Phase 7 text-hyphen-joining slice | 22,195 | 15,384 | 241 | 5,966 | 6,570 | 288,650 | 7.69% | 7.29% | Rust owns trailing-space backtracking, soft/ASCII hyphen recognition, word-continuation policy, and `CharType::kPiece` fallback; C++ retains native buffers, previous-character metadata, platform Unicode predicates, and the separately selected oracle |
| `77e63c362` Phase 7 generated-text-separator slice | 22,344 | 15,533 | 241 | 6,029 | 6,570 | 288,911 | 7.73% | 7.36% | Rust owns none/space/line-break/hyphen action selection, single-hyphen suppression, and trailing-space trim counts; C++ retains Unicode lookup, line closing, character construction, native buffer mutation, and the separately selected oracle |
| `b3076bf2d` Phase 7 text-object-base-spacing slice | 22,439 | 15,628 | 241 | 6,078 | 6,570 | 289,068 | 7.76% | 7.40% | Rust owns per-object kerning scanning, minimum base spacing, negative/two-item suppression, and signed character-space adjustment; C++ retains text state, native matrix distance transforms, kerning storage, and the separately selected oracle |
| `ec21cd640` Phase 7 marked-content slice | 22,693 | 15,882 | 241 | 6,217 | 6,570 | 289,558 | 7.84% | 7.50% | Rust owns ActualText pass/done/delay selection, printable filtering, control replacement, RTL/LTR box subdivision, and retained emissions; C++ retains marked-content dictionaries, object/matrix lifetimes, platform printability, native `CharInfo` construction, and the separately selected oracle |
| `1c8ed33b2` Phase 7 text-space-threshold slice | 22,753 | 15,942 | 241 | 6,247 | 6,570 | 289,674 | 7.85% | 7.53% | Rust owns space-glyph threshold calculation, the one-third cap, halving, fallback-width request, and 300/500/700 normalization; C++ retains font character-code/width lookup, the synchronous fallback callback, and the separately selected oracle |
| `6d6566673` Phase 7 text-item-spacing slice | 22,866 | 16,055 | 241 | 6,298 | 6,570 | 289,902 | 7.89% | 7.58% | Rust owns per-item kerning/base-space composition, prior-text/space suppression, threshold request, and generated-space decision; C++ retains font and text buffers, native character creation, validated writes, and the separately selected oracle |
| `f19b313b9` Phase 7 duplicate-text-object slice | 23,059 | 16,248 | 241 | 6,365 | 6,570 | 290,248 | 7.94% | 7.66% | Rust owns empty/overlap rectangle policy, width/font/item equality, character comparison, and position tolerances; C++ retains text-object/font access, the bounded prior-object scan, synchronous item callback, and the separately selected oracle |
| `d822f622b` Phase 7 generated-character-origin slice | 23,125 | 16,314 | 241 | 6,403 | 6,570 | 290,377 | 7.96% | 7.69% | Rust owns prior-width admission, text-object/box font-size selection, zero-size fallback, and origin advancement; C++ retains prior-character/font access, matrix and native `CharInfo` construction, and the separately selected oracle |
| `f0a4139db` Phase 7 duplicate-character slice | 23,243 | 16,432 | 241 | 6,455 | 6,570 | 290,615 | 8.00% | 7.73% | Rust owns the seven-character reverse scan, code/font/origin equality, threshold comparison, suppression, and first-item space-trim action; C++ retains character/font lifetimes, buffer mutation, and the separately selected oracle |
| `ddf10ee58` Phase 7 text-object-grouping slice | 23,393 | 16,582 | 241 | 6,504 | 6,570 | 290,954 | 8.04% | 7.79% | Rust owns zero-width/duplicate/empty-item actions, line-change thresholding, flush/append selection, and stable horizontal insertion index; C++ retains object/font/matrix lifetimes, transformed geometry callbacks, native vector mutation, and the separately selected oracle |
| `01e74216d` Phase 7 public-redaction slice | 23,638 | 16,827 | 241 | 6,591 | 6,570 | 291,422 | 8.11% | 7.90% | Rust owns public redaction rectangle validation, intersection/containment policy, supported-object checks, atomic removal-index planning, and result status selection; C++ retains page/object lifetimes, applies the validated mutation, and preserves the complete separately selected oracle |
| `764015ee2` Phase 7 indexed-object-insertion slice | 23,724 | 16,913 | 241 | 6,630 | 6,570 | 291,637 | 8.13% | 7.93% | Rust owns insertion-index validation, lazy neighbor lookup, content-stream inheritance, and dirty-stream selection; C++ retains page-object lifetimes, applies the validated deque/stream mutation, and preserves the complete separately selected oracle |
| `aa090f3b1` Phase 7 page-object-matrix slice | 23,860 | 17,049 | 241 | 6,687 | 6,570 | 291,971 | 8.17% | 7.98% | Rust owns supported-object routing for public matrix access, exact image dirty-state selection, and rotated text/image QuadPoints geometry; C++ retains page-object lifetimes, native matrix storage/setters, and the separately selected oracle |
| `feacb5472` Phase 7 page-object-removal slice | 23,935 | 17,124 | 241 | 6,724 | 6,570 | 292,171 | 8.19% | 8.01% | Rust owns stable handle lookup, removal-index selection, non-member results, and dirty content-stream selection; C++ retains page-object lifetimes, unique ownership transfer, deque erasure, dirty-set mutation, and the separately selected oracle |
| `c7e2eadb6` Phase 7 page-object-active-state slice | 24,008 | 17,197 | 241 | 6,771 | 6,570 | 292,373 | 8.21% | 8.04% | Rust owns active-state update and exact dirty decision plus active-object counting orchestration; C++ retains page-object storage/lifetimes, borrowed callbacks, and the separately selected oracle |
| `4d64144fa` Phase 7 annotation-geometry slice | 24,084 | 17,273 | 241 | 6,794 | 6,570 | 292,547 | 8.23% | 8.07% | Rust owns public annotation rectangle corner transformation/reduction and signed page-rotation planning; C++ retains annotation/dictionary lifetimes, native writes, and the separately selected oracle |
| `0649cbf37` Phase 7 public-action-routing slice | 24,143 | 17,332 | 241 | 6,816 | 6,570 | 292,699 | 8.25% | 8.09% | Rust owns internal-to-public action type mapping and destination/file/URI capability routing; C++ retains action dictionaries, destinations, path/URI byte storage, public copying, and the separately selected oracle |
| `9bf42fcc4` Phase 7 public-destination-policy slice | 24,311 | 17,500 | 241 | 6,880 | 6,570 | 293,025 | 8.30% | 8.16% | Rust owns destination zoom-mode mapping, fit-parameter bounds, and XYZ validity/presence/zero-zoom policy; C++ retains destination arrays and name/number lifetimes, conditional output writes, and the separately selected oracle |
| `f9ec47468` Phase 7 public-bookmark-traversal slice | 24,431 | 17,620 | 241 | 6,910 | 6,570 | 293,244 | 8.33% | 8.21% | Rust owns depth-first child/sibling traversal order, visited-set state, and cycle guarding; C++ retains bookmark-tree, dictionary, and title lifetimes, supplies borrowed comparison/navigation callbacks, and preserves the separately selected oracle |
| `4c170af21` Phase 7 public-page-label-formatting slice | 24,524 | 17,713 | 241 | 6,937 | 6,570 | 293,413 | 8.36% | 8.25% | Rust owns decimal, upper/lower Roman, upper/lower repeated-letter, modulo, and unknown-style formatting; C++ retains number-tree, label-dictionary, prefix, and string lifetimes plus the malformed-negative fallback oracle |
| `99e2c736d` Phase 7 public-link-enumeration slice | 24,581 | 17,770 | 241 | 6,969 | 6,570 | 293,570 | 8.37% | 8.27% | Rust owns public annotation-cursor normalization, forward scan, first-link selection, and miss/error state; C++ retains annotation arrays, dictionary/subtype access, selected handles, and the separately selected oracle loop |
| `8f97a67d8` Phase 7 document-number-tree slice | 24,924 | 18,113 | 241 | 7,054 | 6,570 | 294,097 | 8.47% | 8.41% | Rust owns exact and greatest-key-at-most traversal, limits pruning, forward/reverse ordering, and cycle guarding; C++ retains dictionaries, arrays, object lifetimes, borrowed callbacks, and the separately selected oracle traversals |
| `b15eef798` Phase 7 public-destination-page slice | 24,983 | 18,172 | 241 | 7,085 | 6,570 | 294,209 | 8.49% | 8.44% | Rust owns numeric/dictionary/invalid destination-target routing and callback admission; C++ retains destination objects, document page indexing, borrowed lookup, and the separately selected oracle branches |
| `44894c359` Phase 7 document-name-tree-index slice | 25,202 | 18,391 | 241 | 7,153 | 6,570 | 294,599 | 8.55% | 8.52% | Rust owns name-tree pair counting, depth-first index traversal, cumulative leaf offsets, the depth bound, and count-cycle guarding; C++ retains dictionaries, arrays, decoded names, values, borrowed callbacks, and the separately selected oracle traversals |
| `7df1198ab` Phase 7 public-link-hit-test slice | 25,308 | 18,497 | 241 | 7,188 | 6,570 | 294,805 | 8.58% | 8.57% | Rust owns reverse z-order scanning, rectangle normalization, inclusive containment, topmost selection, and miss/error state; C++ retains the per-page link cache, dictionaries, rectangle extraction, selected handles, and the separately selected oracle loop |

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

Alpha-mask cloning retains C++ allocation and the abstract `CFX_DIBBase`
scanline source while Rust extracts every active BGRA alpha byte once per row.
The row boundary supports lazy or otherwise non-contiguous DIB sources and
intentionally leaves the newly allocated 8-bpp destination padding untouched.

Whole-bitmap clearing preserves the format-specific treatment of row padding:
1-/8-bpp images and equal-channel BGR fill padding, while non-gray BGR and
four-component formats write active pixels only. The focused differential
matrix covers seven formats and transparent, gray, and colored ARGB values.

Solid rectangle compositing now clips in C++ and crosses the FFI boundary once
per bitmap. Rust owns opaque and partial-alpha updates for every retained
1-/8-/24-/32-bpp format, including custom 1-bpp palettes, BGRx's forced opaque
fourth byte, and BGRA alpha union. The 96-case differential matrix also locks
down the historical 1-bpp left/right boundary-byte masks, including their
updates outside the logical pixel interval.

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

Bitmap clipping retains C++ rectangle intersection, allocation, palette, and
abstract scanline access while Rust owns each aligned byte copy or unaligned
1-bpp native-word shift. Before activation, the retained oracle was bounded so
the optional word beyond the right scanline edge is defined as zero rather than
read out of bounds. Thirty-six differential cases cover every retained format,
default and custom palettes, full clips, offsets, and negative intersections.

Whole-bitmap copying likewise retains C++ allocation, palette ownership, and
abstract source scanlines. Rust copies each complete row, including historical
padding bytes, through a non-overlapping borrowed-span boundary. Nine focused
cases cover every retained format plus custom 1-bpp and 8-bpp palettes.

The stretch engine still owns its variable-sized table storage in C++, while
Rust now calculates the layout, enforces the 512 MiB cap, and fills every
source interval and 16.16 weight. The two-pass FFI first sizes and then fills
the caller-owned byte table without Rust allocation. Forty exact differential
configurations cover nearest, bilinear, area, no-smoothing, clipped-source,
negative-destination mirror, zero-length, and invalid-range behavior.

Rust also owns the complete horizontal scanline transform once per source row.
The borrowed-span boundary covers the six retained transform methods: 1-bpp
mask expansion, 1-bpp RGB expansion, 8-bpp copy, indexed palette conversion,
opaque BGR/BGRx filtering, and alpha-weighted BGRA filtering. A same-process
`StretchTo` corpus compares 108 combinations of all retained source formats,
default and custom palettes, enlargement, reduction, horizontal and vertical
mirroring, nearest, no-smoothing, and bilinear options byte for byte against
the C++ reference.

The second pass now also runs once per destination row in Rust. It filters the
intermediate scalar and BGR/BGRx values, preserves untouched X bytes, and
unpremultiplies filtered BGRA colors with the reference's integer rounding.
Zero-alpha rows retain their existing RGB bytes while setting alpha to zero.
C++ continues to own the intermediate/output buffers, scanline composition,
pause protocol, and retained fallback implementation.

Bitmap transposition keeps result allocation, palette propagation, and
abstract source scanlines in C++. Rust maps each complete source row into the
transposed destination for packed 1-bpp and 8-/24-/32-bpp pixels, including
independent X/Y mirroring and the retained all-ones initialization for 1-bpp
padding. Thirty-six same-process cases cover every retained format, default
and custom palettes, and all four flip combinations.

Non-axis-aligned alpha-image transforms now pass the six C++-rounded 8.8 fixed
matrix coefficients into Rust. Rust owns destination coordinate mapping,
inclusive edge clamping, residual weights, and four-tap bilinear sampling over
the complete mask buffer. Four public `TransformTo` matrices cover positive
and negative shear, reflection, translation, exact result offsets, and full
output bytes; a native 2-by-2 case locks in the retained double `/256` integer
rounding. Color and indexed transform sampling remain C++-owned in this slice.

Indexed and grayscale transforms reuse the same Rust-owned fixed-matrix
sampler, then resolve the interpolated byte through the C++-prepared 256-entry
ARGB palette into native-order destination pixels. Sixteen public cases cover
1-bpp and 8-bpp sources, default and custom palettes, and the same four skewed
and reflected matrices. Direct native coverage locks palette byte order and
the interpolated index rounding. Color transform sampling remains C++-owned.

Color transforms complete the same sampling boundary for BGR, BGRx, BGRA, and
the retained raw four-channel CMYK branch. Rust interpolates each channel,
forces opaque alpha for non-alpha sources, and preserves interpolated BGRA or
CMYK fourth bytes. Twelve public format/matrix cases compare exact result
offsets, geometry, pitch, and complete buffers; native cases pin channel order,
opaque-alpha insertion, and per-channel fixed-point rounding. C++ still owns
transform classification, intermediate stretching, allocation, and objects.

Stretching a custom 1-bpp RGB source now builds its 256-entry opaque gradient
palette in Rust from the two C++-owned endpoint colors. The implementation
preserves signed integer division toward zero for descending channels, uses no
Rust allocation, and is covered natively plus through the existing 108-case
public `StretchTo` differential matrix.

The Windows-only `FlipImage` entry now delegates each copied or horizontally
reversed row to Rust; C++ still selects the vertically mirrored destination
row, owns allocation, and propagates palettes. The platform-neutral native
test covers packed 1-bpp and 8-/24-/32-bpp reversal plus the no-X-flip padding
copy contract. The Windows public entry remains guarded by its existing build
flag and retained C++ fallback.

## Phase 1 retained C++ inventory

Phase 1 leaves no platform-independent candidate pixel, color, blend,
compositing, stretch, or bilinear-sampling loop in `core/fxge/dib` without an
active Rust boundary. The following C++ remains intentionally:

- `CFX_DIBBase`, `CFX_DIBitmap`, `CFX_BitmapStorer`, and
  `CFX_ImageStretcher` retain object lifetime, allocation, palette and scanline
  storage, pitch-bearing spans, composer callbacks, and pause/progressive
  state.
- `CFX_ImageTransformer` retains transform classification, matrix inversion,
  result/clip object construction, and selection of the stretch, rotate, or
  arbitrary-transform route. Rust owns the arbitrary route's fixed-matrix
  coordinate mapping and every sampling loop.
- AGG, FreeType, public C/C++ declarations, native pixel structs, and the
  Windows `FlipImage` entry remain native system/backend boundaries. The
  Windows entry delegates row behavior to the same platform-neutral Rust core.
- The former C++ algorithms in `cfx_scanlinecompositor.cpp`,
  `cfx_cmyk_to_srgb.cpp`, `cfx_dibbase.cpp`, `cfx_dibitmap.cpp`,
  `cstretchengine.cpp`, and `cfx_imagetransformer.cpp` remain as differential
  oracles and validation-failure fallbacks. The test-only selector reaches
  them in the same process; production selects Rust first.

The phase gate builds `pdfium_light_validation` and `pdfium_all`, including
`pdfium_diff` and every retained fuzzer source. It runs 1,056 unit tests, all
of which pass. The full embedder suite adds five Rust renderer corpus cases;
491 of 542 tests pass and the remaining 51 static macOS golden mismatches are
the exact same named baseline set reproduced on `origin/main` (486 of 537).
All five zero-tolerance same-process Rust/C++ renderer comparisons pass.
Phase 2 therefore starts at render-command planning and page-rendering
orchestration rather than reopening DIB pixel behavior.

## Phase 2 render request planning

The production candidate now asks Rust to turn the public render flags,
color-scheme presence, and restore requirement into a compact request plan.
Rust owns annotation inclusion, LCD/native-text choices, grayscale versus
forced color, fill-to-stroke eligibility, image-cache and halftone policy,
print/view usage, the three smoothing controls, and device restoration. C++
applies that plan to its existing `CPDF_RenderOptions`, page, annotation,
device, and render-context objects; the reference path still derives the same
values directly from flags.

C++ `static_assert`s pin every duplicated public flag value at the FFI
boundary. Two native Rust tests cover the complete bitset and the rule that
fill-to-stroke requires a color scheme. Fourteen same-process renderer cases
exercise each independently observable flag plus text, paths, images,
transparency, and annotations with exact bitmap parity.

The next slice moves page-object type dispatch into the same core render crate.
Rust maps the five pinned `CPDF_PageObject::Type` values to text, path, image,
shading, and form commands; C++ still owns the typed objects and executes the
existing render methods. Invalid FFI values fail closed to the retained C++
mapping, and the test-only render scope switches both the outer SDK plan and
this inner core dispatch so the candidate and oracle remain genuinely
independent in the same process. Two additional native tests cover all five
commands, invalid values, null output, and unchanged output on failure; the
fourteen zero-tolerance renderer cases remain byte-identical.

Object-list traversal now asks Rust whether each entry must stop traversal,
skip, or enter rendering. The planner preserves the original evaluation order:
stop-object identity wins before null/activity checks, inactive entries do not
read bounds, and clipping uses the same strict float comparisons (including
touching and NaN behavior). It performs one constant-size, allocation-free FFI
call per visited entry; object ownership, iteration, matrices, visibility,
transparency, and rendering remain C++-owned at this boundary.

The differential harness records every stop/skip/render decision and every
text/path/image/shading/form dispatch in order. Each of the fourteen corpus
cases now requires an exact non-empty Candidate/Oracle trace in addition to
identical bitmap dimensions, format, stride, and bytes. Four new native tests
cover stop precedence, missing/inactive entries, all four outside directions,
touching bounds, NaN comparison semantics, and null FFI output; 8/8 native
render tests pass.

Render-layer orchestration completes Phase 2. Rust plans optional render
options and last-matrix setup, cache optimization and stopped completion, then
owns the ordered layer loop itself. Each layer invokes one borrowed C++
callback containing the retained `CPDF_RenderStatus`, page-object holder,
device, matrix multiplication, cache object, and concrete render methods. The
loop adds no allocation, preserves cache cleanup before the stop decision, and
stops immediately when the callback reports the retained status.

Three native plan tests cover every setup/completion bit combination and null
outputs; two loop tests prove ascending indices, early stop, and rejection of
null context or callback. All 13 native render tests pass. The fourteen
same-process corpus cases compare non-empty layer, object-list, and object-type
traces as well as exact bitmap dimensions, format, stride, and bytes.

The intentionally retained C++ after Phase 2 is now explicit:

- public C entry points, page/device/context objects, annotation construction,
  save/restore, and clipping-device calls;
- the C++ object graph, page-object holders, matrices, render options/status,
  cache storage, visibility, clip-path and transparency evaluation;
- concrete text, path, image, shading, and form render methods plus their
  background fallback, with AGG and FreeType unchanged;
- the narrow callback/ABI adapter, test-only trace collector, and C++ planning
  functions retained as the same-process oracle/fallback.

The Phase 2 broad gate builds `pdfium_all` and
`pdfium_light_validation`, passes all 1,056 unit tests, all 13 native Rust
render tests, and all 14 zero-tolerance bitmap-plus-trace cases. The full
embedder run passes 500 of 551 tests; its 51 failed static `_agg_mac.png`
goldens are the exact same named baseline set recorded at Phase 1, while the
nine additional Phase 2 corpus cases all pass. Phase 3 therefore starts at
path processing and the narrow AGG adapter boundary.

## Phase 3 path processing and AGG boundary

The first path slice moves the paint decision immediately after retained
pattern handling into Rust. It preserves no-fill/even-odd/winding modes and
stroke state, completes no-paint objects without touching colors or the
backend, and converts a nonempty forced-color fill to stroke only when the
existing option is enabled. The adapter pins all three fill enum values,
rejects unknown plan bits and invalid fill values, and falls back to the
retained C++ decision before color, transfer-function, matrix, or device work.

Three native tests cover the fill/stroke matrix, both forced-color conversion
preconditions, invalid fill values, unchanged output on failure, and null
output. The render trace now includes the exact paint plan. Four additional
path-focused cases cover regular and aliased rectangle sets, direct rectangle
paths, clip paths, and a form-owned path; all 16 native render tests and all 18
zero-tolerance bitmap-plus-trace corpus cases pass. Path colors, transfer
functions, matrix validation, graph-state planning, `CFX_RenderDevice`, and
the AGG raster backend remain C++-owned for the next sub-slices.

Path matrix availability now runs in Rust after the retained fill/stroke color
and transfer-function work, preserving the original observable ordering. The
four linear matrix components use the same axis-pair rules as the C++ oracle;
translation remains irrelevant. Native cases cover identity, swapped axes,
degenerate rows/columns, shear, signed zero, infinity, all-NaN input, and null
output. The availability result is part of the exact render trace. All 18
native render/path tests and all 18 bitmap-plus-trace corpus cases pass.

The complete `CFX_FillRenderOptions` subset used by PDF path drawing is now
planned in Rust. Fill rule, fill-only rectangle AA, no-smooth aliasing,
graph-state stroke adjustment, stroke mode, and Type3 text mode cross the
boundary as a validated compact bitset; all unrelated option fields remain at
their retained defaults. Native tests cover every flag, the fill-only AA rule,
invalid fill values, unchanged output on failure, and null output. The exact
options are recorded in the render trace. All 21 native render/path tests and
all 18 bitmap-plus-trace cases pass.

The first production AGG boundary now delegates dash-pattern applicability
from `RasterizeStroke()` to Rust once per stroke. Rust accepts a borrowed dash
span, rejects non-finite dash entries, clamps negative entries only while
forming the cycle sum, and preserves the exact `0.1` device-pixel threshold.
The original comparison also applies dashes when the scale product is NaN;
Rust preserves that behavior explicitly with `partial_cmp()`. Empty patterns,
invalid boundary inputs, and adapter validation failures retain the C++ oracle
and fallback. A dedicated test selector and trace keep the reference render on
the C++ decision while the candidate render uses Rust. All 3 native AGG tests,
all 21 native render/path tests, and all 18 zero-tolerance bitmap-plus-core-and-
AGG-trace corpus cases pass.

AGG stroke setup now also receives a compact Rust plan for line cap, line join,
effective width, and miter limit. The three pinned PDF cap and join values map
to AGG's butt/round/square and miter-revert/round/bevel modes; unknown wire
values fall closed to the historical defaults. Rust preserves the matrix-unit
width floor and the first-operand behavior of `std::max()`, including a NaN
line width. C++ still owns the graph state, matrix unit calculation inputs,
AGG converter objects, and the complete oracle/fallback. The exact scalar plan
is included in the independent AGG trace. All 6 native AGG tests, all 21 native
render/path tests, and all 18 zero-tolerance bitmap-plus-core-and-AGG-trace
corpus cases pass.

Applied dash patterns are now normalized in one Rust call. Rust replaces dash
lengths at or below `0.000001` with `0.1`, applies the retained signed scale
and absolute-value order, and scales the dash phase without allocating. Each
normalized value is borrowed back into the existing C++ AGG converter through
a callback, so the production path adds neither a Rust allocation nor a new
C++ temporary vector. Boundary rejection occurs before any callback, allowing
the untouched converter to use the complete C++ fallback. A dedicated dashed-
line fixture exercises the active route, and the exact AGG trace includes each
normalized value and phase. All 9 native AGG tests, all 21 native render/path
tests, and all 19 zero-tolerance bitmap-plus-core-and-AGG-trace corpus cases
pass.

The AGG `DrawPath()` branch plan is now Rust-owned. Fill type and color decide
whether to construct a fill rasterizer and whether it uses even-odd or non-zero
winding. Graph-state presence, stroke alpha, and the zero-area flag select no
stroke, the direct zero-area route, or the matrix-normalized stroke route. The
same fill-rule plan is used for non-rectangular clip paths. C++ retains bitmap,
graph-state, matrix, rasterizer, scanline, clip-region, and AGG backend
ownership, plus the complete plan oracle/fallback. Native tests cover fill
suppression, both fill rules, unknown wire values, all three stroke modes, and
null output. The exact plan is part of the AGG trace. All 15 native AGG tests,
all 21 native render/path tests, and all 19 zero-tolerance bitmap-plus-core-and-
AGG-trace corpus cases pass.

The final stroke-specific path math now runs in Rust. An absent object matrix
produces two identity matrices and unit scale. Otherwise Rust preserves the
first-operand `std::max(abs(a), abs(b))` scale semantics, constructs the
normalized AGG stroke matrix, applies the retained inverse rules, and
multiplies the original matrix into the path-emission matrix in the same
operation order. C++ retains the matrix objects passed to AGG and the complete
oracle/fallback. Native tests cover identity, regular scale/translation,
singular division behavior, first-operand NaN, and null output. Both matrices
and the scale are included bit-for-bit in the AGG trace. All 18 native AGG
tests, all 21 native render/path tests, and all 19 zero-tolerance bitmap-plus-
core-and-AGG-trace corpus cases pass.

`BuildAggPath()` now delegates path iteration and command formation to Rust.
Rust borrows points individually from their C++ owner, applies the optional
matrix and the retained `[-32000, 32000]` hard clip, preserves the degenerate
single-point line adjustment, groups complete cubic Bezier quartets, and emits
Move, Line, Bezier, and Close commands through a second borrowed callback.
Neither side creates an intermediate point or command vector. C++ retains the
`CFX_Path`, `agg::path_storage`, `agg::curve4`, and a complete isolated oracle;
an invalid command discards the separate candidate storage before fallback.
Native tests cover command ordering, transforms, clipping, Bezier grouping,
close behavior, and missing callbacks. The exact command stream and float bits
are part of the AGG trace. All 12 native AGG tests, all 21 native render/path
tests, and all 19 zero-tolerance bitmap-plus-core-and-AGG-trace corpus cases
pass.

The intentionally retained C++ after Phase 3 is now explicit:

- page/path/graph-state/matrix object ownership, pattern and transfer-function
  handling, color resolution, the rectangle clip fast path, clip-region and
  bitmap lifetime, and concrete `CFX_RenderDevice` calls;
- AGG `path_storage`, curve, dash, stroke, rasterizer, scanline, renderer, and
  pixel-format objects plus scanline compositing, which remain the pinned
  native raster backend required by the compatibility contract;
- the narrow borrowed point/command/dash callbacks, validated POD adapters,
  test-only selectors and traces, and complete C++ plan/math oracles used only
  for differential validation and boundary fallback.

The Phase 3 broad gate builds `pdfium_all` and `pdfium_light_validation`, passes
all 1,056 unit tests, all 21 native render/path tests, all 18 native AGG tests,
and all 19 zero-tolerance bitmap-plus-core-and-AGG-trace corpus cases. The full
branch embedder run passes 505 of 556 tests; `origin/main` passes 486 of 537.
Both fail the exact same 51 named static macOS golden tests, while all 19 added
differential cases pass. Phase 4 therefore starts at glyph planning, cache
behavior, and the narrow FreeType adapter boundary.

## Phase 4 glyph planning, caches, and FreeType adapter boundary

The first Phase 4 slice moves glyph bitmap cache-key shape planning into Rust.
C++ retains the existing matrix quantization, font and substitution metadata,
cache containers, glyph loading, bitmap ownership, and FreeType calls. Rust
receives only scalar words, selects the exact 6-, 7-, 9-, or 10-word layout,
and writes it into a caller-owned fixed-capacity buffer without allocation.
Signed weight and italic-angle values preserve their two's-complement word
representation, and native-text keys retain the terminal discriminator.

The production route uses the Rust candidate. A test-only selector keeps the
complete C++ key builder available as the same-process oracle and records every
cache-key word in the renderer trace. The `HelloWorldNoNativeText` fixture
forces the bitmap-glyph route, proving that the trace is non-empty; all 19
renderer corpus cases remain byte- and trace-exact. Three native Rust tests
cover every key shape, signed inputs, insufficient capacity, null output, and
the invariant that rejected calls do not mutate caller outputs. Rustfmt,
Clippy with warnings denied, native GN tests, and Rust unit/doc tests pass.

The next Phase 4 boundary is glyph placement and cache lookup/action planning.
Concrete cache storage, font faces, glyph bitmaps, paths, and the FreeType
backend remain intentionally C++-owned until their own differential slices.

The second slice moves `TextGlyphPos::GetOrigin()` into Rust. Its checked
left/top arithmetic combines the planned glyph origin, bitmap bearings, and
caller offset and rejects every signed 32-bit overflow before composition.
The same function feeds both glyph bounding boxes and mono/color draw loops.
C++ retains the glyph object and bitmap lifetime; an invalid Rust boundary
falls back to the complete checked C++ oracle.

Six native glyph tests now cover cache keys plus signed placement, overflow at
all four checked operations, null outputs, and no-mutation rejection. The exact
same-process glyph trace records validity and both origin coordinates for every
placement, and `HelloWorldNoNativeText` explicitly proves that origin events
are reached. All 19 bitmap-plus-trace renderer cases pass. Device-origin
rounding, glyph bounding-box aggregation, cache lookup/action planning, and the
FreeType adapter remain the next Phase 4 boundaries.

The third slice moves device-origin rounding in `DrawNormalText()` into Rust.
Normal text uses PDFium's NaN-to-zero, saturating, half-away-from-zero contract;
LCD x coordinates retain floor conversion. Rust rejects non-finite or
out-of-range LCD x values before conversion so the existing platform C++
expression remains the fallback oracle for inputs where its cast contract is
implementation-defined. Transform ownership and glyph loading remain C++.

Nine native glyph tests cover positive and negative half values, tiny and
extreme floats, NaN, LCD floor behavior, rejected LCD inputs, and null-output
no-mutation. The glyph trace records both float bit patterns, LCD mode, and
planned integer coordinates; `HelloWorldNoNativeText` proves the event is
reached, and all 19 exact renderer cases pass. Glyph bounding-box aggregation,
cache lookup/action planning, and the FreeType boundary remain next.

The fourth slice moves complete glyph bitmap bounding-box aggregation into a
Rust-owned borrowed iteration. A narrow C++ callback exposes only per-glyph
validity, checked origin, and bitmap dimensions; Rust applies LCD width
division, skips missing glyphs and overflowing right/bottom edges, and folds
the remaining rectangles with exact min/max ordering. The callback is
synchronous and allocation-free, while glyph vectors, bitmaps, and lifetimes
remain C++-owned. Boundary failure retains the complete C++ loop as fallback.

Thirteen native glyph tests now cover mixed missing/valid entries, LCD widths,
both edge-overflow directions, callback iteration, missing callback, and
no-mutation outputs in addition to all prior contracts. The structured trace
records the final rectangle and `HelloWorldNoNativeText` proves reachability;
all 19 bitmap-plus-trace renderer cases pass. Cache lookup/action planning and
the FreeType boundary remain the next Phase 4 work.

The fifth slice moves bitmap-glyph cache action selection into Rust. After the
optional C++ native-cache probe, Rust selects invalid-glyph rejection, lookup
with the requested key, return of the native cached bitmap, or regeneration of
the non-native key with `native_text` disabled. C++ retains key/container and
bitmap ownership, performs map operations, and executes the selected action;
the original decision function remains the test oracle and boundary fallback.

Fifteen native glyph tests cover all four actions and null-output rejection in
addition to the existing placement and bounds contracts. The structured trace
records validity, native mode, probe result, and selected action;
`HelloWorldNoNativeText` proves the ordinary lookup route is active, and all 19
exact renderer cases pass. The path/width cache-key slice follows.

The sixth slice moves glyph-path and glyph-width cache key planning into Rust.
Rust preserves the exact glyph-index, destination-width, weight, italic-angle,
and vertical-key shape. Substitution-only metadata is zeroed when no
substitution exists, matching the retained C++ tuple construction. C++ keeps
the `std::map` containers, font calls, path objects, and their lifetimes; any
failed Rust boundary call falls back to the unchanged tuple construction.

Twenty-one native Rust tests now cover the complete scalar key shapes, signed
metadata, absent substitutions, and null-output rejection without caller
mutation. The narrow FreeType adapter boundary follows.

The seventh and final Phase 4 slice moves FreeType glyph-load planning into
Rust. Rust selects no-hinting, pedantic, and render-retry behavior from the
existing TT/OT and tricky-font properties. C++ maps that plan to FreeType
constants and continues to own the face, all FreeType calls, transforms,
emboldening, bitmap/path allocation, and backend error handling. The existing
C++ load planner remains the fallback on an invalid boundary result.

The renderer trace now records the complete render/path load plan, including
the retry decision. The 21 native Rust tests cover every render/path hinting
combination and the FFI's no-mutation rejection behavior; the exact renderer
corpus asserts that a real glyph render reaches the new trace event. Phase 4
is complete. Phase 5 begins with page/render and font-facing PDF logic while
the C++ parser and object graph remain the oracle.

## Phase 5 PDF page/render and font-facing logic

The first Phase 5 slice moves PDF text dispatch into Rust. Rust receives only
the presence of character codes, the PDF rendering mode, Type 3 and face
availability, and whether a clipping path is already supplied. It selects the
unchanged empty/invisible and clip exits, Type 3 dispatch, and normal
fill/stroke/clip behavior. C++ retains color/pattern resolution, text and
device matrices, font objects, graph state, and the concrete normal- and
path-text backend calls. Unsupported modes leave the C++ path in control.

The native Rust render test target has 23 tests covering every observable
dispatch shape and FFI rejection without output mutation. The common local
gate passes. The full GN renderer corpus remains the required exact-bitmap
evidence before this slice can be released from the complete checkout.

The second Phase 5 slice moves the active text-pattern decision into Rust.
Only an active fill or stroke color may force the existing pattern route;
inactive pattern colors are ignored exactly as before. C++ continues to
resolve concrete ARGB values and invokes `DrawTextPathWithPattern()`. The
native test target now covers that active-paint invariant, while the common
local gate remains green.

The third Phase 5 slice applies the same Rust matrix-availability rule already
used for paths before any text backend invocation. It preserves signed zero,
infinity, and NaN behavior exactly; C++ still constructs, composes, and passes
the text matrix to the retained backend. The same-process trace records the
availability result in both reference and candidate paths.

The fourth Phase 5 slice moves normal-versus-path text backend routing into
Rust: clipping or stroking selects the existing path-text call; otherwise the
normal-text call is used. C++ retains every call argument, device-matrix
adjustment, and backend operation. The native Rust test target covers all four
clip/stroke combinations and the common local gate passes.

The fifth Phase 5 slice moves the stroked-text CTM-adjustment decision into
Rust. Only stroked text with a non-identity horizontal or vertical scale takes
the existing adjustment path. C++ still constructs the CTM, computes its
inverse, composes the device matrix, and invokes the retained backend. Native
tests include identity, non-stroke, signed-zero, and NaN scale inputs.

The sixth Phase 5 slice moves the path-text fill options into Rust. Rust
preserves the special combined fill-and-stroke flags plus stroke adjustment
and text smoothing. C++ keeps the text object and render options, and still
calls `DrawTextPath()` with the planned result. The Rust unit suite covers
stroke-only, combined, and independent adjustment/smoothing cases.

The seventh Phase 5 slice moves pattern, path, or normal text-backend execution
into Rust. Rust chooses the command with pattern precedence and invokes exactly
one synchronous C++ callback, returning the concrete backend result unchanged.
C++ continues to own text/font objects, matrices, color resolution, device
state, path preparation, and the concrete renderer calls. Twenty-nine native
render tests cover command priority, callback result propagation, and invalid
boundaries without output mutation.

The full differential gate opens a fresh document for each Oracle/Candidate
render so glyph and font caches cannot leak from the first run into the second.
All 19 renderer cases match bitmap dimensions, format, stride, bytes, and every
core/AGG/glyph/backend trace; `pdfium_all` builds and all 1,057 unit tests pass.

This completes the Phase 5 boundary. The retained C++ text objects, fonts,
matrices, colors, device state, and concrete AGG/FreeType backend calls are the
designated native backends, not parallel planning state. Exact bitmap and trace
parity covers every activated route before the ledger advances to Phase 6.

## Phase 6 parser and PDF object graph

The first Phase 6 slice moves variable-width big-endian cross-reference stream
field reading into Rust. Rust accepts a borrowed byte span and preserves the
original unsigned 32-bit wraparound semantics even for malformed over-wide
fields. C++ retains cross-reference stream segmentation, field widths, object
entry interpretation, bounds checks, and all cross-reference table ownership.
An invalid FFI boundary retains the original C++ reader.

Two native Rust tests cover empty through five-byte fields, wraparound, and
null-input rejection without output mutation. The local validation gate passes;
full parser/object snapshots and fuzzing remain mandatory evidence as the
object-model migration advances.

The second Phase 6 slice moves cross-reference object-type validation into
Rust. Only the defined free, normal, and compressed codes are accepted; an
unknown code retains the C++ oracle. C++ continues to interpret the field
values and mutate the cross-reference table. The native parser test target now
also covers all defined type codes, invalid codes, and no-mutation rejection.

The third Phase 6 slice moves effective entry-type planning into Rust. A
missing first field receives the ISO default `normal` type; explicit values
must still be one of the three defined codes. C++ continues to read offset,
generation, and archive values and performs every table mutation. The native
parser suite verifies both the default and invalid explicit codes.

The fourth Phase 6 slice moves the entry action into Rust. Given the decoded
type and range results, Rust selects free, normal, compressed, or skip.
Generation overflow, non-representable normal offsets, and invalid archive
object numbers remain rejects. C++ owns every `CPDF_CrossRefTable` mutation,
so the object graph and lifetime boundary remain unchanged. Native tests cover
each valid action and every scalar rejection path.

The fifth Phase 6 slice moves `/Index` pair validation into Rust. Negative
starts and non-positive counts are rejected; accepted signed values are
normalized to the original unsigned representation. C++ retains the segment
vector, the empty-array default, and later segment/object-graph processing.
Native tests cover valid bounds plus negative and empty rejection.

The sixth Phase 6 slice moves segment byte-range planning into Rust. Rust
checks the offset and length multiplications, their sum, and the filtered
stream length before C++ creates its borrowed span. C++ retains the span,
iteration, object loading, and all object-graph mutations. Native tests cover
valid ranges, out-of-bounds ranges, and large scalar inputs.

The seventh Phase 6 slice moves ordered segment entry iteration into Rust. A
narrow synchronous private `CPDF_Parser` callback receives each index and
stops at PDFium's maximum object number. C++ retains entry decoding,
cross-reference table mutation, and every parser/object lifetime. The native
parser test target proves ascending iteration and the callback stop boundary.

The eighth Phase 6 slice moves `/W` field-width conversion into Rust. Rust
preserves PDFium's signed-to-unsigned cast semantics, including negative
values, while C++ retains the array traversal, field-width vector, and all
subsequent segment/object-graph work. Native tests cover zero, ordinary,
negative, and minimum signed values.

The ninth Phase 6 slice moves `CPDF_SimpleParser` whitespace and comment
scanning into Rust. Rust consumes only the borrowed byte span and returns the
new position plus the first parseable byte; C++ retains the span lifetime and
all delimiter-, name-, string-, and token-specific handling. End-of-input
still advances the same consumed position, including after an unterminated
comment, before returning no byte. The native parser tests cover every PDFium
whitespace byte, comment line endings, chained comments, and the no-mutation
byte-output failure contract.

The tenth Phase 6 slice moves complete three-field cross-reference entry
collection into Rust. Rust reads all ISO table-18 field widths from the same
borrowed entry and preserves PDFium's zero-width and 32-bit wrapping rules.
C++ retains type interpretation fallback, range policy, and every
`CPDF_CrossRefTable` mutation. The native parser tests cover zero-width fields,
over-wide wrapping fields, short-entry rejection, and the no-output-mutation
failure contract.

The eleventh Phase 6 slice moves complete `CPDF_SimpleParser::GetWord()` token
range planning into Rust. Rust preserves names, regular tokens, nested
parentheses, hexadecimal strings, dictionary brackets, single delimiters,
PDFium's extended `0x80` and `0xff` whitespace bytes, and the historical empty
result for unterminated names. C++ retains the borrowed input storage,
`ByteStringView` construction, and the complete handler-based oracle as the
boundary fallback. Sixteen native parser tests cover the scalar and FFI
contracts, including rejection without output mutation; the common local gate
passes.

The Phase 6 differential baseline runs the versioned normal, default-type,
unknown-type, and truncated cross-reference corpus twice in the same process.
A test-only scoped selector chooses the retained C++ oracle or production Rust
candidate, then compares the parse error, rebuild decision, trailer object
number, and every free, normal, and compressed cross-reference object-map
entry. The hermetic full gate builds `pdfium_unittests` and the retained parser
fuzzer, passes all 16 native Rust parser tests, all four corpus snapshots, both
existing simple-parser tests, and the complete 1,057-test unit suite.

`pdf_parser_fuzzer` bounds each input to 1 MiB, compares every
`CPDF_SimpleParser` token and consumed position between the C++ oracle and Rust
candidate without intermediate allocations, and then passes the same bytes to
the production public in-memory document parser. `pdfium_all` compiles this
retained target so later sanitizer/libFuzzer campaigns keep the active parser
boundary covered.

The twelfth Phase 6 slice moves cross-reference table mutation orchestration
into Rust. Rust applies the existing generation, offset, and archive-object
range policy, completes rejected entries without crossing the boundary, and
invokes exactly one synchronous C++ callback for accepted free, normal, or
compressed entries. C++ retains `CPDF_CrossRefTable` map storage and PDF object
lifetimes; the complete C++ switch remains the boundary fallback.

Eighteen native parser tests cover mutation command selection, exactly-once
callback behavior, skip-without-callback, and invalid-boundary rejection. The
four-case object snapshot corpus proves identical parse status, rebuild state,
trailer number, and cross-reference entries; the complete 1,057-test unit suite
passes in the full GN build.

The thirteenth Phase 6 slice moves cross-reference object-map sizing
orchestration into Rust. A zero size invokes one clear operation; a nonzero
size first truncates object numbers at the exclusive limit, then ensures the
last in-range entry. C++ retains the only `std::map` storage and performs the
synchronous mutations selected by Rust, so this slice introduces no shadow
object state. The scoped parser selector keeps the original C++ algorithm as
the same-process oracle.

Nineteen native parser tests cover the ordered sizing operations, zero and
maximum scalar boundaries, callback failure short-circuiting, and invalid FFI
boundaries. The four-case object snapshot corpus and both simple-parser tests
match the C++ oracle, the retained parser fuzzer source builds, and all 1,057
unit tests pass in the full GN build.

The fourteenth Phase 6 slice moves cross-reference overlay conflict policy
into Rust. For each current entry, Rust chooses whether the top table remains
authoritative, the current entry fills a missing key, or the object-stream flag
must survive when both entries are normal objects. C++ retains the sole
`std::map`, ordered traversal, and all object lifetimes; the original merge
algorithm remains selected by the test-only oracle switch.

Twenty native parser tests cover missing entries, overlay precedence, flag
preservation, invalid type codes, and no-output-mutation FFI failures. The
four-case object snapshot corpus and both simple-parser tests match the oracle,
the retained parser fuzzer source builds, and all 1,057 unit tests pass in the
full GN build.

The fifteenth Phase 6 slice moves normal and compressed cross-reference entry
admission into Rust. Rust preserves normal-object generation precedence,
rejects compressed replacements after a nonzero generation, and prevents
known object streams from being nested in object streams. C++ retains the sole
map, writes admitted entry fields, and marks the archive object.

Twenty-one native parser tests cover equal, newer, and older generations,
compressed-entry guards, invalid type codes, and FFI failure behavior. The
four-case object snapshots and both simple-parser tests match the oracle, the
retained parser fuzzer and `pdfium_all` build, and all 1,057 unit tests pass.

The sixteenth Phase 6 slice replaces the production cross-reference entry map
with a Rust-owned `BTreeMap`. Rust now owns free, normal, and compressed entry
storage, generation and object-stream guards, size truncation, overlay merge
semantics, lookup state, and ascending snapshot traversal. The C++ map remains
the independently selected oracle when the test selector disables the
candidate. In production it is only a lazily rebuilt compatibility view for
legacy iteration and test access; mutations never use that view as input.

The initial storage commit adds opaque allocation, mutation, overlay, lookup,
and snapshot boundaries. Activation fixes the implementation choice for each
`CPDF_CrossRefTable` lifetime and invalidates the derived view after every Rust
mutation. A cleanup commit removes the three superseded policy-only FFIs and
their tests so they cannot inflate the active ownership claim.

Twenty focused native parser tests remain after that cleanup. The four normal,
defaulted, unknown, and truncated object snapshots match the retained C++ map,
both simple-parser tests pass, the parser fuzzer source builds, `pdfium_all`
builds, and all 1,057 unit tests pass in the full GN configuration.

The seventeenth Phase 6 slice replaces the production indirect-object number
map with a Rust-owned index. Rust now owns last-object-number state, wrapping
number allocation, recursive-parse placeholders, object-handle lookup,
generation-aware replacement, deletion, and ascending iteration. Each holder
fixes its Oracle/Candidate choice at construction, matching the cross-reference
storage activation shape.

C++ retains `CPDF_Object` values in a native `RetainPtr` registry keyed only by
opaque pointer handles, including reference counts when one object appears in
multiple Rust slots. It does not keep a second object-number index. The public
C++ iterator map is cleared on every mutation and lazily rebuilt from a Rust
snapshot, so it is a derived compatibility view rather than mutation input.

One focused Rust state test covers recursive reservation, parse completion and
cancellation, generation precedence, replacement, deletion, absent handles,
last-number updates, and unsigned wraparound. A same-process C++/Rust scenario
compares numbering, lower/higher generation replacement, deletion, last-number
state, and ordered iteration. All six indirect-object holder tests, four parser
object snapshots, both simple-parser tests, all 1,058 unit tests, the retained
parser fuzzer build, and `pdfium_all` pass in the full GN configuration.

The eighteenth Phase 6 slice replaces `CPDF_Number`'s production `FX_Number`
value with one Rust-owned signed, unsigned, or float variant. Rust preserves
the unsigned permissions-value path, signed limit handling, deliberate integer
overflow reset, token-prefix parsing, PDFium's multiple-leading-sign float
rules, float rounding, saturated integer conversion, `SetString()`, and the
value used by clones. Each number holds either the retained C++ oracle or the
Rust value, never two synchronized representations.

C++ retains `WriteFloat()` as the exact PDF text-formatting adapter and writes
the resulting bytes to `IFX_ArchiveStream`. The same-process differential
scenario compares integer classification, signed conversion, exact float bits,
formatted strings, mutation, and clone results across empty, malformed,
overflow, boundary, exponent-like, precision, multiple-sign, and infinity
inputs. The focused Rust suite has 22 tests, all three `CPDFNumber` tests pass,
all 1,059 unit tests pass, the parser fuzzer source builds, and `pdfium_all`
builds in the full GN configuration.

The nineteenth Phase 6 slice replaces `CPDF_Boolean`'s production inline bool
with a Rust-owned value. Rust owns construction, `SetString()` keyword
interpretation, integer conversion, and clone state. The retained C++ oracle
remains separately selected per object; C++ only selects the static `true` or
`false` output bytes and writes them to the archive stream.

The initial differential run exposed PDFium's historical `ByteString ==
"true"` behavior for embedded NUL bytes: `true\0...` is accepted because the
comparison observes the C-string terminator. Rust preserves that behavior, and
the native test covers exact, false, case-mismatched, empty, and NUL-terminated
inputs. The same-process test compares integer, string, clone, and serialized
output. All 23 native Rust tests, the boolean and three number tests, all 1,060
unit tests, the retained parser fuzzer build, and `pdfium_all` pass in the full
GN configuration.

The twentieth Phase 6 slice replaces `CPDF_Reference`'s production object
number with a Rust-owned value. Rust owns construction, all unsigned object
number values, `SetRef()` number mutation, lookup input, serialization input,
and the value copied into clones and references-to-references. Each reference
contains either the retained C++ oracle number or the Rust state, never two
synchronized numbers.

The holder remains a C++ `UnownedPtr`: this slice deliberately does not claim
ownership of, retain, or extend the `CPDF_IndirectObjectHolder` lifetime. C++
also retains direct-object resolution and archive writes. The same-process
scenario compares null and live holders, direct number resolution, zero,
missing, ordinary, and maximum object numbers, mutation, cloning, string and
numeric forwarding, and serialized output. All 24 native Rust tests, the
reference, boolean, and three number tests, all 1,061 unit tests, the retained
parser fuzzer build, and `pdfium_all` pass in the full GN configuration.

The twenty-first Phase 6 slice replaces `CPDF_Array`'s production object
vector with a Rust-owned ordered handle vector. Rust owns size, indexed lookup,
append, insert, replacement, removal, clear, and the canonical order used for
cloning, serialization, and iteration. Array construction fixes the retained
C++ oracle or Rust candidate for the full object lifetime.

C++ retains `CPDF_Object` values in a native registry keyed by opaque pointer
handles. Per-handle slot counts preserve duplicate objects without creating a
second C++ index, and the existing cycle-breaking destructor still leaks only
objects marked with the invalid cycle sentinel. `CPDF_ArrayLocker` rebuilds a
read-only C++ vector lazily from the Rust order; every mutation clears it, so it
is a derived compatibility view rather than mutation input. The existing
`ByteStringPool` weak pointer remains C++-owned for child construction.

The same-process scenario compares duplicate slots, order, append, insert,
replacement, invalid indices, removal, clear, clone state, indexed reads, and
the locker view. All 25 native Rust tests, 20 focused array tests, all 1,062
unit tests, the retained parser fuzzer build, and `pdfium_all` pass in the full
GN configuration.

The twenty-second Phase 6 slice replaces `CPDF_Dictionary`'s production map
with a Rust-owned `BTreeMap<Vec<u8>, handle>`. Rust owns exact binary key bytes,
bytewise sorted order, lookup, insertion, overwrite, removal, key replacement,
and the canonical order used for cloning and serialization. Embedded NUL bytes
remain part of the key rather than being treated as C-string terminators.

C++ retains `CPDF_Object` values in a reference-counted opaque-handle registry
and preserves the existing cycle-breaking destructor. The legacy dictionary
locker map is cleared after every mutation and lazily derived from a Rust
snapshot. Its keys may still use the existing `ByteStringPool` for
compatibility, but that pooled C++ map is never mutation input; the canonical
key ownership is Rust. Child `CPDF_String`, `CPDF_Name`, array, and dictionary
construction keeps the existing weak pool adapter.

The same-process scenario compares sorted iteration, shared values, binary
keys, overwrite, replacement over an existing key, removal with ownership
return, missing-key deletion, clone state, and indexed lookup. All 26 native
Rust tests, five focused dictionary tests, all 1,063 unit tests, the retained
parser fuzzer build, and `pdfium_all` pass in the full GN configuration.

The twenty-third Phase 6 slice establishes the string-ownership foundation by
replacing `ByteStringPool`'s C++ `unordered_set` index with a Rust-owned binary
key-to-handle map. Rust decides whether a byte sequence is new or already
interned, including sequences containing embedded NUL bytes. A dedicated
`core/fxcrt/rust` crate keeps this low-level boundary independent of the parser
crate and available to parser object constructors without an upward dependency.

C++ retains one `ByteString` per opaque handle because `ByteString` is the
native copy-on-write ABI value returned throughout PDFium. This is an explicit
buffer adapter, not a second lookup index. The existing pool test proves that
the first input buffer remains the canonical backing allocation and repeated
interning returns the exact same non-null `c_str()` address. The native Rust
test covers distinct binary keys and stable existing-handle selection.

The Rust pool test, the existing exact-sharing pool test, both array and
dictionary differential regressions, all 1,063 unit tests, the retained parser
fuzzer build, and `pdfium_all` pass in the full GN configuration.

The twenty-fourth Phase 6 slice replaces `CPDF_String`'s canonical binary
value and hex-output flag with Rust state. Rust owns construction bytes,
embedded NULs, `SetString()` mutation, and the mode copied into clones. The
retained C++ `ByteString` is an ABI and copy-on-write view: every candidate read
checks it byte-for-byte against Rust before Unicode decoding, encryption,
encoding, serialization, or return to native callers.

Keeping that checked view preserves two resource contracts that a naive
`Vec<u8>` return would lose: strings built through `ByteStringPool` retain the
interned backing allocation, and clones share the original nonempty buffer.
Neither the view nor the C++ hex flag selects candidate behavior; Rust supplies
the authoritative hex mode and accepts every mutation first.

The same-process scenario compares regular and hex strings, embedded NUL and
arbitrary binary bytes, mutation, encoding, serialization, clone content, and
exact clone buffer sharing. All 27 parser-native Rust tests, the string test,
all 1,064 unit tests, the retained parser fuzzer build, and `pdfium_all` pass in
the full GN configuration.

The twenty-fifth Phase 6 slice applies the same Rust-owned binary state to
`CPDF_Name`. Rust accepts construction and `SetString()` bytes first, including
embedded NULs, while every native read verifies the pooled `ByteString` ABI
view byte-for-byte before Unicode decoding, name encoding, serialization, or
cloning. No new Rust data model is introduced: names reuse the already tested
binary string state with hex mode disabled.

The same-process scenario compares bytes requiring PDF name escaping,
embedded NULs, mutation, encoded serialization, clone content, and exact clone
buffer sharing. Both name and string differential tests pass, as do all 1,065
unit tests, the retained parser fuzzer build, and `pdfium_all` in the full GN
configuration.

The twenty-sixth Phase 6 slice replaces every `CPDF_Stream` in-memory
`DataVector` with a Rust-owned byte vector. The implementation choice is fixed
for each stream lifetime; Rust owns bytes and length after span, stringstream,
and ownership-taking mutations, and exposes a borrowed immutable span to the
existing stream accumulator and encoder paths. Empty and arbitrary binary
payloads retain exact byte semantics.

File-backed streams deliberately remain a native
`RetainPtr<IFX_SeekableReadStream>` backend, and the stream dictionary remains
a native `RetainPtr<CPDF_Dictionary>` lifetime adapter. C++ also retains Flate
selection, metadata rules, encryption, dictionary/archive orchestration, and
the public borrowed-span ABI. Moving data to Rust adds one copy when a native
`DataVector` is transferred; no persistent duplicate buffer remains.

The same-process scenario compares construction, raw bytes and size,
`SetDataAndRemoveFilter()`, ownership-taking `TakeData()`, stringstream input,
`/Length`, filter state, clone bytes, and exact serialization. All 28
parser-native Rust tests, the stream differential test, all 1,066 unit tests,
the retained parser fuzzer build, and `pdfium_all` pass in the full GN
configuration.

The Phase 6 retained fuzzer now opens every bounded input independently with
the C++ oracle and Rust candidate. It compares load success, exact public error
code, page count, file-version status and value, permissions, security-handler
revision, and first/last page load success and float geometry. Token bytes and
consumed offsets remain exact differential checks before document loading.

`pdf_parser_fuzzer_corpus` links the real fuzzer entry point into a standalone
runner over the versioned valid, truncated, default-width, and unknown-entry
inputs. This makes the retained fuzzer executable locally rather than proving
only that its source compiles. The normal light runner and a separate
`is_asan=true` build with leak detection and immediate failure both complete
without a mismatch or sanitizer finding. Inputs remain capped at 1 MiB and
only the first and last pages are loaded, bounding per-input page work.

The Phase 6 save/reload gate opens `hello_world.pdf` through the production
Rust parser/object graph, captures sorted catalog and page keys, catalog/page
types, MediaBox coordinates, and the concatenated fully filtered content-stream
bytes, then saves and reloads through the public API. The complete semantic
snapshot, 30 extracted characters, and rendered page all remain identical.

All six `RustCodecEmbedderTest` save/reload cases pass. The full embedder run
passes 506 of 557 tests; its 51 failures are the same static macOS AGG golden
class already recorded by the renderer phases. The new object-graph test adds
one passing case and does not add a failure. The render-intensive
`LargeImageDoesNotRenderBlank` case also passes after 181.8 seconds.

The next Phase 6 lifetime slice removes the three C++ slot-reference counters
from arrays, dictionaries, and indirect-object holders. After every failed,
replaced, removed, deleted, or cleared mutation, C++ asks the owning Rust state
whether the opaque handle remains present in any canonical slot. Only Rust's
answer decides whether the single native registry entry is erased.

C++ still stores one `RetainPtr<CPDF_Object>` per live opaque handle because
`CPDF_Object` remains the intrusive native ABI value and its cycle sentinel is
implemented by native destructors. The registry no longer duplicates slot
counts or lifetime policy. Native tests cover retained and last-removed
handles, shared array/dictionary values, holder replacement/deletion, lazy
views, save/reload, and the normal and ASan public parser corpus. All 28 Rust
parser tests, 19 focused container/holder tests, all 1,066 unit tests, all six
focused embedder tests, `pdfium_all`, and the ASan+LSan corpus pass.

This completes the Phase 6 owned-state boundary. Intentionally retained native
backends are file-backed `IFX_SeekableReadStream`, one intrusive
`RetainPtr<CPDF_Object>` ABI value per live opaque handle, native cycle-sentinel
destructors, encryption/compression/archive calls, and public C ABI wrappers.
None retains a parallel PDF value, key map, object-number index, slot count, or
in-memory stream buffer. The candidate ledger now advances to Phase 7 rather
than treating these declared backends as unfinished parser behavior.

## Phase 7 edit, document, text, and SDK implementation

The first Phase 7 slice replaces `CPDF_Document::page_list_` in production with
a Rust-owned object-number vector. Rust owns page-count cache size, unloaded
zero slots, object-number lookup and membership, cache population, resize,
insertion, and removal used by page creation, deletion, movement, traversal,
and object-null replacement decisions. Each document fixes its Oracle/Candidate
choice at construction.

C++ retains the recursive PDF `/Pages` dictionary traversal and repair logic,
page dictionary objects, extension callbacks, and public SDK entry points. A
same-process scenario builds the seven-page nested tree independently under
the oracle and candidate, loads pages out of order, creates a page, moves two
pages, deletes one, then compares count, logical numbering, object numbers, and
loaded state for every remaining page.

All 29 parser-native Rust tests, all nine `DocumentTest` cases, six public page
delete/save cases, the public page-move corpus, the object-graph save/reload
test, all 1,067 unit tests, and `pdfium_all` pass in the full light build.

The second Phase 7 slice moves the complete public page-move plan into Rust.
Rust rejects empty, oversized, duplicate, negative, out-of-range, and invalid
destination requests before mutation and returns the unique source indices in
descending deletion order. C++ retains borrowed page dictionaries in original
input order, XFA extension policy/callbacks, page-tree deletion/insertion, and
the public `FPDF_MovePages()` wrapper.

The native Rust test covers valid unsorted input and every rejection class.
The existing public 44-case move corpus passes unchanged, including expected
success/failure results and rendered/save behavior. All 30 parser-native Rust
tests, the document Oracle/Candidate create-move-delete scenario, all 1,067
unit tests, and `pdfium_all` pass in the full light build.

The third Phase 7 slice moves recursive page-tree counting into Rust. Rust owns
`/Count` hint acceptance, child traversal, active-recursion-path cycle guards,
malformed `/Type` inference, corrected `/Type` and `/Count` writes, checked page
totals, and the existing page maximum. C++ exposes borrowed dictionaries only
for the synchronous call and retains the original `CountPages()` fallback.

The candidate traversal adds a 1,024-level resource bound matching the existing
page-tree lookup bound. If that bound or any callback contract is rejected, the
unchanged C++ traversal runs, so the public result and repair behavior remain
available. A regression case proves that a shared subtree is counted once per
appearance while cycles are guarded only along the active path, matching the
oracle rather than globally deduplicating dictionaries.

All 32 parser-native Rust tests, all nine `DocumentTest` cases (including bad
counts and missing kids), six public delete/save cases, the complete public
page-move case, the object-graph save/reload case, all 1,067 unit tests, and
`pdfium_all` pass in the full light build.

The fourth Phase 7 slice moves uncached page-object-number lookup traversal
into Rust. It preserves the loaded-prefix skip count, valid subtree-count skip,
direct-reference fast path, leaf index advancement, self-reference rejection,
and 1,024-level bound. C++ lends dictionary/array/reference views synchronously
and retains the unchanged `FindPageIndex()` fallback on boundary rejection.

The same-process document scenario now clears every cache slot after its
create/move/delete sequence and compares the index resolved for every remaining
page under Oracle and Candidate. All 33 parser-native tests, all nine document
tests, all 1,067 unit tests, and `pdfium_all` pass.

The fifth Phase 7 slice moves the page-range grammar used by public
`FPDF_ImportPages()` into Rust. Rust owns character validation, space removal,
single-page and inclusive-range expansion, one-based bounds, input ordering,
and duplicate preservation. The adapter uses a sizing pass followed by a fill
pass; this doubles parsing work by a constant factor without changing linear
time or output memory complexity. Decimal values outside the C++ `atoi()`
domain reject the Rust boundary and run the retained platform oracle.

The full legacy grammar corpus and an explicit same-process Oracle/Candidate
matrix pass, as do the public import good/bad-range cases. All 34 parser-native
tests, all three SDK-helper tests, all 1,068 unit tests, and `pdfium_all` pass.

The sixth Phase 7 slice moves nested page-tree insertion/deletion path planning
into Rust. Rust owns leaf selection, subtree `/Count` skipping, active-path
cycle rejection, and the ordered child-index path. C++ retains dictionary and
reference ownership, leaf insertion/removal, `/Parent` and ancestor `/Count`
writes, traversal-cache reset, extension callbacks, and public wrappers.

The candidate uses the same 1,024-level resource bound as page lookup; depth or
checked-arithmetic rejection falls back to the unchanged recursive C++ oracle.
The malformed-count regression proves `INT32_MIN` cannot panic across the FFI.
The document create/move/delete Oracle/Candidate scenario, six public
delete/save cases, the complete public move case, object-graph save/reload, all
35 parser-native tests, all 1,068 unit tests, and `pdfium_all` pass.

The seventh Phase 7 slice moves the shared byte-string output contract used by
public action, document, viewer-preference, signature, image, and structure
getters into Rust. Rust owns checked required-length calculation, trailing NUL,
and the all-or-nothing copy rule that leaves undersized buffers untouched. C++
retains source `ByteString` ownership, each public wrapper, and checked
conversion to the public `unsigned long` length.

Native coverage includes embedded NUL bytes and short/exact buffers. A
same-process Oracle/Candidate matrix compares every output byte for zero,
undersized, and exact capacities. Six representative public getter cases, all
four SDK-helper tests, all 36 parser-native tests, all 1,069 unit tests, and
`pdfium_all` pass.

The eighth Phase 7 slice replaces the production incremental page-tree
traversal state with a Rust-owned stack. Rust owns active node handles and child
indices, recursive continuation, the next-page cursor, maximum-depth state,
null/self-child handling, page-cache scheduling, and reset/pop policy. C++
retains synchronous dictionary/array access and an unordered lifetime registry
containing exactly one `RetainPtr` token per active Rust stack occurrence; it
does not retain child positions, order, cursor, or traversal policy.

The original `PageTreeData` traversal remains only on the separately selected
oracle and boundary-fallback path. The native state test walks three sequential
pages through a nested tree, proves cache writes `(0,10)`, `(1,11)`, `(2,12)`,
and verifies every lifetime token is released by clear. All 37 parser-native
tests, all nine document tests, the public delete/move/save-reload cases, all
1,069 unit tests, and `pdfium_all` pass. The full embedder gate remains 506/557
with exactly the same 51 documented macOS AGG static-golden failures;
`LargeImageDoesNotRenderBlank` passes after 189.6 seconds. The rebuilt public
parser corpus also completes under ASan+LSan with leak detection and immediate
failure enabled.

The ninth Phase 7 slice establishes a dedicated `core/fpdftext/rust` domain and
replaces the production `CPDF_TextPageFind` state machine. Rust owns the copied
page/search-word code points, forward and backward cursors, result bounds,
consecutive-overlap behavior, whitespace-separated matching, whole-word rules,
and previous-result scan. C++ retains PDF text-page ownership, historical case
folding and query tokenization, public wrappers, and synchronous
text-index/character-index mapping callbacks. Candidate construction keeps no
parallel C++ page text, query vector, cursor, or result state.

`FindPrev()` intentionally creates a temporary Rust search state just as the
C++ oracle creates a temporary engine; time and memory complexity therefore
remain unchanged. The first public run exposed the historical unset-result
sentinel (`res_end = -1`), fixed separately by preserving its wrapping ABI
conversion. Both native text tests, all 62 public text embedder cases, the
five-scenario same-process Oracle/Candidate forward/backward matrix, all 1,069
unit tests, and `pdfium_all` pass.

The tenth Phase 7 slice moves search-query tokenization into the Rust text
domain. Candidate construction now passes one normalized query and never builds
or retains the C++ `find_what_array_`. Rust preserves the special all-space
query, one leading/trailing empty marker, collapsed internal ASCII spaces,
single-character splitting for the historical script ranges, and U+2019 inside
a word. C++ retains case folding and `ExtractFindWhat()` only for the separately
selected oracle.

Three native text tests cover spaces, Han boundaries, and the right apostrophe.
The initial build caught the now-oracle-only temporary constructor in
`FindPrev()` and its compile fix is isolated. All 62 public text cases, the
same-process forward/backward matrix, all 1,069 unit tests, and `pdfium_all`
pass.

The eleventh Phase 7 slice moves `CPDF_LinkExtract` production state and web/
mail recognition into the Rust text domain. Rust owns the link vector, original
case URL bytes, text ranges, generated-hyphen replacement, line-break joining,
HTTP/HTTPS/WWW detection, external bracket/quote trimming, host and IPv6-port
ending rules, and local/domain email validation. C++ retains the text page,
rectangle calculation, public handles/copying, and one synchronous
`FXSYS_iswalnum()` callback so platform Unicode classification does not drift.
The candidate keeps no parallel C++ `link_array_`.

URL retrieval now reconstructs a `WideString` before the already-linear public
UTF-16 copy, so public time and output-memory complexity remain linear. Five
native text tests, both existing web/mail oracle unit corpora, all five existing
public web/annotation link cases, and a new same-process comparison of every
multi-line URL code unit and text range pass. All 1,069 unit tests and
`pdfium_all` also pass.

The twelfth Phase 7 slice replaces the production `CPDF_TextPage` visible-text
index vector with a Rust-owned segment map. Candidate text pages classify each
character once in C++, pass only inclusion bytes across the synchronous
constructor boundary, and retain no parallel C++ `char_indices_`. Rust owns
segment compaction and both text-to-character and character-to-text conversion;
C++ retains PDF character objects, the historical normal/generated-character
classification, public wrappers, and the separately selected oracle vector.

Six native Rust text tests cover excluded runs and both mapping directions. A
new same-process public differential constructs candidate and oracle text pages
separately on a fixture containing a non-printable character, then compares
both conversions across negative, in-range, and out-of-range indexes. All
seven search-extension tests, all 63 public text tests, all 1,069 unit tests,
and `pdfium_all` pass.

The thirteenth Phase 7 slice moves `FPDFText_GetCharIndexAtPos()` containment
and nearest-character selection into Rust. The candidate reads one borrowed
character rectangle at a time through a synchronous callback, preserving the
oracle's O(n) time and O(1) auxiliary memory. Rust owns rectangle normalization,
inclusive containment, tolerance expansion, edge-distance ranking, and stable
first-match ties; C++ retains the character list, rectangle storage, public
wrapper, and separately selected loop.

The initial native callback harness failed before public validation, so the
algorithm was separated into a safe statically dispatched helper and the raw
callback was confined to the FFI wrapper. Seven native Rust text tests, a
same-process public matrix covering exact, missed, extreme, negative, and
zero-tolerance positions, all seven search-extension tests, all 64 public text
tests, all 1,069 unit tests, and `pdfium_all` pass.

The fourteenth Phase 7 slice moves text selection-rectangle planning and the
retained `FPDFText_CountRects()` result into Rust. Candidate pages no longer
populate `sel_rects_`: Rust owns range normalization, generated/degenerate
character filtering, text-object grouping, rectangle normalization and union,
stable output order, and the stored result queried by `FPDFText_GetRect()`.
C++ retains borrowed character metadata through one synchronous callback and
the separately selected planning loop. Link-rectangle callers receive the
same required C++ value copy from a temporary Rust-owned plan.

Eight native Rust text tests cover generated-character skipping, same-object
union, and object-boundary splitting. A same-process public matrix compares
counts and every rectangle for invalid, empty, bounded, remainder, and
oversized ranges. All seven search-extension tests, all 65 public text tests,
all 1,069 unit tests, and `pdfium_all` pass.

The fifteenth Phase 7 slice moves `CPDF_TextPage::GetPageText()` range planning
onto the existing Rust-owned character/visible-text index map. Rust now owns
forward scanning from a non-printing start, backward scanning from a
non-printing end, count clipping, and the final visible buffer offset/length.
C++ retains the `WideTextBuffer`, final substring value copy, public wrapper,
and separately selected scan. The candidate performs no mapping callback or
parallel C++ scan.

The native corpus exposed and records the historical shifted-start rule: the
requested count is applied after advancing past a non-printing start, so the
visible range can extend one character farther than a caller might expect.
Nine native Rust text tests and a same-process public comparison of exact
buffers and return lengths across invalid, empty, clipped, and non-printing
ranges pass, together with all seven search-extension tests, all 66 public text
tests, all 1,069 unit tests, and `pdfium_all`.

The sixteenth Phase 7 slice moves `GetTextByPredicate()` assembly into Rust for
both bounded-rectangle and text-object selection. Rust owns included/excluded
character state, intervening-space preservation, vertical transition tracking,
CRLF insertion, zero-Unicode filtering, and output code-point order. C++
retains predicate evaluation against native geometry/object identity, borrowed
Unicode/origin metadata through a synchronous callback, final `WideString`
conversion, public wrappers, and the separately selected loop.

The candidate retains O(n) traversal and O(output) memory, using a temporary
Rust code-point result before the required native string value copy. Ten native
Rust text tests cover space and line-transition behavior. A same-process public
matrix compares required lengths, copied lengths, every UTF-16 code unit, and
untouched buffer capacity for partial, full-page, and empty rectangles. All
seven search-extension tests, all 67 public text tests, all 1,069 unit tests,
and `pdfium_all` pass.

The seventeenth Phase 7 slice moves temporary text-line normalization and Bidi
emission planning into Rust. Rust owns duplicate-space keep indices, carried
segment direction, neutral-after-RTL handling, reverse/forward segment order,
source-character indices, and per-character RTL flags. C++ retains the native
`CFX_BidiString` Unicode classifier, borrowed `CharInfo` values, mirror/
normalization application in `AddCharInfo()`, and the separately selected
erase/iteration oracle.

The candidate replaces repeated middle erases (worst-case O(n²)) with linear
keep-index construction and linear emission planning. It remains O(n) in
auxiliary memory for the collapsed line, keep indices, native segments, and
validated emissions; this resource change is intentional and measured here.
Eleven native Rust text tests cover duplicate spaces, mixed LTR/RTL segments,
reversal, and RTL flags. The same-process public differential compares count,
return length, NUL termination, every UTF-16 code unit, and untouched capacity
on mixed `ActualText` RTL content. All seven search-extension tests, all 68
public text tests, all 1,069 unit tests, and `pdfium_all` pass.

The eighteenth Phase 7 slice moves `AddCharInfo()` classification and emission
planning into Rust. Rust owns the control-code set, the hyphen exception,
zero-Unicode/character-code fallback, whether mirrored display Unicode is
required, whether normalization is required, empty-normalization fallback,
ligature expansion, text-buffer append decisions, Unicode overrides, and
`CharType::kPiece` assignment. C++ performs mirror and normalization table
lookups only when requested, then applies the validated emissions to native
`WideTextBuffer` and `CharInfo` storage; the complete original loop remains the
separately selected oracle.

The request/set state machine avoids calling native Unicode backends for
non-normal characters and preserves linear time and O(normalized output)
temporary memory. Twelve native Rust text tests cover controls, the hyphen
exception, ligature expansion, mirrored display requests, and emission types.
A same-process public differential compares every character's Unicode,
generated flag, and hyphen flag across the full ligature/control fixture, while
the mixed RTL differential covers mirrored normalization. All seven
search-extension tests, all 69 public text tests, all 1,069 unit tests, and
`pdfium_all` pass.

The nineteenth Phase 7 slice moves `FindTextlineFlowOrientation()` mask
construction and decision logic into Rust. Rust clamps active text-object
bounds to the integral page extent, fills the horizontal and vertical masks,
tracks occupied spans and the first usable line height, computes the exact
filled ratios, and selects horizontal, vertical, or unknown flow. C++ supplies
borrowed page-object activity, type, and geometry through a synchronous
callback; the complete original implementation remains the separately selected
oracle.

The candidate preserves the legacy page-sized O(width + height) masks and
linear object/range traversal, including integer truncation and empty-page
behavior. Thirteen native Rust text tests include horizontal and vertical
fixtures. A same-process public differential compares every character's
Unicode, origin, and box on vertical text. All seven search-extension tests,
all 70 public text tests, all 1,069 unit tests, and `pdfium_all` pass.

The twentieth Phase 7 slice moves per-object writing-mode selection into Rust.
C++ transforms the first and last character origins with the native text
matrix, then Rust owns endpoint deltas, coincident-origin epsilon handling,
vector normalization, horizontal/vertical axis thresholds, and fallback to the
page flow direction. The complete original implementation remains the
separately selected oracle.

The boundary is constant-space and preserves the original O(1) decision after
endpoint access. Fourteen native Rust text tests include horizontal, vertical,
single-character, coincident-origin, and fallback cases. A same-process public
differential compares every character's Unicode, angle, and origin on rotated
text. All seven search-extension tests, all 71 public text tests, all 1,069 unit
tests, and `pdfium_all` pass.

The twenty-first Phase 7 slice moves adjacent text-object separator geometry
into Rust. Rust owns horizontal and vertical line-end intersection rules,
small-object/font-size guards, ordered character-width threshold bands, and the
final gap-based space insertion decision. C++ retains text/font access, matrix
and distance transforms, hyphen policy, and mutation of the native text and
character buffers; the original numeric helpers remain the separately selected
oracle.

The candidate remains constant-space and O(1) per adjacent object pair.
Fifteen native Rust text tests cover horizontal and vertical line endings,
threshold bands, touching glyphs, and separated glyphs. A same-process public
differential compares every Unicode value, generated flag, and hyphen flag on
the multiline hyphen fixture. All seven search-extension tests, all 72 public
text tests, all 1,069 unit tests, and `pdfium_all` pass.

The twenty-second Phase 7 slice moves line-ending hyphen joining into Rust.
Rust scans the supplied active text buffer backward across trailing spaces,
recognizes ASCII and soft hyphens, applies the alphabetic-to-alphanumeric word
continuation rule, and preserves the prior `CharType::kPiece` fallback. C++
retains native buffer and previous-character storage and supplies the platform
Unicode alpha/alphanumeric predicates through a synchronous callback; the
complete original implementation remains the separately selected oracle.

The candidate remains O(n) only in the trailing-space suffix and uses O(1)
auxiliary storage; Rust reads required code points through a synchronous,
checked index callback. Sixteen native Rust text tests cover skipped spaces,
ordinary word continuation, and piece fallback. The existing same-process
multiline hyphen differential compares every Unicode value, generated flag,
and hyphen flag. All seven
search-extension tests, all 72 public text tests, all 1,069 unit tests, and
`pdfium_all` pass.

The twenty-third Phase 7 slice moves generated inter-object character action
planning into Rust. Rust maps the requested separator type to no-op, generated
space, line close, or hyphen replacement; suppresses a standalone incoming
ASCII/soft hyphen; and counts trailing spaces that must be removed before a
hyphen replacement. C++ retains native font Unicode lookup, line closing,
generated-character construction, and the requested buffer/list mutations;
the complete original switch remains the separately selected oracle.

The candidate scans only the trailing temporary-text suffix through a checked
synchronous callback, uses O(1) auxiliary storage, and preserves the original
mutation order. Seventeen native Rust text tests cover all action kinds,
standalone-hyphen suppression, and multi-space trimming. The existing
same-process separator/hyphen differential continues to compare every Unicode
value, generated flag, and hyphen flag. All seven search-extension tests, all
72 public text tests, all 1,069 unit tests, and `pdfium_all` pass.

The twenty-fourth Phase 7 slice moves text-object base spacing into Rust. Rust
owns the character-space/item-count gates, nonzero-kerning scan, transformed
spacing minimum, negative-base suppression, two-item kerning special case, and
positive/negative character-space adjustment. C++ retains text state, native
matrix distance transforms, and borrowed kerning storage supplied through a
synchronous callback; the complete original helpers remain the separately
selected oracle.

The candidate remains O(k) in the object's kerning count with O(1) auxiliary
storage. Eighteen native Rust text tests cover positive and negative character
space, kerning minima, the two-item special case, and adjustment composition.
The same-process separator/hyphen differential and whitespace fixtures cover
the production path. All seven search-extension tests, all 72 public text
tests, all 1,069 unit tests, and `pdfium_all` pass.

The twenty-fifth Phase 7 slice moves ActualText marked-content policy and
emission planning into Rust. Rust owns pass/done/delay state selection,
repeated-mark suppression, printable-content detection, ASCII control-to-space
replacement, invalid-Unicode skipping, exact RTL/LTR box subdivision, and the
retained replacement-emission list. C++ retains content-mark dictionaries,
text-object and matrix lifetimes, the platform printability predicate, and
native `CharInfo` construction; the complete original state and emission paths
remain the separately selected oracle.

The candidate performs O(n) work and intentionally increases auxiliary storage
from the original loop's O(1) to O(n) for the ActualText length: Rust retains
the replacement plan and C++ stages a second validated emission list so an
invalid plan cannot partially mutate native text buffers. Nineteen native Rust
text tests cover state selection,
control replacement, RTL ordering, and exact boxes. A same-process public
differential compares every character's Unicode, origin availability/value,
and box availability/value on mixed ActualText RTL content. All seven
search-extension tests, all 73 public text tests, all 1,069 unit tests, and
`pdfium_all` pass.

The twenty-sixth Phase 7 slice moves per-character generated-space threshold
policy into Rust. Rust owns the optional space-glyph width calculation, the
font-size one-third cap, accepted-width halving, lazy fallback-width request,
300/500/700 normalization bands, and final font-size scaling. C++ retains font
character-code and width lookup and supplies the fallback glyph width through a
synchronous callback; the complete original helper remains the separately
selected oracle.

The candidate is O(1) in time and auxiliary storage and avoids the fallback
font lookup when the space-glyph threshold is usable. Twenty native Rust text
tests cover accepted space widths, capped widths, missing space glyphs, and
normalized fallback bands. Existing whitespace and separator differentials
cover the production path. All seven search-extension tests, all 73 public text
tests, all 1,069 unit tests, and `pdfium_all` pass.

The twenty-seventh Phase 7 slice moves per-item spacing state and generated
space decisions into Rust. Rust owns kerning-to-spacing conversion, previous
text/space suppression, base-space subtraction, threshold requests, and the
final generate/no-generate decision. C++ retains font and text-buffer access,
native origin/`CharInfo` construction, and validated writes; the complete
original loop remains the separately selected per-item fallback.

The candidate remains O(n) in the text object's item count with O(1) auxiliary
storage and requests the fallback glyph width only when needed. Twenty-two
native Rust text tests cover first items, kerning, base spacing, prior spaces,
and lazy fallback. The expanded same-process separator differential compares
Unicode, generated/hyphen flags, origins, and character boxes exactly. All
seven search-extension tests, all 73 public text tests, all 1,069 unit tests,
and `pdfium_all` pass.

The twenty-eighth Phase 7 slice moves duplicate text-object comparison policy
into Rust. Rust owns empty-rectangle spacing, normalized intersection and width
rules, font-size and item-count equality, per-item character comparison, and
the final position tolerances. C++ retains text-object/font access, the outer
scan of at most five prior objects, glyph-width lookup, and a synchronous item
callback; the complete original comparison remains the separately selected
oracle. Rust explicitly mirrors the native rectangle normalization and
`std::min`/`std::max` ordering, including their special-float behavior.

Each comparison remains O(n) in item count with O(1) auxiliary storage. Twenty-
three native Rust text tests cover overlapping, disjoint, and empty geometry,
item equality, and position tolerance. A synthetic same-process differential
proves sensitivity by expanding from nine to nineteen characters when duplicate
filtering is disabled, then compares the candidate's Unicode, origins, and
character boxes exactly against the C++ oracle. All seven search-extension
tests, all 74 public text tests, all 1,069 unit tests, and `pdfium_all` pass.

The twenty-ninth Phase 7 slice moves generated-character origin planning into
Rust. Rust owns valid-character width admission, text-object versus character-
box font-size selection, the default size for zero-height generated content,
and horizontal origin advancement. C++ retains prior-character and font
access, the form matrix, and native `CharInfo` construction; the complete
original helper remains the separately selected oracle.

The candidate remains O(1) in time and auxiliary storage. Twenty-four native
Rust text tests cover ordinary advancement, invalid characters, missing text
objects, and the size fallback. The exact separator differential is sensitive
to a one-unit candidate-origin perturbation. All seven search-extension tests,
all 74 public text tests, all 1,069 unit tests, and `pdfium_all` pass.

The thirtieth Phase 7 slice moves local duplicate-character suppression into
Rust. Rust owns the reverse scan over at most seven emitted characters,
character-code and font identity matching, origin-distance thresholds, and the
first-item action that trims a preceding generated space. C++ retains native
character/font lifetimes and applies the selected buffer mutation; the complete
original scan remains the separately selected oracle.

The candidate remains O(1) because the scan is bounded at seven characters and
uses O(1) auxiliary storage. Twenty-five native Rust text tests cover matching,
font mismatch, suppression, and space trim. A dedicated overlapping-`TJ`
differential produces exactly one character under both implementations and
compares Unicode, origin, and box values exactly. Twenty same-process
repetitions pass, followed by the complete 82-case text/search matrix. All
seven search-extension tests, all 75 public text tests, all 1,069 unit tests,
and `pdfium_all` pass.

The thirty-first Phase 7 slice moves text-object grouping decisions into Rust.
Rust owns zero-width and duplicate suppression actions, empty-item handling,
line-change thresholding, flush/append selection, and the stable horizontal
insertion index. C++ retains text-object/font/matrix lifetimes, supplies
transformed widths and positions through synchronous callbacks, and applies the
validated vector mutation; the complete original grouping path remains the
separately selected oracle.

The candidate remains O(n) in the current line's grouped object count and uses
O(1) auxiliary storage. Twenty-six native Rust text tests cover skip, append,
flush, front/middle/tail insertion, and equal-X stability. The exact separator
differential compares final order, Unicode, flags, origins, and boxes through
the production grouping path. A dedicated `B`, `A`, then next-line `C` fixture
proves `A`, `B`, `C` output order with exact generated flags, origins, and boxes.
All seven search-extension tests, all 76 public text tests, all 1,069 unit
tests, and `pdfium_all` pass.

The thirty-second Phase 7 slice moves public redaction validation and atomic
removal planning into Rust. Rust validates every requested rectangle, scans
active page objects, preserves strict intersection and full-containment
semantics, rejects unsupported or partially covered objects before mutation,
and returns the exact public status plus stable removal indexes. C++ retains
page and object lifetimes, supplies borrowed geometry through synchronous
callbacks, validates the returned indexes, and applies removals only after the
complete plan succeeds; the original implementation remains the separately
selected oracle and the invalid-page check remains at the public ABI boundary.

For `r` rectangles, `o` page objects, and `k` selected removals, both paths use
O(r + o*r) time and O(r + k) auxiliary storage. Two parser-native Rust tests
cover successful selection, inactive and disjoint objects, invalid input,
partial intersection, unsupported objects, and no partial mutation. A
same-process public differential compares invalid-rectangle, no-content, and
success statuses on identical pages, then compares object count and extracted
text before and after content generation, save, and reload. All 39
parser-native tests, all 1,069 unit tests, and `pdfium_all` pass. The retained
save/render regression reaches the expected object and text state under both
implementations; its four-pixel macOS golden mismatch is identical in the C++
oracle and Rust candidate.

The thirty-third Phase 7 slice moves indexed page-object insertion planning
into Rust. Rust owns bounds validation for the requested index, lazy lookup of
the following object's content-stream number, stream inheritance for new
streamless objects, and the decision to dirty an inherited stream. C++ retains
page-object ownership, supplies the neighboring stream through a synchronous
callback, and applies the validated content-stream, dirty-set, and deque
mutations; the complete original implementation remains the separately
selected oracle.

The candidate remains O(1) outside the retained deque insertion and uses O(1)
auxiliary storage, matching the oracle. One parser-native test covers invalid,
append, inherited-stream, and already-assigned-stream plans. The public
same-process differential compares rejected and successful insertions, object
types, content-stream IDs, and extracted text on identical split-stream pages,
then repeats the comparison after content generation, save, and reload. The
existing cross-stream regression independently proves exact stream ordering.
All 40 parser-native tests, both focused public cases, all 1,069 unit tests,
and `pdfium_all` pass.

The thirty-fourth Phase 7 slice moves public page-object matrix policy and
rotated-bounds geometry into Rust. Rust selects the supported text, path,
image, and form matrix routes, preserves the image-specific comparison against
the original matrix when choosing dirty state, and transforms the four
original-rectangle corners into PDF QuadPoints order for text and image
objects. C++ retains page-object lifetimes, native matrix storage and setters,
and the complete original implementations as the separately selected oracle.

Each route and geometry calculation remains O(1) in time and auxiliary
storage. Two parser-native tests cover every supported and rejected object
type, unchanged and changed image matrices, a non-axis-aligned transform, and
exact corner ordering. A same-process public differential applies the same
nontrivial text matrix under the C++ oracle and Rust candidate, compares all
six matrix values and eight QuadPoints values exactly, then generates content,
saves, reloads, and compares both pages again. The four existing public normal
and rotated text/image coordinate tests also pass. All 42 parser-native tests,
all 1,069 unit tests, and `pdfium_all` pass.

The thirty-fifth Phase 7 slice moves page-object removal lookup and dirty-stream
planning into Rust. Rust scans the stable borrowed object handles in order,
returns the exact removal index and content-stream number for the first match,
and distinguishes a non-member from a callback failure. C++ validates the
returned index, retains `unique_ptr` ownership transfer and deque erasure,
marks only nonnegative content streams dirty, and preserves the complete
original implementation as the separately selected oracle.

The scan remains O(n) in page-object count with O(1) auxiliary storage. One
parser-native test covers first-match selection, streamless objects,
non-members, and callback failure. A same-process public differential compares
the initial object types, stream IDs, and text, successful removal and repeated
non-member rejection under both implementations, then generates content,
saves, reloads, and compares both pages again. The existing basic removal and
add/remove path regressions also pass. All 43 parser-native tests, all 1,069
unit tests, and `pdfium_all` pass.

The thirty-sixth Phase 7 slice moves page-object active-state mutation policy
and holder-wide active counting into Rust. Rust preserves the requested state,
marks the object dirty only when that state changes, and counts active borrowed
objects through an ordered synchronous callback. C++ retains the object fields,
page-object storage and lifetimes, applies the validated update, and preserves
the complete original implementations as the separately selected oracle.

Both operations remain O(1) for one mutation and O(n) for holder counting with
O(1) auxiliary storage. One parser-native test covers unchanged, activation,
deactivation, mixed active lists, and callback failure. A same-process public
differential compares initial and twice-deactivated state, object count, and
public text character count under both implementations, then generates
content, saves, reloads, and compares both pages again. The existing public
active-state save/render regression also passes. All 44 parser-native tests,
all 1,069 unit tests, and `pdfium_all` pass.

The thirty-seventh Phase 7 slice moves public annotation rectangle geometry
and page-rotation planning into Rust. Rust transforms all four rectangle
corners, reduces them in the exact C++ point and comparison order (including
first-point NaN propagation), and preserves signed `% 4` quarter-turn behavior
before converting to dictionary degrees. C++ retains annotation and dictionary
lifetimes, applies the returned rectangle/rotation, updates page dimensions,
and preserves the complete original implementations as the separately
selected oracle.

Both calculations remain O(1) in time and auxiliary storage. One
parser-native test covers a non-axis-aligned transform, positive, negative,
and wrapped rotations, plus NaN reduction. A same-process public differential
creates identical annotations on two pages, applies signed rotation and an
affine transform under the C++ oracle and Rust candidate, compares rectangle
and rotation exactly, then saves, reloads, and compares both pages again. The
existing public rotation and annotation-transform regressions also pass. All
45 parser-native tests, all 1,069 unit tests, and `pdfium_all` pass.

The thirty-eighth Phase 7 slice moves public action type and capability routing
into Rust. Rust maps every internal `CPDF_Action::Type` to the supported public
constant and owns the exact eligibility matrix for destination, file-path,
and URI access. C++ retains action dictionaries, document/destination
lifetimes, file/URI extraction and byte storage, public buffer copying, and the
complete original switch and predicates as the separately selected oracle.

The mapping remains O(1) in time and auxiliary storage. One parser-native test
covers all nineteen internal action values, all five public types, unsupported
values, and every capability combination. A same-process public differential
loads Launch, URI, non-ASCII URI, GoTo, Embedded-GoTo, and unsupported action
fixtures and compares public type, destination availability, and complete file
and URI bytes including terminators under the C++ oracle and Rust candidate.
All eight public action cases, all 46 parser-native tests, all 1,069 unit tests,
and `pdfium_all` pass.

The thirty-ninth Phase 7 slice moves public destination view and XYZ policy
into Rust. Rust owns the complete zoom-mode name table, per-mode parameter
caps, array-size bounding, XYZ structural validity, null-number presence flags,
and the rule that numeric zero means an absent zoom. C++ retains destination
arrays and name/number lifetimes, supplies borrowed decoded slot values,
performs only the conditionally approved output writes, and preserves the
complete original implementations as the separately selected oracle.

Each destination operation remains O(1) in time and auxiliary storage. One
parser-native test covers known and unknown modes, short and oversized arrays,
all validity failures, absent coordinates, and zero zoom. A same-process
public differential compares four named destinations across zoom mode,
parameter count and untouched sentinel slots, XYZ validity, presence flags,
and coordinate/zoom values under the C++ oracle and Rust candidate. All four
public destination cases, all 47 parser-native tests, all 1,069 unit tests, and
`pdfium_all` pass.

The fortieth Phase 7 slice moves public bookmark search traversal into Rust.
Rust owns depth-first child/sibling ordering, visited-set state, early-match
return, and circular-tree guarding. C++ retains bookmark-tree, dictionary, and
title lifetimes, supplies synchronous case-insensitive title comparison and
borrowed child/sibling navigation callbacks, and preserves the complete
original traversal as the separately selected oracle.

A search remains O(n) in time, with O(n) visited-set storage and O(depth)
recursion stack in both implementations. One parser-native test covers exact
depth-first comparison order, a child cycle, and callback failure. A
same-process public differential compares top-level, case-insensitive,
descendant, and missing titles under the C++ oracle and Rust candidate; the
existing circular-bookmark regression now compares both implementations too.
All four focused public bookmark cases, all 48 parser-native tests, all 1,069
unit tests, and `pdfium_all` pass.

The forty-first Phase 7 slice moves public page-label numeric formatting into
Rust. Rust owns decimal output, upper/lower Roman conversion, upper/lower
repeated-letter conversion, the one-million Roman modulo, the thousand-repeat
letter cap, and empty/unknown-style results. C++ retains the page-label number
tree, label dictionaries, prefixes, native strings, and the complete original
formatters as the separately selected oracle. Negative alphabetic inputs are
rejected at the candidate boundary and use that oracle because the PDF format
forbids them but the historical native formatter has platform-sensitive
malformed-input behavior.

Formatting remains O(output) in time and storage, with both non-decimal output
families bounded by the retained historical caps. One parser-native test
covers every style plus zero, modulo, unknown, and rejected-negative cases. A
same-process unit differential compares all 10,003 indices from -1 through
10,001 in the three-level number-tree fixture. A public differential compares
required lengths, complete UTF-16 buffers, and untouched sentinels for indices
-1 through 8. All three focused page-label unit cases, all three public cases,
all 49 parser-native tests, all 1,070 unit tests, and `pdfium_all` pass.

The forty-second Phase 7 slice moves public link-annotation enumeration into
Rust. Rust owns signed cursor normalization, the forward annotation-index scan,
first-link selection, and distinct miss/callback-failure states. C++ retains
page and annotation-array lifetimes, dictionary and subtype access through a
synchronous callback, selected public handles, and the complete original loop
as the separately selected oracle.

Each call remains O(n - cursor) in time and O(1) in auxiliary storage. One
parser-native test covers skipped annotations, resumed enumeration, negative
and end cursors, and callback failure. A same-process public differential
compares all four selected handles, every advanced cursor, terminal state,
negative cursor, and oversized cursor under the C++ oracle and Rust candidate.
Both focused public link cases, all 50 parser-native tests, all 1,070 unit
tests, and `pdfium_all` pass.

The forty-third Phase 7 slice moves both document number-tree searches into
Rust. Rust owns exact-key and greatest-key-at-most traversal, `/Limits`
pruning, `/Nums` precedence, forward exact-search ordering, reverse lower-bound
ordering, and shared/circular-node guarding. C++ retains dictionary, array, and
value lifetimes, supplies synchronous borrowed node/entry/child callbacks, and
preserves both complete original recursive traversals as separately selected
oracles.

Valid-tree traversal remains O(nodes + entries) in expected time and O(depth)
stack. The candidate deliberately adds O(nodes) visited-set storage so cyclic
or shared malformed graphs terminate; hash-backed membership keeps the added
work expected-linear (`b2bff4e05`). One parser-native test covers limits,
forward and reverse order, exact and lower-bound results, misses, and a child
cycle. Same-process differentials compare both operations
over all 10,003 keys from -1 through 10,001 in the three-level page-label tree.
All four page-label unit cases, all 21 StructTree public cases, all 51
parser-native tests, all 1,071 unit tests, and `pdfium_all` pass.

The forty-fourth Phase 7 slice moves public destination page-target resolution
into Rust. Rust owns numeric, dictionary, and invalid target classification,
direct numeric results, and exact admission of document-index lookup. C++
retains destination objects and document page-index state, supplies the
dictionary object number through one synchronous callback, and preserves the
complete original branches as the separately selected oracle.

Routing remains O(1) in time and storage; dictionary targets retain the
existing document page-index lookup complexity. One parser-native test covers
direct numbers, dictionary callback admission, invalid targets, and callback
failure. The four-name same-process destination differential now also compares
page indices for a direct number, valid page reference, alternate number, and
invalid reference. The three page-cache regressions and public link-target path
also pass. All seven focused cases, all 52 parser-native tests, all 1,071 unit
tests, and `pdfium_all` pass.

The forty-fifth Phase 7 slice moves read-only name-tree counting and indexed
lookup into Rust. Rust owns leaf-pair counting, depth-first child order,
cumulative pair offsets, target-leaf/pair selection, the historical 32-level
bound, and cycle guarding for count traversal. C++ retains dictionaries,
arrays, decoded name strings and values, supplies synchronous borrowed node and
child callbacks, and preserves the complete original count and index
traversals as separately selected oracles. Name-based lookup and all
insert/delete mutation remain C++ for the next slices.

Both operations remain O(nodes) in time. Indexed lookup uses O(depth) stack;
counting also uses O(nodes) visited-set storage, matching its retained cycle
guard. One parser-native test covers multiple leaves, exact pair order,
out-of-range lookup, depth cutoff, and a count cycle. A same-process unit
differential compares total count, all five values/names, and two misses in the
three-level fixture. All seven NameTree unit cases and all twelve public
Attachment cases, including mutation and save/reload, pass. Evidence also
includes all 53 parser-native tests, all 1,072 unit tests, and `pdfium_all`.

The forty-sixth Phase 7 slice moves public link hit testing into Rust. Rust
owns reverse z-order scanning, rectangle normalization, inclusive point
containment, topmost-link selection, and distinct miss/callback-failure state.
C++ retains the page link cache, annotation dictionaries and lifetimes,
supplies borrowed rectangles synchronously, materializes the selected link,
and preserves the complete original loop as the separately selected oracle.

Each query remains O(n) in link count and O(1) in auxiliary storage. One
parser-native test covers overlapping links, inverted rectangles, inclusive
edges, null entries, misses, NaN, and callback failure. A same-process public
differential compares exact link handles and z-order for two hits and two
misses. The existing destination and Link-to-Annotation regressions also pass.
All three focused public cases, all 54 parser-native tests, all 1,072 unit
tests, and `pdfium_all` pass.

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
