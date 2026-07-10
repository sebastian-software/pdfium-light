// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxcodec/rust/rust_codec_adapter.h"

#include <stddef.h>

#include <utility>

namespace fxcodec {

namespace {

struct RustCodecResult {
  uint8_t* data;
  size_t len;
  size_t capacity;
  uint32_t bytes_consumed;
};

struct RustFaxScanlineResult {
  RustCodecResult data;
  uint32_t* offsets;
  size_t offsets_len;
  size_t offsets_capacity;
};

extern "C" RustCodecResult pdfium_rust_a85_encode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_run_length_encode(const uint8_t* data,
                                                           size_t len);
extern "C" RustCodecResult pdfium_rust_a85_decode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_hex_decode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_fax_g4_decode(const uint8_t* data,
                                                       size_t len,
                                                       uint32_t starting_bitpos,
                                                       int width,
                                                       int height,
                                                       int pitch);
extern "C" RustFaxScanlineResult pdfium_rust_fax_scanline_decode(
    const uint8_t* data,
    size_t len,
    int width,
    int height,
    int encoding,
    bool end_of_line,
    bool byte_align,
    bool black_is_1,
    int pitch);
extern "C" RustCodecResult pdfium_rust_lzw_decode(const uint8_t* data,
                                                    size_t len,
                                                    bool early_change);
extern "C" RustCodecResult pdfium_rust_png_predictor(
    const uint8_t* data,
    size_t len,
    int colors,
    int bits_per_component,
    int columns);
extern "C" RustCodecResult pdfium_rust_run_length_decode(const uint8_t* data,
                                                           size_t len);
extern "C" RustCodecResult pdfium_rust_tiff_predictor(
    const uint8_t* data,
    size_t len,
    int colors,
    int bits_per_component,
    int columns);
extern "C" void pdfium_rust_codec_result_free(uint8_t* data,
                                                size_t len,
                                                size_t capacity);
extern "C" void pdfium_rust_fax_scanline_result_free(
    RustFaxScanlineResult result);

DataVector<uint8_t> CopyAndFree(RustCodecResult result) {
  DataVector<uint8_t> output;
  if (result.data && result.len) {
    output.assign(result.data, result.data + result.len);
  }
  pdfium_rust_codec_result_free(result.data, result.len, result.capacity);
  return output;
}

DataAndBytesConsumed DecodeResult(RustCodecResult result) {
  const uint32_t bytes_consumed = result.bytes_consumed;
  return {CopyAndFree(result), bytes_consumed};
}

RustFaxScanlineData CopyFaxScanlineAndFree(RustFaxScanlineResult result) {
  RustFaxScanlineData output;
  if (result.data.data && result.data.len) {
    output.data.assign(result.data.data, result.data.data + result.data.len);
  }
  if (result.offsets && result.offsets_len) {
    output.offsets.assign(result.offsets, result.offsets + result.offsets_len);
  }
  pdfium_rust_fax_scanline_result_free(result);
  return output;
}

}  // namespace

// static
DataVector<uint8_t> RustCodecAdapter::A85Encode(
    pdfium::span<const uint8_t> src_span) {
  return CopyAndFree(pdfium_rust_a85_encode(src_span.data(), src_span.size()));
}

// static
DataVector<uint8_t> RustCodecAdapter::RunLengthEncode(
    pdfium::span<const uint8_t> src_span) {
  return CopyAndFree(
      pdfium_rust_run_length_encode(src_span.data(), src_span.size()));
}

// static
DataAndBytesConsumed RustCodecAdapter::A85Decode(
    pdfium::span<const uint8_t> src_span) {
  return DecodeResult(pdfium_rust_a85_decode(src_span.data(), src_span.size()));
}

// static
DataAndBytesConsumed RustCodecAdapter::HexDecode(
    pdfium::span<const uint8_t> src_span) {
  return DecodeResult(pdfium_rust_hex_decode(src_span.data(), src_span.size()));
}

// static
DataAndBytesConsumed RustCodecAdapter::FaxG4Decode(
    pdfium::span<const uint8_t> src_span,
    uint32_t starting_bitpos,
    int width,
    int height,
    int pitch) {
  return DecodeResult(pdfium_rust_fax_g4_decode(
      src_span.data(), src_span.size(), starting_bitpos, width, height,
      pitch));
}

// static
DataAndBytesConsumed RustCodecAdapter::LZWDecode(
    pdfium::span<const uint8_t> src_span,
    bool early_change) {
  return DecodeResult(
      pdfium_rust_lzw_decode(src_span.data(), src_span.size(), early_change));
}

// static
DataAndBytesConsumed RustCodecAdapter::PNGPredictor(
    pdfium::span<const uint8_t> src_span,
    int colors,
    int bits_per_component,
    int columns) {
  return DecodeResult(pdfium_rust_png_predictor(
      src_span.data(), src_span.size(), colors, bits_per_component, columns));
}

// static
DataAndBytesConsumed RustCodecAdapter::RunLengthDecode(
    pdfium::span<const uint8_t> src_span) {
  return DecodeResult(
      pdfium_rust_run_length_decode(src_span.data(), src_span.size()));
}

// static
DataAndBytesConsumed RustCodecAdapter::TIFFPredictor(
    pdfium::span<const uint8_t> src_span,
    int colors,
    int bits_per_component,
    int columns) {
  return DecodeResult(pdfium_rust_tiff_predictor(
      src_span.data(), src_span.size(), colors, bits_per_component, columns));
}

// static
RustFaxScanlineData RustCodecAdapter::FaxScanlineDecode(
    pdfium::span<const uint8_t> src_span,
    int width,
    int height,
    int encoding,
    bool end_of_line,
    bool byte_align,
    bool black_is_1,
    int pitch) {
  return CopyFaxScanlineAndFree(pdfium_rust_fax_scanline_decode(
      src_span.data(), src_span.size(), width, height, encoding, end_of_line,
      byte_align, black_is_1, pitch));
}

}  // namespace fxcodec
