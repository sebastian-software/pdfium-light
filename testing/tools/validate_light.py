#!/usr/bin/env python3
# Copyright 2026 Sebastian Werner
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

"""Runs the local pdfium-light validation gate.

The full build gate still requires a complete depot_tools/gclient checkout.
This script covers the repeatable checks that can run from the reduced
pdfium-light repository: retained public header compilation, sample
compilation, removed API absence, retained target naming, and smoke-test source
coverage for the light feature set.
"""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path


REMOVED_PUBLIC_HEADERS = (
    "public/fpdf_formfill.h",
    "public/fpdf_fwlevent.h",
    "public/fpdf_javascript.h",
    "public/fpdf_dataavail.h",
    "public/fpdf_progressive.h",
)

REMOVED_PUBLIC_TOKENS = (
    "FPDF_FORMHANDLE",
    "FPDF_FORMFILLINFO",
    "IPDF_JSPLATFORM",
    "FPDF_FFLDraw",
    "FORMTYPE_XFA",
    "FPDF_GetXFAPacket",
    "FPDF_LoadXFA",
    "FPDFDOC_GetPageAAction",
    "FPDFDOC_GetJavaScriptAction",
    "FPDFPage_GetAA",
    "FPDFAvail_",
    "FPDF_RenderPageBitmap_Start",
    "FPDF_RenderPage_Continue",
    "FPDF_RenderPage_Close",
)

RETAINED_BUILD_PATTERNS = {
    "light library target": ("BUILD.gn", 'component("pdfium")'),
    "public header compile target": (
        "BUILD.gn",
        'source_set("pdfium_light_public_headers_test")',
    ),
    "unit test binary": ("BUILD.gn", 'test("pdfium_unittests")'),
    "embedder test binary": ("BUILD.gn", 'test("pdfium_embeddertests")'),
    "focused light validation target": (
        "BUILD.gn",
        'group("pdfium_light_validation")',
    ),
    "broader retained aggregate target": ("BUILD.gn", 'group("pdfium_all")'),
    "fpdfsdk embedder test target": (
        "fpdfsdk/BUILD.gn",
        'pdfium_embeddertest_source_set("embeddertests")',
    ),
}

REMOVED_BUILD_PATTERNS = (
    'executable("pdfium_test")',
    'test("pdfium_test")',
    'group("pdfium_test")',
    '":fxjs"',
    '"//fxjs"',
)

SMOKE_COVERAGE = {
    "public header surface": (
        "testing/light_api_headers_test.cc",
        (
            'include "public/fpdfview.h"',
            'include "public/fpdf_text.h"',
            'include "public/fpdf_edit.h"',
            'include "public/fpdf_save.h"',
            "CompileStaticAnnotationSurface",
            "CompileRedactionSurface",
        ),
    ),
    "static rendering": (
        "fpdfsdk/fpdf_view_embeddertest.cpp",
        (
            "RenderHelloWorldWithFlags",
            "RenderManyRectanglesWithFlags",
            "TestRenderPageBitmapWithFlags",
        ),
    ),
    "text extraction": (
        "fpdfsdk/fpdf_text_embeddertest.cpp",
        (
            "TEST_F(FPDFTextEmbedderTest, Text)",
            "TEST_F(FPDFTextEmbedderTest, GetText)",
            "FPDFText_GetText",
        ),
    ),
    "edit save remove and insert": (
        "fpdfsdk/fpdf_edit_embeddertest.cpp",
        (
            "RemoveTextObject",
            "InsertPageObjectAndSave",
            "InsertObjectAtIndexPersistsOrder",
            "FPDF_SaveAsCopy",
        ),
    ),
    "ordinary annotations": (
        "fpdfsdk/fpdf_view_embeddertest.cpp",
        (
            "RenderAnnotationWithPrintingFlag",
            "FPDF_ANNOT",
            "bug_1658_annot",
        ),
    ),
    "annotation inspection": (
        "fpdfsdk/fpdf_text_embeddertest.cpp",
        (
            "AnnotLinks",
            "FPDFPage_GetAnnotCount",
            "FPDFAnnot_GetSubtype",
        ),
    ),
    "real redaction": (
        "fpdfsdk/fpdf_edit_embeddertest.cpp",
        (
            "ApplyRedactionsRemovesCoveredTextAndSaves",
            "ApplyRedactionsRejectsPartialTextIntersection",
            "ExtractPageText",
            "FPDF_REDACTION_ERROR_UNSAFE_PARTIAL_INTERSECTION",
        ),
    ),
}

SYNTAX_COMMANDS = (
    (
        "retained public headers",
        "clang++",
        (
            "clang++",
            "-std=c++20",
            "-DPDFIUM_LIGHT",
            "-I.",
            "-Ithird_party/googletest/src/googletest/include",
            "-Ithird_party/googletest/src/googlemock/include",
            "-fsyntax-only",
            "testing/light_api_headers_test.cc",
        ),
    ),
    (
        "static sample",
        "clang",
        (
            "clang",
            "-std=c11",
            "-DPDFIUM_LIGHT",
            "-I.",
            "-fsyntax-only",
            "samples/simple_no_v8.c",
        ),
    ),
)


def read_text(root: Path, relative_path: str) -> str:
    path = root / relative_path
    if not path.is_file():
        raise AssertionError(f"missing required file: {relative_path}")
    return path.read_text(encoding="utf-8")


def parse_light_public_headers(root: Path) -> list[str]:
    build_gn = read_text(root, "BUILD.gn")
    match = re.search(
        r"pdfium_light_public_headers\s*=\s*\[(.*?)\]",
        build_gn,
        re.DOTALL,
    )
    if not match:
        raise AssertionError("BUILD.gn does not define pdfium_light_public_headers")
    return re.findall(r'"([^"]+)"', match.group(1))


def check_public_headers(root: Path) -> None:
    headers = parse_light_public_headers(root)
    if not headers:
        raise AssertionError("pdfium_light_public_headers is empty")

    header_set = set(headers)
    for header in headers:
        if not (root / header).is_file():
            raise AssertionError(f"listed public header is missing: {header}")

    for removed_header in REMOVED_PUBLIC_HEADERS:
        if removed_header in header_set:
            raise AssertionError(f"removed header is still exported: {removed_header}")
        if (root / removed_header).exists():
            raise AssertionError(f"removed header still exists: {removed_header}")

    for header in headers:
        text = read_text(root, header)
        for token in REMOVED_PUBLIC_TOKENS:
            if token in text:
                raise AssertionError(f"{header} still exposes removed token {token}")


def check_build_targets(root: Path) -> None:
    for label, (relative_path, pattern) in RETAINED_BUILD_PATTERNS.items():
        text = read_text(root, relative_path)
        if pattern not in text:
            raise AssertionError(f"missing retained {label}: {pattern}")

    for relative_path in ("BUILD.gn", "fpdfsdk/BUILD.gn", "testing/BUILD.gn"):
        text = read_text(root, relative_path)
        for pattern in REMOVED_BUILD_PATTERNS:
            if pattern in text:
                raise AssertionError(f"{relative_path} still references {pattern}")


def check_platform_cleanup(root: Path) -> None:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    )
    files = result.stdout.splitlines()
    x86_expectations = [
        path
        for path in files
        if path.startswith("testing/resources/embedder_tests/")
        and ("_mac_x86" in path or path.endswith("_x86.png"))
    ]
    if x86_expectations:
        raise AssertionError(
            "stale x86-only golden expectations remain: "
            + ", ".join(sorted(x86_expectations))
        )

    for relative_path in (
        "testing/embedder_test.cpp",
        "testing/embedder_test.h",
        "testing/SUPPRESSIONS",
        "testing/SUPPRESSIONS_EXACT_MATCHING",
        "testing/SUPPRESSIONS_IMAGE_DIFF",
        "testing/tools/common.py",
        "testing/tools/suppressor.py",
    ):
        text = read_text(root, relative_path)
        for marker in (
            "GetCpuArchSuffix",
            "mac_x86",
            "_mac_x86.png",
            "return 'x86'",
        ):
            if marker in text:
                raise AssertionError(
                    f"{relative_path} still contains unsupported-platform marker {marker}"
                )


def check_smoke_coverage(root: Path) -> None:
    for label, (relative_path, patterns) in SMOKE_COVERAGE.items():
        text = read_text(root, relative_path)
        for pattern in patterns:
            if pattern not in text:
                raise AssertionError(
                    f"missing {label} smoke coverage marker {pattern!r} "
                    f"in {relative_path}"
                )


def run_syntax_checks(root: Path) -> None:
    for label, executable, command in SYNTAX_COMMANDS:
        if not shutil.which(executable):
            raise AssertionError(f"{executable} is required for {label} syntax check")
        subprocess.run(command, cwd=root, check=True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--skip-syntax",
        action="store_true",
        help="skip clang/clang++ syntax-only compilation checks",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    root = Path(__file__).resolve().parents[2]
    checks = (
        ("public header manifest and removed API absence", check_public_headers),
        ("retained build target names", check_build_targets),
        ("smoke coverage markers", check_smoke_coverage),
        ("unsupported platform cleanup", check_platform_cleanup),
    )

    try:
        for label, check in checks:
            check(root)
            print(f"ok: {label}")
        if args.skip_syntax:
            print("skip: syntax-only compilation")
        else:
            run_syntax_checks(root)
            print("ok: syntax-only compilation")
    except (AssertionError, subprocess.CalledProcessError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
