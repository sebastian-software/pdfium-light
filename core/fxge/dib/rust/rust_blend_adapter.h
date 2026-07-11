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
  static bool SetBgraRedFromAlpha(pdfium::span<uint8_t> buffer,
                                  int width,
                                  int height,
                                  uint32_t pitch);
  static bool SetBgraOpaqueAlpha(pdfium::span<uint8_t> buffer,
                                 int width,
                                 int height,
                                 uint32_t pitch);
  static bool MultiplyBgraAlphaMask(pdfium::span<uint8_t> buffer,
                                    uint32_t buffer_pitch,
                                    pdfium::span<const uint8_t> mask,
                                    uint32_t mask_pitch,
                                    int width,
                                    int height);
  static bool MultiplyBgraAlpha(pdfium::span<uint8_t> buffer,
                                int width,
                                int height,
                                uint32_t pitch,
                                uint8_t alpha);
  static bool ClearBitmap(pdfium::span<uint8_t> buffer,
                          int width,
                          int height,
                          uint32_t pitch,
                          size_t components,
                          std::array<uint8_t, 4> pixel,
                          bool fill_padding);
  static bool ConvertBgrColorScale(pdfium::span<uint8_t> buffer,
                                   int width,
                                   int height,
                                   uint32_t pitch,
                                   size_t components);
  static std::optional<std::array<uint32_t, 2>> CalculatePitchAndSize(
      int width,
      int height,
      FXDIB_Format format,
      uint32_t requested_pitch);
  static bool Expand1bppMask(pdfium::span<const uint8_t> source,
                             uint32_t source_pitch,
                             pdfium::span<uint8_t> destination,
                             uint32_t destination_pitch,
                             int width,
                             int height);
  static bool PopulateBitmap(pdfium::span<const uint8_t> source,
                             uint32_t source_pitch,
                             pdfium::span<uint8_t> destination,
                             uint32_t destination_pitch,
                             int height);
  static bool TransferBitmapRow(pdfium::span<const uint8_t> source,
                                int source_left,
                                pdfium::span<uint8_t> destination,
                                int destination_left,
                                int width,
                                size_t components);
  static bool Transfer1bppRow(pdfium::span<const uint8_t> source,
                              int source_left,
                              pdfium::span<uint8_t> destination,
                              int destination_left,
                              int width);
  static bool Composite1bppMaskRow(pdfium::span<const uint8_t> source,
                                   int source_left,
                                   pdfium::span<uint8_t> destination,
                                   int destination_left,
                                   int width);
  static std::optional<std::array<int32_t, 6>> GetOverlapRect(
      int destination_width,
      int destination_height,
      int destination_left,
      int destination_top,
      int width,
      int height,
      int source_width,
      int source_height,
      int source_left,
      int source_top,
      bool has_clip,
      int clip_left,
      int clip_top,
      int clip_right,
      int clip_bottom);
  static bool BuildDefaultPalette(int bits_per_pixel,
                                  pdfium::span<uint32_t> output);
  static std::optional<uint32_t> GetDefaultPaletteArgb(int bits_per_pixel,
                                                       int index);
  static std::optional<int> FindPalette(int bits_per_pixel,
                                        pdfium::span<const uint32_t> palette,
                                        uint32_t color);
  static bool ConvertBufferRow(FXDIB_Format destination_format,
                               FXDIB_Format source_format,
                               pdfium::span<const uint8_t> source,
                               int source_left,
                               pdfium::span<const uint32_t> palette,
                               pdfium::span<uint8_t> output,
                               int width);
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
