// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/cfx_dibbase.h"

#include <limits>
#include <optional>

#include "core/fxcrt/fx_coordinates.h"
#include "core/fxge/dib/cfx_dibitmap.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

struct Input {
  CFX_Point src_top_left;
  CFX_Size src_size;
  CFX_Point dest_top_left;
  CFX_Size overlap_size;
};

struct Output {
  CFX_Point src_top_left;
  CFX_Point dest_top_left;
  CFX_Size overlap_size;
};

void RunOverlapRectTest(const CFX_DIBitmap* bitmap,
                        const Input& input,
                        const Output* expected_output) {
  // Initialize in-out parameters.
  int src_left = input.src_top_left.x;
  int src_top = input.src_top_left.y;
  int dest_left = input.dest_top_left.x;
  int dest_top = input.dest_top_left.y;
  int overlap_width = input.overlap_size.width;
  int overlap_height = input.overlap_size.height;

  bool success = bitmap->GetOverlapRect(
      dest_left, dest_top, overlap_width, overlap_height, input.src_size.width,
      input.src_size.height, src_left, src_top,
      /*pClipRgn=*/nullptr);
  if (success == !expected_output) {
    ADD_FAILURE();
    return;
  }

  if (expected_output) {
    EXPECT_EQ(expected_output->src_top_left.x, src_left);
    EXPECT_EQ(expected_output->src_top_left.y, src_top);
    EXPECT_EQ(expected_output->dest_top_left.x, dest_left);
    EXPECT_EQ(expected_output->dest_top_left.y, dest_top);
    EXPECT_EQ(expected_output->overlap_size.width, overlap_width);
    EXPECT_EQ(expected_output->overlap_size.height, overlap_height);
  }
}

}  // namespace

TEST(CFXDIBBaseTest, GetOverlapRectTrivialOverlap) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{0, 0},
                        /*overlap_size=*/{400, 300}};
  const Output kExpectedOutput = {/*src_top_left=*/{0, 0},
                                  /*dest_top_left=*/{0, 0},
                                  /*overlap_size=*/{400, 300}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapNoLimit) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{35, 41}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{123, 137},
                        /*overlap_size=*/{200, 100}};
  const Output kExpectedOutput = {/*src_top_left=*/{35, 41},
                                  /*dest_top_left=*/{123, 137},
                                  /*overlap_size=*/{200, 100}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapLimitedBySource) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{141, 154}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{35, 41},
                        /*overlap_size=*/{270, 160}};
  const Output kExpectedOutput = {/*src_top_left=*/{141, 154},
                                  /*dest_top_left=*/{35, 41},
                                  /*overlap_size=*/{259, 146}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectOverlapLimitedByDestination) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kInput = {/*src_top_left=*/{35, 41}, /*src_size=*/{400, 300},
                        /*dest_top_left=*/{123, 137},
                        /*overlap_size=*/{280, 170}};
  const Output kExpectedOutput = {/*src_top_left=*/{35, 41},
                                  /*dest_top_left=*/{123, 137},
                                  /*overlap_size=*/{277, 163}};
  RunOverlapRectTest(bitmap.Get(), kInput, &kExpectedOutput);
}

TEST(CFXDIBBaseTest, GetOverlapRectBadInputs) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  EXPECT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  const Input kEmptyInputs[] = {
      // Empty source rect.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{0, 0},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Empty overlap size.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{0, 0}},
      // Source out of bounds on x-axis.
      {/*src_top_left=*/{-400, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
  };
  for (const Input& input : kEmptyInputs) {
    RunOverlapRectTest(bitmap.Get(), input, /*expected_output=*/nullptr);
  }

  const Input kOutOfBoundInputs[] = {
      // Source out of bounds on x-axis.
      {/*src_top_left=*/{400, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Source out of bounds on y-axis.
      {/*src_top_left=*/{0, 300}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Source out of bounds on y-axis.
      {/*src_top_left=*/{0, -300}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on x-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{-400, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on x-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{400, 0},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on y-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, -300},
       /*overlap_size=*/{400, 300}},
      // Destination out of bounds on y-axis.
      {/*src_top_left=*/{0, 0}, /*src_size=*/{400, 300},
       /*dest_top_left=*/{0, 300},
       /*overlap_size=*/{400, 300}},
  };
  for (const Input& input : kOutOfBoundInputs) {
    RunOverlapRectTest(bitmap.Get(), input, /*expected_output=*/nullptr);
  }
}

TEST(CFXDIBBaseTest, RustOverlapRectMatchesCppReferenceCorpus) {
  auto bitmap = pdfium::MakeRetain<CFX_DIBitmap>();
  ASSERT_TRUE(bitmap->Create(400, 300, FXDIB_Format::k1bppRgb));

  for (int index = 0; index < 1026; ++index) {
    int dest_left = (index * 97 % 901) - 450;
    int dest_top = (index * 53 % 701) - 350;
    int width = 1 + index * 37 % 500;
    int height = 1 + index * 29 % 400;
    const int src_width = 250 + index * 11 % 350;
    const int src_height = 200 + index * 17 % 300;
    int src_left = (index * 71 % 801) - 400;
    int src_top = (index * 43 % 601) - 300;
    if (index == 1024) {
      src_left = std::numeric_limits<int>::max() - 10;
      width = 100;
    } else if (index == 1025) {
      dest_left = std::numeric_limits<int>::min() + 5;
      src_left = std::numeric_limits<int>::max() - 5;
      width = 20;
    }
    const std::optional<FX_RECT> clip =
        index % 3 == 0
            ? std::optional<FX_RECT>(FX_RECT(-20 + index % 40,
                                             -10 + index % 30,
                                             200 + index % 250,
                                             150 + index % 200))
            : std::nullopt;

    int reference_dest_left = dest_left;
    int reference_dest_top = dest_top;
    int reference_width = width;
    int reference_height = height;
    int reference_src_left = src_left;
    int reference_src_top = src_top;
    bool reference_success;
    {
      fxge::ScopedRustDibImplementationForTesting implementation(false);
      reference_success = bitmap->GetOverlapRect(
          reference_dest_left, reference_dest_top, reference_width,
          reference_height, src_width, src_height, reference_src_left,
          reference_src_top, clip ? &clip.value() : nullptr);
    }
    const auto candidate = fxge::RustBlendAdapter::GetOverlapRect(
        bitmap->GetWidth(), bitmap->GetHeight(), dest_left, dest_top, width,
        height, src_width, src_height, src_left, src_top, clip.has_value(),
        clip ? clip->left : 0, clip ? clip->top : 0, clip ? clip->right : 0,
        clip ? clip->bottom : 0);
    ASSERT_EQ(reference_success, candidate.has_value()) << "index=" << index;
    if (reference_success) {
      EXPECT_EQ(reference_dest_left, (*candidate)[0]);
      EXPECT_EQ(reference_dest_top, (*candidate)[1]);
      EXPECT_EQ(reference_width, (*candidate)[2]);
      EXPECT_EQ(reference_height, (*candidate)[3]);
      EXPECT_EQ(reference_src_left, (*candidate)[4]);
      EXPECT_EQ(reference_src_top, (*candidate)[5]);
    }
  }
}
