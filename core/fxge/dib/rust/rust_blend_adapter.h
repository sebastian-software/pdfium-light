// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_DIB_RUST_RUST_BLEND_ADAPTER_H_
#define CORE_FXGE_DIB_RUST_RUST_BLEND_ADAPTER_H_

#include <stdint.h>

#include <optional>

#include "core/fxcrt/data_vector.h"
#include "core/fxcrt/span.h"

namespace fxge {

enum class BlendMode;

// Internal batch boundary for Rust-owned separable blend primitives.
class RustBlendAdapter final {
 public:
  static std::optional<DataVector<uint8_t>> BlendChannels(
      BlendMode mode,
      pdfium::span<const uint8_t> backdrop,
      pdfium::span<const uint8_t> source);
  static bool CompositeBgraRow(BlendMode mode,
                               pdfium::span<const uint8_t> source,
                               pdfium::span<const uint8_t> clip,
                               pdfium::span<uint8_t> output);
  static bool UseCandidate();

  RustBlendAdapter() = delete;
};

class ScopedRustDibImplementationForTesting final {
 public:
  explicit ScopedRustDibImplementationForTesting(bool use_candidate);
  ScopedRustDibImplementationForTesting(
      const ScopedRustDibImplementationForTesting&) = delete;
  ScopedRustDibImplementationForTesting& operator=(
      const ScopedRustDibImplementationForTesting&) = delete;
  ~ScopedRustDibImplementationForTesting();

 private:
  bool previous_;
};

}  // namespace fxge

#endif  // CORE_FXGE_DIB_RUST_RUST_BLEND_ADAPTER_H_
