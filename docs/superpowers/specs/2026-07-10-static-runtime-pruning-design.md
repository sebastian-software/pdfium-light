# Static runtime pruning design

## Decision

pdfium-light removes the optional Skia renderer and its coupled experimental
Fontations/Skrifa integration. AGG and FreeType are the single current
graphics and font stack.

This is a current-scope decision, not a permanent choice for the eventual Rust
port. The public C API remains stable while implementation modules may migrate
incrementally. When the font subsystem is migrated, the project will evaluate a
then-current Fontations release and a Rust rasterization path against the
retained corpus with documented image tolerances.

## Scope

- Remove Skia, Fontations/Skrifa, Rust-PNG, renderer-selection API, their
  sources, tests, test expectations, build flags, and dependencies.
- Remove unreachable XFA-only CSS and image/progressive codec sources and the
  XFA codec configuration knobs.
- Remove incremental data-availability and pause/continue rendering APIs.
- Use the existing synchronous `CPDF_RenderContext::Render()` path for normal
  bitmap rendering.
- Retain `CPDF_ReadValidator` and linearized-document parsing support where
  the normal parser still uses them.

## Validation

- The local light validation gate must cover the reduced public header set and
  synchronous rendering smoke markers.
- Repository searches must find no active Skia, Fontations/Skrifa, XFA codec,
  data-availability, or progressive-rendering implementation surface.
- A full GN/Ninja light build remains the release-quality follow-up in a
  complete depot_tools checkout.
