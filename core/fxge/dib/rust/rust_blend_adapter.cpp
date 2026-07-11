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
extern "C" bool pdfium_rust_composite_bgra_to_bgr_row(
    uint8_t mode,
    const uint8_t* source,
    const uint8_t* clip,
    uint8_t* output,
    size_t output_components,
    bool rgb_byte_order,
    size_t pixel_count);
extern "C" bool pdfium_rust_composite_bgra_to_byte_row(uint8_t mode,
                                                        const uint8_t* source,
                                                        const uint8_t* clip,
                                                        uint8_t* output,
                                                        bool is_mask,
                                                        size_t pixel_count);
extern "C" bool pdfium_rust_composite_opaque_row(
    uint8_t mode,
    const uint8_t* source,
    size_t source_components,
    const uint8_t* clip,
    uint8_t* output,
    size_t output_components,
    uint8_t target,
    bool rgb_byte_order,
    size_t pixel_count);
extern "C" bool pdfium_rust_composite_mask_row(
    uint8_t mode,
    const uint8_t* source,
    size_t source_left,
    bool source_is_bit_mask,
    const uint8_t* clip,
    uint8_t mask_blue,
    uint8_t mask_green,
    uint8_t mask_red,
    uint8_t mask_alpha,
    uint8_t* output,
    size_t output_components,
    uint8_t target,
    bool rgb_byte_order,
    size_t pixel_count);
extern "C" bool pdfium_rust_composite_palette_row(
    uint8_t mode,
    const uint8_t* source,
    size_t source_left,
    bool source_is_bit,
    const uint8_t* gray_palette,
    size_t gray_palette_len,
    const uint32_t* argb_palette,
    size_t argb_palette_len,
    const uint8_t* clip,
    uint8_t* output,
    size_t output_components,
    uint8_t target,
    bool rgb_byte_order,
    size_t pixel_count);
extern "C" bool pdfium_rust_adobe_cmyk_to_rgb(uint8_t cyan,
                                                uint8_t magenta,
                                                uint8_t yellow,
                                                uint8_t key,
                                                uint8_t* red,
                                                uint8_t* green,
                                                uint8_t* blue);
extern "C" bool pdfium_rust_adobe_cmyk_to_bgr_row(const uint8_t* source,
                                                    uint8_t* output,
                                                    size_t pixel_count);
extern "C" bool pdfium_rust_bgra_set_red_from_alpha(uint8_t* buffer,
                                                     size_t width,
                                                     size_t height,
                                                     size_t pitch);
extern "C" bool pdfium_rust_bgra_set_opaque_alpha(uint8_t* buffer,
                                                   size_t width,
                                                   size_t height,
                                                   size_t pitch);
extern "C" bool pdfium_rust_bgra_multiply_alpha_mask(
    uint8_t* buffer,
    size_t buffer_pitch,
    const uint8_t* mask,
    size_t mask_pitch,
    size_t width,
    size_t height);
extern "C" bool pdfium_rust_bgra_multiply_alpha(uint8_t* buffer,
                                                 size_t width,
                                                 size_t height,
                                                 size_t pitch,
                                                 uint8_t alpha);

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
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_bgra_row(
      static_cast<uint8_t>(mode), source.data(),
      clip.empty() ? nullptr : clip.data(), output.data(),
      source.size() / kBytesPerPixel);
}

// static
bool RustBlendAdapter::CompositeBgraToBgrRow(
    BlendMode mode,
    pdfium::span<const uint8_t> source,
    pdfium::span<const uint8_t> clip,
    int output_components,
    bool rgb_byte_order,
    pdfium::span<uint8_t> output) {
  constexpr size_t kSourceBytesPerPixel = 4;
  if (source.size() % kSourceBytesPerPixel != 0 ||
      (output_components != 3 && output_components != 4) ||
      output.size() != source.size() / kSourceBytesPerPixel * output_components ||
      (!clip.empty() && clip.size() != source.size() / kSourceBytesPerPixel) ||
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_bgra_to_bgr_row(
      static_cast<uint8_t>(mode), source.data(),
      clip.empty() ? nullptr : clip.data(), output.data(), output_components,
      rgb_byte_order, source.size() / kSourceBytesPerPixel);
}

// static
bool RustBlendAdapter::CompositeBgraToByteRow(
    BlendMode mode,
    pdfium::span<const uint8_t> source,
    pdfium::span<const uint8_t> clip,
    bool is_mask,
    pdfium::span<uint8_t> output) {
  constexpr size_t kSourceBytesPerPixel = 4;
  if (source.size() % kSourceBytesPerPixel != 0 ||
      output.size() != source.size() / kSourceBytesPerPixel ||
      (!clip.empty() && clip.size() != output.size()) ||
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_bgra_to_byte_row(
      static_cast<uint8_t>(mode), source.data(),
      clip.empty() ? nullptr : clip.data(), output.data(), is_mask,
      output.size());
}

// static
bool RustBlendAdapter::CompositeOpaqueRow(
    BlendMode mode,
    FXDIB_Format destination_format,
    pdfium::span<const uint8_t> source,
    int source_components,
    pdfium::span<const uint8_t> clip,
    bool rgb_byte_order,
    pdfium::span<uint8_t> output) {
  uint8_t target;
  int output_components;
  switch (destination_format) {
    case FXDIB_Format::k8bppRgb:
      target = 0;
      output_components = 1;
      break;
    case FXDIB_Format::k8bppMask:
      target = 1;
      output_components = 1;
      break;
    case FXDIB_Format::kBgr:
      target = 2;
      output_components = 3;
      break;
    case FXDIB_Format::kBgrx:
      target = 2;
      output_components = 4;
      break;
    case FXDIB_Format::kBgra:
      target = 3;
      output_components = 4;
      break;
    default:
      return false;
  }
  if ((source_components != 3 && source_components != 4) ||
      source.size() % source_components != 0 ||
      output.size() != source.size() / source_components * output_components ||
      (!clip.empty() && clip.size() != source.size() / source_components) ||
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_opaque_row(
      static_cast<uint8_t>(mode), source.data(), source_components,
      clip.empty() ? nullptr : clip.data(), output.data(), output_components,
      target, rgb_byte_order, source.size() / source_components);
}

// static
bool RustBlendAdapter::CompositeMaskRow(
    BlendMode mode,
    FXDIB_Format destination_format,
    pdfium::span<const uint8_t> source,
    int source_left,
    bool source_is_bit_mask,
    pdfium::span<const uint8_t> clip,
    uint32_t mask_argb,
    bool rgb_byte_order,
    pdfium::span<uint8_t> output) {
  uint8_t target;
  int output_components;
  switch (destination_format) {
    case FXDIB_Format::k8bppRgb:
      target = 0;
      output_components = 1;
      break;
    case FXDIB_Format::k8bppMask:
      target = 1;
      output_components = 1;
      break;
    case FXDIB_Format::kBgr:
      target = 2;
      output_components = 3;
      break;
    case FXDIB_Format::kBgrx:
      target = 2;
      output_components = 4;
      break;
    case FXDIB_Format::kBgra:
      target = 3;
      output_components = 4;
      break;
    default:
      return false;
  }
  const size_t pixel_count = output.size() / output_components;
  if (source_left < 0) {
    return false;
  }
  const size_t required_source_size = source_is_bit_mask
                                          ? (static_cast<size_t>(source_left) +
                                             pixel_count + 7) /
                                                8
                                          : pixel_count;
  if (source.size() < required_source_size ||
      output.size() % output_components != 0 ||
      (!clip.empty() && clip.size() != pixel_count) ||
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_mask_row(
      static_cast<uint8_t>(mode), source.data(), source_left,
      source_is_bit_mask, clip.empty() ? nullptr : clip.data(),
      FXARGB_B(mask_argb), FXARGB_G(mask_argb), FXARGB_R(mask_argb),
      FXARGB_A(mask_argb), output.data(), output_components, target,
      rgb_byte_order, pixel_count);
}

// static
bool RustBlendAdapter::CompositePaletteRow(
    BlendMode mode,
    FXDIB_Format destination_format,
    pdfium::span<const uint8_t> source,
    int source_left,
    bool source_is_bit,
    pdfium::span<const uint8_t> gray_palette,
    pdfium::span<const uint32_t> argb_palette,
    pdfium::span<const uint8_t> clip,
    bool rgb_byte_order,
    pdfium::span<uint8_t> output) {
  uint8_t target;
  int output_components;
  switch (destination_format) {
    case FXDIB_Format::k8bppRgb:
      target = 0;
      output_components = 1;
      break;
    case FXDIB_Format::k8bppMask:
      target = 1;
      output_components = 1;
      break;
    case FXDIB_Format::kBgr:
      target = 2;
      output_components = 3;
      break;
    case FXDIB_Format::kBgrx:
      target = 2;
      output_components = 4;
      break;
    case FXDIB_Format::kBgra:
      target = 3;
      output_components = 4;
      break;
    default:
      return false;
  }
  if (source_left < 0 || output.size() % output_components != 0) {
    return false;
  }
  const size_t pixel_count = output.size() / output_components;
  const size_t required_source_size = source_is_bit
                                          ? (static_cast<size_t>(source_left) +
                                             pixel_count + 7) /
                                                8
                                          : pixel_count;
  if (source.size() < required_source_size ||
      (!clip.empty() && clip.size() != pixel_count) ||
      mode > BlendMode::kLast) {
    return false;
  }
  return pdfium_rust_composite_palette_row(
      static_cast<uint8_t>(mode), source.data(), source_left, source_is_bit,
      gray_palette.data(), gray_palette.size(), argb_palette.data(),
      argb_palette.size(), clip.empty() ? nullptr : clip.data(), output.data(),
      output_components, target, rgb_byte_order, pixel_count);
}

// static
std::optional<std::array<uint8_t, 3>> RustBlendAdapter::ConvertCmykToRgb(
    uint8_t cyan,
    uint8_t magenta,
    uint8_t yellow,
    uint8_t key) {
  std::array<uint8_t, 3> output;
  if (!pdfium_rust_adobe_cmyk_to_rgb(cyan, magenta, yellow, key, &output[0],
                                     &output[1], &output[2])) {
    return std::nullopt;
  }
  return output;
}

// static
bool RustBlendAdapter::ConvertCmykToBgrRow(
    pdfium::span<const uint8_t> source,
    pdfium::span<uint8_t> output) {
  constexpr size_t kSourceBytesPerPixel = 4;
  constexpr size_t kOutputBytesPerPixel = 3;
  if (source.size() % kSourceBytesPerPixel != 0 ||
      output.size() !=
          source.size() / kSourceBytesPerPixel * kOutputBytesPerPixel) {
    return false;
  }
  return pdfium_rust_adobe_cmyk_to_bgr_row(
      source.data(), output.data(), source.size() / kSourceBytesPerPixel);
}

namespace {

bool IsValidBitmapBuffer(size_t buffer_size,
                         int width,
                         int height,
                         uint32_t pitch,
                         size_t components) {
  return width > 0 && height > 0 && pitch > 0 &&
         static_cast<size_t>(width) <= pitch / components &&
         static_cast<size_t>(height) <= buffer_size / pitch;
}

}  // namespace

// static
bool RustBlendAdapter::SetBgraRedFromAlpha(pdfium::span<uint8_t> buffer,
                                           int width,
                                           int height,
                                           uint32_t pitch) {
  return IsValidBitmapBuffer(buffer.size(), width, height, pitch, 4) &&
         pdfium_rust_bgra_set_red_from_alpha(buffer.data(), width, height,
                                             pitch);
}

// static
bool RustBlendAdapter::SetBgraOpaqueAlpha(pdfium::span<uint8_t> buffer,
                                          int width,
                                          int height,
                                          uint32_t pitch) {
  return IsValidBitmapBuffer(buffer.size(), width, height, pitch, 4) &&
         pdfium_rust_bgra_set_opaque_alpha(buffer.data(), width, height, pitch);
}

// static
bool RustBlendAdapter::MultiplyBgraAlphaMask(
    pdfium::span<uint8_t> buffer,
    uint32_t buffer_pitch,
    pdfium::span<const uint8_t> mask,
    uint32_t mask_pitch,
    int width,
    int height) {
  return IsValidBitmapBuffer(buffer.size(), width, height, buffer_pitch, 4) &&
         IsValidBitmapBuffer(mask.size(), width, height, mask_pitch, 1) &&
         pdfium_rust_bgra_multiply_alpha_mask(
             buffer.data(), buffer_pitch, mask.data(), mask_pitch, width,
             height);
}

// static
bool RustBlendAdapter::MultiplyBgraAlpha(pdfium::span<uint8_t> buffer,
                                         int width,
                                         int height,
                                         uint32_t pitch,
                                         uint8_t alpha) {
  return IsValidBitmapBuffer(buffer.size(), width, height, pitch, 4) &&
         pdfium_rust_bgra_multiply_alpha(buffer.data(), width, height, pitch,
                                         alpha);
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
