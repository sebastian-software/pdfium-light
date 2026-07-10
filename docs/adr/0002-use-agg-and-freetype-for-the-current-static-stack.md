# ADR 0002: Use AGG and FreeType for the current static stack

## Status

Accepted

## Context

pdfium-light exposes a stable C API while its C++ implementation is reduced
and then ported module by module to Rust. Upstream PDFium contains an
experimental Skia raster backend and an experimental Fontations/Skrifa bridge.
They add a selectable runtime architecture, but neither is required for the
supported static-document API today.

## Decision

The current rendering stack is AGG for CPU rasterization and FreeType for font
handling. Remove Skia, the current Fontations/Skrifa bridge, Rust-PNG support,
and their public configuration and runtime-selection APIs.

This is a decision about the current C++ implementation, not a permanent
choice of font technology. The stable C API remains the boundary while C++
modules are ported incrementally.

## Consequences

There is one supported rendering path, no runtime renderer or font-backend
selection, and no Skia checkout/build dependency. Output compatibility is
measured with the project corpus using defined, documented tolerances where
pixel-exact identity is not meaningful.

When the font module becomes a Rust-porting target, evaluate the then-current
Fontations release and the appropriate Rust rasterization path against that
corpus and those tolerances. Do not revive this removed experimental bridge or
freeze its current dependency versions merely to avoid that later evaluation.
