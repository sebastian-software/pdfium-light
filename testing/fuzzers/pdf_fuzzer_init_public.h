// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef TESTING_FUZZERS_PDF_FUZZER_INIT_PUBLIC_H_
#define TESTING_FUZZERS_PDF_FUZZER_INIT_PUBLIC_H_

#include "public/fpdf_ext.h"
#include "public/fpdfview.h"

// Initializes the library once for all runs of the fuzzer.
class PDFFuzzerInitPublic {
 public:
  PDFFuzzerInitPublic();
  ~PDFFuzzerInitPublic();

 private:
  FPDF_LIBRARY_CONFIG config_;
  UNSUPPORT_INFO unsupport_info_;
};

#endif  // TESTING_FUZZERS_PDF_FUZZER_INIT_PUBLIC_H_
