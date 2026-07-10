// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "testing/embedder_test_environment.h"

#include <ostream>

#include "core/fxcrt/check.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/fx_system.h"
#include "public/fpdfview.h"

#ifdef PDF_ENABLE_V8
#include "testing/v8_test_environment.h"
#endif  // PDF_ENABLE_V8

namespace {

EmbedderTestEnvironment* g_environment = nullptr;

}  // namespace

EmbedderTestEnvironment::EmbedderTestEnvironment() {
  DCHECK(!g_environment);
  g_environment = this;
}

EmbedderTestEnvironment::~EmbedderTestEnvironment() {
  DCHECK(g_environment);
  g_environment = nullptr;
}

// static
EmbedderTestEnvironment* EmbedderTestEnvironment::GetInstance() {
  return g_environment;
}

void EmbedderTestEnvironment::SetUp() {
  FPDF_LIBRARY_CONFIG config = {
      .version = version_,
      .m_pUserFontPaths = test_fonts_.font_paths(),
      .m_BrotliEnabled = brotli_enabled_,
  };

  FPDF_InitLibraryWithConfig(&config);

  test_fonts_.InstallFontMapper();
}

void EmbedderTestEnvironment::TearDown() {
  FPDF_DestroyLibrary();
}

void EmbedderTestEnvironment::AddFlags(int argc, char** argv) {
  for (int i = 1; i < argc; ++i) {
    AddFlag(UNSAFE_TODO(argv[i]));
  }
  CHECK(CheckFlags());
}

void EmbedderTestEnvironment::AddFlag(const std::string& flag) {
  if (flag == "--write-pngs") {
    write_pngs_ = true;
    return;
  }


  std::cerr << "Unknown flag: " << flag << "\n";
}

bool EmbedderTestEnvironment::CheckFlags() { return true; }
