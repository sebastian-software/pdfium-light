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

extern "C" RustCodecResult pdfium_rust_a85_encode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_run_length_encode(const uint8_t* data,
                                                           size_t len);
extern "C" RustCodecResult pdfium_rust_a85_decode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_hex_decode(const uint8_t* data,
                                                    size_t len);
extern "C" RustCodecResult pdfium_rust_run_length_decode(const uint8_t* data,
                                                           size_t len);
extern "C" void pdfium_rust_codec_result_free(uint8_t* data,
                                                size_t len,
                                                size_t capacity);

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
DataAndBytesConsumed RustCodecAdapter::RunLengthDecode(
    pdfium::span<const uint8_t> src_span) {
  return DecodeResult(
      pdfium_rust_run_length_decode(src_span.data(), src_span.size()));
}

}  // namespace fxcodec
