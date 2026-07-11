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

}  // namespace fxge
