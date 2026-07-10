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
RUST_SUFFIX = ".rs"
ADAPTER_DIR = "core/fxcodec/rust"
ADAPTER_SUFFIXES = (".cpp", ".h")
ACTIVE_SURFACES = (
    "ASCII85 encode/decode",
    "ASCIIHex decode",
    "LZW decode",
    "CCITT Fax Group 4 and scanline decode",
    "PNG/TIFF predictor transforms",
    "RunLength encode/decode",
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


def count_physical_lines(paths: tuple[Path, ...]) -> int:
    return sum(
        sum(1 for _ in path.open(encoding="utf-8"))
        for path in paths
    )


def collect_metrics(root: Path) -> dict[str, int | tuple[str, ...]]:
    paths = tracked_paths(root)
    product_sources = tuple(path for path in paths if is_product_source(path, root))
    rust_sources = tuple(path for path in product_sources if path.suffix == RUST_SUFFIX)
    adapter_sources = tuple(
        path
        for path in paths
        if path.relative_to(root).parent.as_posix() == ADAPTER_DIR
        and path.suffix in ADAPTER_SUFFIXES
    )
    return {
        "product_native_loc": count_physical_lines(product_sources),
        "rust_loc": count_physical_lines(rust_sources),
        "adapter_loc": count_physical_lines(adapter_sources),
        "active_surfaces": ACTIVE_SURFACES,
    }


def format_report(metrics: dict[str, int | tuple[str, ...]]) -> str:
    product_native_loc = int(metrics["product_native_loc"])
    rust_loc = int(metrics["rust_loc"])
    adapter_loc = int(metrics["adapter_loc"])
    rust_percentage = 100 * rust_loc / product_native_loc
    active_surfaces = ", ".join(metrics["active_surfaces"])
    return "\n".join(
        (
            f"product_native_loc: {product_native_loc}",
            f"rust_loc: {rust_loc}",
            f"cxx_adapter_loc: {adapter_loc}",
            f"rust_owned_boundary_loc: {rust_loc + adapter_loc}",
            f"rust_percentage: {rust_percentage:.2f}%",
            f"active_surfaces: {active_surfaces}",
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
    except (OSError, subprocess.CalledProcessError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    if args.check and (
        not metrics["product_native_loc"]
        or not metrics["rust_loc"]
        or not metrics["active_surfaces"]
    ):
        print("error: Rust migration metrics are incomplete", file=sys.stderr)
        return 1

    print(format_report(metrics))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
