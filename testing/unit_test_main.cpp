// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"
#include "testing/pdf_test_environment.h"

#if defined(PDF_USE_PARTITION_ALLOC)
#include "testing/allocator_shim_config.h"
#endif

// Can't use gtest-provided main since we need to initialize partition
// alloc before invoking any test, and add test environments.
int main(int argc, char** argv) {
#if defined(PDF_USE_PARTITION_ALLOC)
  pdfium::ConfigurePartitionAllocShimPartitionForTest();
#endif  // defined(PDF_USE_PARTITION_ALLOC)

  // PDF test environment will be deleted by gtest.
  AddGlobalTestEnvironment(new PDFTestEnvironment());

  testing::InitGoogleTest(&argc, argv);
  testing::InitGoogleMock(&argc, argv);

  return RUN_ALL_TESTS();
}
