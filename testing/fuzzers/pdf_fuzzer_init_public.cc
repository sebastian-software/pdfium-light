// Copyright 2019 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "testing/fuzzers/pdf_fuzzer_init_public.h"

#include <string.h>  // For memset()

#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/fx_memcpy_wrappers.h"
#include "testing/fuzzers/pdfium_fuzzer_util.h"

namespace {

// pdf_fuzzer_init.cc and pdf_fuzzer_init_public.cc are mutually exclusive
// and should not be built together. Static initializers and destructors
// avoid problems with fuzzer initialization and termination.
PDFFuzzerInitPublic g_instance;

}  // namespace

PDFFuzzerInitPublic::PDFFuzzerInitPublic() {
  UNSAFE_TODO(FXSYS_memset(&config_, '\0', sizeof(config_)));
  config_.version = 1;
  config_.m_pUserFontPaths = nullptr;
  FPDF_InitLibraryWithConfig(&config_);

  UNSAFE_TODO(FXSYS_memset(&unsupport_info_, '\0', sizeof(unsupport_info_)));
  unsupport_info_.version = 1;
  unsupport_info_.FSDK_UnSupport_Handler = [](UNSUPPORT_INFO*, int) {};
  FSDK_SetUnSpObjProcessHandler(&unsupport_info_);
}

PDFFuzzerInitPublic::~PDFFuzzerInitPublic() {
  FPDF_SetFuzzerPerProcessState(nullptr);
}
