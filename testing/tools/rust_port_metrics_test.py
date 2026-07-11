#!/usr/bin/env python3
# Copyright 2026 Sebastian Werner
# Use of this source code is governed by a BSD-style license that can be
# found in the LICENSE file.

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import rust_port_metrics


class RustPortMetricsTest(unittest.TestCase):
    def test_count_abi_thunk_lines_includes_region_markers(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "lib.rs"
            source.write_text(
                "behavior\n"
                f"{rust_port_metrics.ABI_BEGIN}\n"
                "extern fn thunk() {}\n"
                f"{rust_port_metrics.ABI_END}\n",
                encoding="utf-8",
            )

            self.assertEqual(
                3, rust_port_metrics.count_abi_thunk_lines((source,))
            )

    def test_count_abi_thunk_lines_rejects_unterminated_region(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "lib.rs"
            source.write_text(
                f"{rust_port_metrics.ABI_BEGIN}\nextern fn thunk() {{}}\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(ValueError, "unterminated ABI"):
                rust_port_metrics.count_abi_thunk_lines((source,))

    def test_generated_marker_must_be_near_file_start(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            generated = Path(directory) / "generated.rs"
            generated.write_text("// @generated\nconst DATA: u8 = 1;\n", encoding="utf-8")
            authored = Path(directory) / "authored.rs"
            authored.write_text(
                "\n".join(["// authored"] * 20 + ["// @generated"]),
                encoding="utf-8",
            )

            self.assertTrue(rust_port_metrics.is_generated_rust(generated))
            self.assertFalse(rust_port_metrics.is_generated_rust(authored))


if __name__ == "__main__":
    unittest.main()
