// Copyright 2024 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/fx_dib.h"

#include <stdint.h>

#include "testing/gtest/include/gtest/gtest.h"

TEST(FxDibTest, ArgbToBGRAStruct) {
  FX_BGRA_STRUCT<uint8_t> white = ArgbToBGRAStruct(0xffffffff);
  EXPECT_EQ(0xff, white.blue);
  EXPECT_EQ(0xff, white.green);
  EXPECT_EQ(0xff, white.red);
  EXPECT_EQ(0xff, white.alpha);

  FX_BGRA_STRUCT<uint8_t> black = ArgbToBGRAStruct(0xff000000);
  EXPECT_EQ(0, black.blue);
  EXPECT_EQ(0, black.green);
  EXPECT_EQ(0, black.red);
  EXPECT_EQ(0xff, black.alpha);

  FX_BGRA_STRUCT<uint8_t> abeebead = ArgbToBGRAStruct(0xabeebead);
  EXPECT_EQ(0xad, abeebead.blue);
  EXPECT_EQ(0xbe, abeebead.green);
  EXPECT_EQ(0xee, abeebead.red);
  EXPECT_EQ(0xab, abeebead.alpha);
}

TEST(FxDibTest, AlphaMerge) {
  EXPECT_EQ(0, AlphaMerge(0, 0, 0));
  EXPECT_EQ(0, AlphaMerge(0, 0, 127));
  EXPECT_EQ(0, AlphaMerge(0, 0, 255));
  EXPECT_EQ(0, AlphaMerge(0, 127, 0));
  EXPECT_EQ(63, AlphaMerge(0, 127, 127));
  EXPECT_EQ(127, AlphaMerge(0, 127, 255));
  EXPECT_EQ(0, AlphaMerge(0, 255, 0));
  EXPECT_EQ(127, AlphaMerge(0, 255, 127));
  EXPECT_EQ(255, AlphaMerge(0, 255, 255));
  EXPECT_EQ(127, AlphaMerge(127, 0, 0));
  EXPECT_EQ(63, AlphaMerge(127, 0, 127));
  EXPECT_EQ(0, AlphaMerge(127, 0, 255));
  EXPECT_EQ(127, AlphaMerge(127, 127, 0));
  EXPECT_EQ(127, AlphaMerge(127, 127, 127));
  EXPECT_EQ(127, AlphaMerge(127, 127, 255));
  EXPECT_EQ(127, AlphaMerge(127, 255, 0));
  EXPECT_EQ(190, AlphaMerge(127, 255, 127));
  EXPECT_EQ(255, AlphaMerge(127, 255, 255));
  EXPECT_EQ(255, AlphaMerge(255, 0, 0));
  EXPECT_EQ(128, AlphaMerge(255, 0, 127));
  EXPECT_EQ(0, AlphaMerge(255, 0, 255));
  EXPECT_EQ(255, AlphaMerge(255, 127, 0));
  EXPECT_EQ(191, AlphaMerge(255, 127, 127));
  EXPECT_EQ(127, AlphaMerge(255, 127, 255));
  EXPECT_EQ(255, AlphaMerge(255, 255, 0));
  EXPECT_EQ(255, AlphaMerge(255, 255, 127));
  EXPECT_EQ(255, AlphaMerge(255, 255, 255));
}
