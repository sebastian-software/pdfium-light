// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXGE_DIB_RUST_RUST_BLEND_ADAPTER_H_
#define CORE_FXGE_DIB_RUST_RUST_BLEND_ADAPTER_H_

#include <stdint.h>

#include <array>
#include <optional>

#include "core/fxcrt/data_vector.h"
#include "core/fxcrt/span.h"

namespace fxge {

enum class BlendMode;
enum class FXDIB_Format : uint16_t;

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
  static bool CompositeBgraToBgrRow(BlendMode mode,
                                    pdfium::span<const uint8_t> source,
                                    pdfium::span<const uint8_t> clip,
                                    int output_components,
                                    bool rgb_byte_order,
                                    pdfium::span<uint8_t> output);
  static bool CompositeBgraToByteRow(BlendMode mode,
                                     pdfium::span<const uint8_t> source,
                                     pdfium::span<const uint8_t> clip,
                                     bool is_mask,
                                     pdfium::span<uint8_t> output);
  static bool CompositeOpaqueRow(BlendMode mode,
                                 FXDIB_Format destination_format,
                                 pdfium::span<const uint8_t> source,
                                 int source_components,
                                 pdfium::span<const uint8_t> clip,
                                 bool rgb_byte_order,
                                 pdfium::span<uint8_t> output);
  static bool CompositeMaskRow(BlendMode mode,
                               FXDIB_Format destination_format,
                               pdfium::span<const uint8_t> source,
                               int source_left,
                               bool source_is_bit_mask,
                               pdfium::span<const uint8_t> clip,
                               uint32_t mask_argb,
                               bool rgb_byte_order,
                               pdfium::span<uint8_t> output);
  static bool CompositePaletteRow(
      BlendMode mode,
      FXDIB_Format destination_format,
      pdfium::span<const uint8_t> source,
      int source_left,
      bool source_is_bit,
      pdfium::span<const uint8_t> gray_palette,
      pdfium::span<const uint32_t> argb_palette,
      pdfium::span<const uint8_t> clip,
      bool rgb_byte_order,
      pdfium::span<uint8_t> output);
  static std::optional<std::array<uint8_t, 3>> ConvertCmykToRgb(
      uint8_t cyan,
      uint8_t magenta,
      uint8_t yellow,
      uint8_t key);
  static bool ConvertCmykToBgrRow(pdfium::span<const uint8_t> source,
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
