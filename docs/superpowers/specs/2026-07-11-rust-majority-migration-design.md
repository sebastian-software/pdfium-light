# Rust-majority migration design

## Status

Approved direction. This document is a decision record and planning input; it
does not authorize implementation work on the listed phases by itself.

## Objective

Move the behavior-owning implementation of pdfium-light predominantly to Rust
without changing the supported C API or turning the work into a greenfield PDF
engine. The working direction is an aggressive Rust-majority programme, with
80% as a progress target rather than an acceptance shortcut.

At merge commit `3955312c6`, the physical product-source metric is 1,232 Rust
LOC of 255,525 product-native LOC (0.48%). An unchanged denominator would put
an 80% physical-source share at about 204,420 Rust LOC. That is deliberately a
dashboard estimate: the denominator includes C++ headers and the thin ABI
boundary which must remain while the public C API is supported.

Two values remain visible throughout the programme:

1. Physical Rust share, using `testing/tools/rust_port_metrics.py`.
2. Rust share of behavior-owning implementation, with public declarations,
   ABI thunks, and unavoidable system-backend adapters reported separately.

No phase is accepted merely by increasing either number.

## Non-negotiable compatibility contract

- The supported public C API, static-library contract, and light feature scope
  remain unchanged.
- A retained C++ implementation is the differential oracle until its Rust
  replacement has passed the phase gate.
- For a 1:1 renderer port, pixel tolerance is exactly zero: same input, same
  platform, and same build must produce identical bitmap dimensions, format,
  stride, and bytes.
- AGG and FreeType remain the current raster/font backends. Rust may call them
  through a narrow native boundary; replacing either backend is a later,
  separately approved compatibility project.
- A difference in bytes, parsed objects, errors, source offsets, memory
  bounds, or public result is a defect until an explicit ADR changes the
  contract.
- Fuzzers and corpus fixtures are first-class acceptance evidence, not
  post-port cleanup.

## Migration shape

The migration proceeds as vertical, differential slices. Each slice may begin
by accepting C++-owned inputs at an internal boundary, then move ownership
inward only after the observed behavior is proven. This permits the renderer
to move before the parser without creating a second PDF implementation.

| Phase | Rust-owned scope | Retained C++ at entry | Primary proof |
| --- | --- | --- | --- |
| 0 | Differential platform, corpus manifest, metrics split | Entire product | Reproducible C++/Rust dual runs |
| 1 | `fxge/dib`: bitmap, color, blend, compositing primitives | Page/parser/render orchestration | Exact bitmap bytes |
| 2 | Render command plan and page rendering orchestration | C++ object graph and parser | Exact bitmap bytes and render traces |
| 3 | Paths plus AGG adapter boundary | AGG raster backend | Exact path-rendering corpus |
| 4 | Glyph planning, caches, and FreeType adapter boundary | FreeType backend | Exact glyph and text-rendering corpus |
| 5 | `fpdfapi/page`, `render`, and font-facing PDF logic | C++ parser/object graph | Object, render, and error parity |
| 6 | Parser and PDF object graph | C ABI and native backends | Object-graph, error, and save/reload parity |
| 7 | Edit, text, document logic, and SDK implementation | Thin C ABI wrappers | Public API and save/reload parity |
| 8 | `fxcrt` consolidation and remaining implementation cleanup | Explicit ABI/system shim inventory | No unowned C++ behavior remains |

The expected largest physical contributors are `core/fpdfapi` (about 79k
lines), `core/fxge` (about 66k), `core/fxcrt` (about 31k), and `fpdfsdk`
(about 28k). `fpdfapi/cmaps` is roughly 27k lines of mostly mapping data. If
it is generated for Rust, generated data must be reported separately from
authored behavior so that it cannot inflate the migration claim.

## Required decision record for every implementation epic

Before opening a production-port PR, its parent issue records:

1. The public entry points and internal ownership boundary.
2. The retained C++ oracle and how the candidate reaches it.
3. Exact observable contract: bytes, objects, errors, offsets, resource
   bounds, and pixel output as applicable.
4. The allowed backend calls and third-party version assumptions.
5. The corpus/fuzzer inputs that prove normal, malformed, truncated, and
   boundary behavior.
6. The activation switch and rollback shape.
7. A list of intentionally retained C++ code after the slice.

An activation that changes asymptotic memory or latency behavior must state
that trade-off in the issue and docs before merge. The eager Fax decoder is an
example of a trade-off that must be explicit rather than silently inherited.

## Validation gates

Every phase uses the common local and full gates in `docs/validation.md`, plus
the following focused evidence:

| Surface | Required evidence |
| --- | --- |
| Renderer | C++/Rust same-process bitmap comparison with zero tolerance, per supported platform and pinned backend configuration |
| Parser/object model | Differential object snapshots, exact error/status checks, malformed/truncated corpus, and parser fuzzing |
| Edit/save | Public API tests, save/reload byte or semantic comparisons, and document mutation corpus |
| Text/font | Glyph, extraction, search, geometry, and text-rendering differential cases |
| Resource behavior | Explicit allocation/CPU limits for malformed inputs, sanitizer runs, and fuzzing regression corpus |
| Public boundary | Retained public-header compile test, API absence checks, and ABI-thunk inventory |

The full gate includes an `enable_rust=true` light build, `pdfium_all`, unit
tests, embedder tests, retained fuzzers, and the local light validation gate.
No CI workflow is assumed; each PR body records the exact local commands and
results until a minimal workflow is introduced separately.

## Initial GitHub epic breakdown

The first Rust-majority epic should create independent, ordered child issues:

1. Build the zero-tolerance renderer differential harness and corpus manifest.
2. Port bitmap, color, and compositing primitives behind the harness.
3. Port the Rust render-command plan while C++ supplies page objects.
4. Port path processing and the AGG adapter boundary.
5. Port glyph planning/cache behavior and the FreeType adapter boundary.
6. Activate Rust page/render orchestration with C++ parser input.
7. Establish parser/object-model differential corpus and fuzzing baseline.
8. Port parser/object ownership in bounded sub-slices.
9. Publish a measured post-epic decision on edit/text/document/SDK ordering.

The issues remain separate PRs or small grouped PRs only where they share an
activation boundary. Each remains mergeable on its own and updates the parent
tracker with its measured Rust share and validation record.

## Explicit deferrals

- Replacing AGG or FreeType.
- Relaxing renderer comparison to a nonzero tolerance.
- Counting generated mapping data as authored behavior.
- Large rewrites that remove the C++ oracle before the Rust candidate is
  proven.
- Any public API expansion or C++ ABI break.
