// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Versioned parser differential corpus. Keep normal, malformed, truncated,
// defaulted, and unknown cross-reference behavior represented as Rust takes
// ownership of the parser and PDF object graph.
//
// Arguments: stable case name, complete in-memory PDF bytes.
RUST_PARSER_CASE(
    ValidMixedEntries,
    "%PDF1-7\n%\xa0\xf2\xa4\xf4\n"
    "7 0 obj <<\n"
    "  /Filter /ASCIIHexDecode\n"
    "  /Root 1 0 R\n"
    "  /Size 3\n"
    "  /W [1 1 1]\n"
    ">>\n"
    "stream\n"
    "00 00 05\n"
    "01 0F 00\n"
    "02 01 02\n"
    "endstream\n"
    "endobj\n"
    "startxref\n"
    "14\n"
    "%%EOF\n")
RUST_PARSER_CASE(
    TruncatedEntry,
    "%PDF1-7\n%\xa0\xf2\xa4\xf4\n"
    "7 0 obj <<\n"
    "  /Filter /ASCIIHexDecode\n"
    "  /Root 1 0 R\n"
    "  /Size 3\n"
    "  /W [1 1 1]\n"
    ">>\n"
    "stream\n"
    "00 00 05\n"
    "01 0F 00\n"
    "02 01\n"
    "endstream\n"
    "endobj\n"
    "startxref\n"
    "14\n"
    "%%EOF\n")
RUST_PARSER_CASE(
    DefaultTypeWidth,
    "%PDF1-7\n%\xa0\xf2\xa4\xf4\n"
    "7 0 obj <<\n"
    "  /Filter /ASCIIHexDecode\n"
    "  /Root 1 0 R\n"
    "  /Size 2\n"
    "  /W [0 1 1]\n"
    ">>\n"
    "stream\n"
    "0F 00\n"
    "12 00\n"
    "endstream\n"
    "endobj\n"
    "startxref\n"
    "14\n"
    "%%EOF\n")
RUST_PARSER_CASE(
    UnknownEntryType,
    "%PDF1-7\n%\xa0\xf2\xa4\xf4\n"
    "7 0 obj <<\n"
    "  /Filter /ASCIIHexDecode\n"
    "  /Root 1 0 R\n"
    "  /Size 3\n"
    "  /W [1 1 1]\n"
    ">>\n"
    "stream\n"
    "03 00 00\n"
    "01 0F 00\n"
    "02 01 02\n"
    "endstream\n"
    "endobj\n"
    "startxref\n"
    "14\n"
    "%%EOF\n")
