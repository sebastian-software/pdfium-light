// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxge/dib/rust/rust_blend_adapter.h"

#include <stddef.h>
#include <stdint.h>

#include "core/fxge/dib/fx_dib.h"

extern "C" bool pdfium_rust_blend_channels(uint8_t mode,
                                            const uint8_t* backdrop,
                                            const uint8_t* source,
                                            uint8_t* output,
                                            size_t len);
extern "C" bool pdfium_rust_composite_bgra_row(uint8_t mode,
                                                const uint8_t* source,
                                                const uint8_t* clip,
                                                uint8_t* output,
                                                size_t pixel_count);

namespace {

thread_local bool g_use_rust_dib_candidate = true;

}  // namespace

namespace fxge {

// static
std::optional<DataVector<uint8_t>> RustBlendAdapter::BlendChannels(
    BlendMode mode,
    pdfium::span<const uint8_t> backdrop,
    pdfium::span<const uint8_t> source) {
  if (backdrop.size() != source.size() || mode > BlendMode::kExclusion) {
    return std::nullopt;
  }

  DataVector<uint8_t> output(backdrop.size());
  if (!pdfium_rust_blend_channels(static_cast<uint8_t>(mode), backdrop.data(),
                                  source.data(), output.data(), output.size())) {
    return std::nullopt;
  }
  return output;
}

// static
bool RustBlendAdapter::CompositeBgraRow(BlendMode mode,
                                        pdfium::span<const uint8_t> source,
                                        pdfium::span<const uint8_t> clip,
                                        pdfium::span<uint8_t> output) {
  constexpr size_t kBytesPerPixel = 4;
  if (source.size() != output.size() || source.size() % kBytesPerPixel != 0 ||
      (!clip.empty() && clip.size() != source.size() / kBytesPerPixel) ||
      mode > BlendMode::kExclusion) {
    return false;
  }
  return pdfium_rust_composite_bgra_row(
      static_cast<uint8_t>(mode), source.data(),
      clip.empty() ? nullptr : clip.data(), output.data(),
      source.size() / kBytesPerPixel);
}

// static
bool RustBlendAdapter::UseCandidate() {
  return g_use_rust_dib_candidate;
}

ScopedRustDibImplementationForTesting::ScopedRustDibImplementationForTesting(
    bool use_candidate)
    : previous_(g_use_rust_dib_candidate) {
  g_use_rust_dib_candidate = use_candidate;
}

ScopedRustDibImplementationForTesting::~ScopedRustDibImplementationForTesting() {
  g_use_rust_dib_candidate = previous_;
}

}  // namespace fxge
