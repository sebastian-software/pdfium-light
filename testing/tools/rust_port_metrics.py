#!/usr/bin/env python3
# Copyright 2026 Sebastian Werner
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

"""Reports reproducible Rust migration metrics for pdfium-light.

The report deliberately counts physical lines in tracked first-party source
files. It is a migration-progress signal, not a statement about feature
completeness, runtime performance, or binary size.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


PRODUCT_DIRS = ("core", "fpdfsdk", "public")
NATIVE_SUFFIXES = (".c", ".cc", ".cpp", ".h", ".rs")
IMPLEMENTATION_SUFFIXES = (".c", ".cc", ".cpp")
RUST_SUFFIX = ".rs"
ADAPTER_SUFFIXES = (".cpp", ".h")
MAPPING_DATA_DIR = "core/fpdfapi/cmaps"
ABI_BEGIN = "// RUST_PORT_METRICS_BEGIN abi_thunk"
ABI_END = "// RUST_PORT_METRICS_END abi_thunk"
GENERATED_MARKER = "@generated"
ACTIVE_SURFACES = (
    "ASCII85 encode/decode",
    "ASCIIHex decode",
    "LZW decode",
    "CCITT Fax Group 4 and scanline decode",
    "PNG/TIFF predictor transforms",
    "RunLength encode/decode",
    "all PDF blend modes across BGRA and opaque BGR/BGRx row compositing",
    "byte and bit mask row compositing",
    "1-bit and 8-bit palette row compositing",
    "Adobe CMYK scalar and row conversion",
    "BGRA bitmap alpha mutations",
    "bitmap clearing across retained formats",
    "BGR/BGRx/BGRA bitmap color scaling",
    "bitmap pitch and allocation-size calculation",
    "1-bpp mask expansion and bitmap span population",
    "equal-format 1-bpp and multi-BPP bitmap transfers",
    "source/destination overlap and clip geometry",
    "default and custom palette primitives",
    "mask, indexed, grayscale, and RGB buffer conversion matrix",
    "1-bpp mask OR compositing",
    "solid rectangle compositing across retained bitmap formats",
    "BGRA alpha-mask row extraction",
    "aligned and shifted bitmap clipping",
    "complete bitmap scanline copies",
    "stretch-engine weight-table layout and fixed-point weights",
    "horizontal stretch pixel transforms across retained formats",
    "vertical stretch filtering and alpha unpremultiplication",
    "bitmap transposition with independent axis mirroring",
    "fixed-matrix bilinear alpha image transforms",
    "fixed-matrix bilinear indexed and grayscale image transforms",
    "fixed-matrix bilinear BGR, BGRx, BGRA, and raw CMYK image transforms",
    "1-bpp stretch palette interpolation",
    "bitmap row flipping across retained pixel widths",
    "public render flag request planning",
    "page-object render command dispatch",
    "page-object list stop, activity, and clip planning",
    "render-layer setup, completion, and ordered iteration",
    "path fill, stroke, and forced-color paint planning",
    "path matrix availability validation",
    "path fill and graph-state option planning",
    "AGG dash-pattern applicability planning",
    "AGG stroke cap, join, width, and miter planning",
    "AGG dash-value and phase normalization",
    "AGG path transform, clipping, and command emission",
    "AGG path fill-rule and stroke-mode orchestration",
    "AGG stroke matrix decomposition",
    "glyph bitmap cache-key shape planning",
    "checked glyph bitmap origin placement",
    "glyph device-origin rounding",
    "glyph bitmap bounding-box aggregation",
    "glyph bitmap lookup and native fallback action planning",
    "glyph path and width cache key planning",
    "FreeType glyph load-flag planning",
    "PDF text render dispatch planning",
    "PDF text pattern-path planning",
    "PDF text matrix availability validation",
    "PDF text backend route planning",
    "PDF stroked-text device-matrix adjustment planning",
    "PDF path-text fill-option planning",
    "PDF text backend execution",
    "cross-reference stream big-endian field reading",
    "cross-reference stream object-type validation",
    "cross-reference stream effective entry-type planning",
    "cross-reference stream entry action planning",
    "cross-reference stream index-pair validation",
    "cross-reference stream segment byte-range planning",
    "cross-reference stream segment entry iteration",
    "cross-reference stream field-width conversion",
    "cross-reference stream entry field collection",
    "complete simple PDF token scanning",
    "cross-reference stream mutation orchestration",
    "cross-reference table storage, mutation, sizing, and overlay",
    "indirect-object numbering, recursive parse state, indexing, and iteration",
    "PDF number value storage, parsing, conversion, and cloning",
    "PDF boolean value storage, mutation, and cloning",
    "PDF reference object-number storage, mutation, and cloning",
    "PDF array slot ordering, mutation, lookup, and iteration",
    "PDF dictionary key storage, ordering, mutation, lookup, and iteration",
    "ByteStringPool binary-key interning index and handle reuse",
    "PDF string binary value, mutation, clone, and hex-mode state",
    "PDF name binary value, mutation, clone, and encoding state",
    "PDF stream in-memory byte storage, mutation, sizing, and cloning",
    "PDF object slot membership and last-reference lifetime decisions",
    "document page object-number indexing, caching, insertion, removal, and lookup",
    "public document page-move validation and deletion-order planning",
    "document page-tree count traversal, cycle guarding, normalization, and limits",
    "document page-tree object-number lookup and skip-count traversal",
    "public SDK page-range validation, expansion, ordering, and duplication",
)
CANDIDATE_SURFACES = (
    "Phase 7 edit, document, and SDK behavior plus Phase 8 fxcrt consolidation",
)


def tracked_paths(root: Path) -> tuple[Path, ...]:
    result = subprocess.run(
        ("git", "ls-files", "-z"),
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
    )
    return tuple(
        root / relative_path
        for relative_path in result.stdout.decode("utf-8").split("\0")
        if relative_path
    )


def is_product_source(path: Path, root: Path) -> bool:
    relative_path = path.relative_to(root)
    return (
        relative_path.parts[0] in PRODUCT_DIRS
        and relative_path.suffix in NATIVE_SUFFIXES
    )


def is_cxx_rust_adapter(path: Path, root: Path) -> bool:
    relative_path = path.relative_to(root)
    return "rust" in relative_path.parent.parts and path.suffix in ADAPTER_SUFFIXES


def count_physical_lines(paths: tuple[Path, ...]) -> int:
    return sum(
        sum(1 for _ in path.open(encoding="utf-8"))
        for path in paths
    )


def count_abi_thunk_lines(paths: tuple[Path, ...]) -> int:
    count = 0
    for path in paths:
        in_abi_thunk = False
        for line in path.read_text(encoding="utf-8").splitlines():
            marker = line.strip()
            if marker == ABI_BEGIN:
                if in_abi_thunk:
                    raise ValueError(f"nested ABI metric region in {path}")
                in_abi_thunk = True
                count += 1
                continue
            if marker == ABI_END:
                if not in_abi_thunk:
                    raise ValueError(f"unmatched ABI metric region end in {path}")
                in_abi_thunk = False
                count += 1
                continue
            if in_abi_thunk:
                count += 1
        if in_abi_thunk:
            raise ValueError(f"unterminated ABI metric region in {path}")
    return count


def is_generated_rust(path: Path) -> bool:
    with path.open(encoding="utf-8") as source:
        return any(GENERATED_MARKER in line for _, line in zip(range(20), source))


def collect_metrics(root: Path) -> dict[str, int | tuple[str, ...]]:
    paths = tracked_paths(root)
    product_sources = tuple(path for path in paths if is_product_source(path, root))
    rust_sources = tuple(path for path in product_sources if path.suffix == RUST_SUFFIX)
    generated_rust_sources = tuple(
        path for path in rust_sources if is_generated_rust(path)
    )
    authored_rust_sources = tuple(
        path for path in rust_sources if path not in generated_rust_sources
    )
    generated_rust_loc = count_physical_lines(generated_rust_sources)
    rust_abi_thunk_loc = count_abi_thunk_lines(authored_rust_sources)
    rust_loc = count_physical_lines(rust_sources)
    rust_authored_behavior_loc = rust_loc - generated_rust_loc - rust_abi_thunk_loc
    adapter_sources = tuple(
        path
        for path in product_sources
        if is_cxx_rust_adapter(path, root)
    )
    cxx_behavior_sources = tuple(
        path
        for path in product_sources
        if path.suffix in IMPLEMENTATION_SUFFIXES
        and path not in adapter_sources
        and not path.relative_to(root).as_posix().startswith(f"{MAPPING_DATA_DIR}/")
    )
    cxx_behavior_loc = count_physical_lines(cxx_behavior_sources)
    behavior_implementation_loc = cxx_behavior_loc + rust_authored_behavior_loc
    declaration_sources = tuple(
        path for path in product_sources if path.suffix == ".h"
    )
    mapping_data_sources = tuple(
        path
        for path in product_sources
        if path.relative_to(root).as_posix().startswith(f"{MAPPING_DATA_DIR}/")
    )
    return {
        "product_native_loc": count_physical_lines(product_sources),
        "rust_loc": rust_loc,
        "rust_authored_behavior_loc": rust_authored_behavior_loc,
        "rust_abi_thunk_loc": rust_abi_thunk_loc,
        "generated_rust_loc": generated_rust_loc,
        "adapter_loc": count_physical_lines(adapter_sources),
        "declaration_loc": count_physical_lines(declaration_sources),
        "mapping_data_loc": count_physical_lines(mapping_data_sources),
        "behavior_implementation_loc": behavior_implementation_loc,
        "cxx_behavior_loc": cxx_behavior_loc,
        "active_surfaces": ACTIVE_SURFACES,
        "candidate_surfaces": CANDIDATE_SURFACES,
    }


def format_report(metrics: dict[str, int | tuple[str, ...]]) -> str:
    product_native_loc = int(metrics["product_native_loc"])
    rust_loc = int(metrics["rust_loc"])
    rust_authored_behavior_loc = int(metrics["rust_authored_behavior_loc"])
    rust_abi_thunk_loc = int(metrics["rust_abi_thunk_loc"])
    generated_rust_loc = int(metrics["generated_rust_loc"])
    adapter_loc = int(metrics["adapter_loc"])
    declaration_loc = int(metrics["declaration_loc"])
    mapping_data_loc = int(metrics["mapping_data_loc"])
    behavior_implementation_loc = int(metrics["behavior_implementation_loc"])
    cxx_behavior_loc = int(metrics["cxx_behavior_loc"])
    rust_percentage = 100 * rust_loc / product_native_loc
    behavior_rust_percentage = (
        100 * rust_authored_behavior_loc / behavior_implementation_loc
    )
    active_surfaces = ", ".join(metrics["active_surfaces"])
    candidate_surfaces = ", ".join(metrics["candidate_surfaces"])
    return "\n".join(
        (
            f"product_native_loc: {product_native_loc}",
            f"rust_loc: {rust_loc}",
            f"rust_authored_behavior_loc: {rust_authored_behavior_loc}",
            f"rust_abi_thunk_loc: {rust_abi_thunk_loc}",
            f"generated_rust_loc: {generated_rust_loc}",
            f"cxx_adapter_loc: {adapter_loc}",
            f"declaration_loc: {declaration_loc}",
            f"mapping_data_loc: {mapping_data_loc}",
            f"behavior_implementation_loc: {behavior_implementation_loc}",
            f"cxx_behavior_loc: {cxx_behavior_loc}",
            f"rust_owned_boundary_loc: {rust_loc + adapter_loc}",
            f"physical_rust_percentage: {rust_percentage:.2f}%",
            f"behavior_rust_percentage: {behavior_rust_percentage:.2f}%",
            f"active_surfaces: {active_surfaces}",
            f"candidate_surfaces: {candidate_surfaces}",
        )
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="validate that the report has a non-empty source denominator",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    root = Path(__file__).resolve().parents[2]
    try:
        metrics = collect_metrics(root)
    except (OSError, subprocess.CalledProcessError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if args.check and (
        not metrics["product_native_loc"]
        or not metrics["rust_loc"]
        or not metrics["rust_authored_behavior_loc"]
        or not metrics["behavior_implementation_loc"]
        or not metrics["active_surfaces"]
        or any(
            int(metrics[name]) < 0
            for name in (
                "rust_authored_behavior_loc",
                "rust_abi_thunk_loc",
                "generated_rust_loc",
            )
        )
        or metrics["rust_loc"]
        != metrics["rust_authored_behavior_loc"]
        + metrics["rust_abi_thunk_loc"]
        + metrics["generated_rust_loc"]
    ):
        print("error: Rust migration metrics are incomplete", file=sys.stderr)
        return 1

    print(format_report(metrics))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
