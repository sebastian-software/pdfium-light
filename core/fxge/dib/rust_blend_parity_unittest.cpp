// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <stddef.h>
#include <stdint.h>

#include <array>

#include "core/fxge/dib/blend.h"
#include "core/fxge/dib/fx_dib.h"
#include "core/fxge/dib/rust/rust_blend_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace fxge {
namespace {

constexpr size_t kChannelPairCount = 256 * 256;

TEST(RustBlendParityTest, EverySeparableModeAndChannelPairMatchesCpp) {
  std::array<uint8_t, kChannelPairCount> backdrop = {};
  std::array<uint8_t, kChannelPairCount> source = {};
  for (size_t index = 0; index < kChannelPairCount; ++index) {
    backdrop[index] = static_cast<uint8_t>(index / 256);
    source[index] = static_cast<uint8_t>(index % 256);
  }

  for (int mode_value = static_cast<int>(BlendMode::kNormal);
       mode_value <= static_cast<int>(BlendMode::kExclusion); ++mode_value) {
    const auto mode = static_cast<BlendMode>(mode_value);
    auto candidate = RustBlendAdapter::BlendChannels(mode, backdrop, source);
    ASSERT_TRUE(candidate.has_value());
    ASSERT_EQ(kChannelPairCount, candidate->size());
    for (size_t index = 0; index < kChannelPairCount; ++index) {
      ASSERT_EQ(Blend(mode, backdrop[index], source[index]),
                candidate.value()[index])
          << "mode=" << mode_value << " backdrop=" << int{backdrop[index]}
          << " source=" << int{source[index]};
    }
  }
}

TEST(RustBlendParityTest, RejectsMismatchedInputLengths) {
  const std::array<uint8_t, 1> backdrop = {0};
  const std::array<uint8_t, 2> source = {0, 1};
  EXPECT_FALSE(RustBlendAdapter::BlendChannels(BlendMode::kNormal, backdrop,
                                               source)
                   .has_value());
}

TEST(RustBlendParityTest, RejectsNonSeparableModes) {
  const std::array<uint8_t, 1> channel = {0};
  EXPECT_FALSE(RustBlendAdapter::BlendChannels(BlendMode::kHue, channel, channel)
                   .has_value());
}

}  // namespace
}  // namespace fxge
