// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXCODEC_RUST_RUST_CODEC_ADAPTER_H_
#define CORE_FXCODEC_RUST_RUST_CODEC_ADAPTER_H_

#include <stdint.h>

#include "core/fxcodec/data_and_bytes_consumed.h"
#include "core/fxcrt/data_vector.h"
#include "core/fxcrt/span.h"

namespace fxcodec {

// Internal boundary for byte-oriented codecs. Rust owns results returned over
// the C ABI until this adapter copies and releases them.
class RustCodecAdapter final {
 public:
  static DataVector<uint8_t> A85Encode(pdfium::span<const uint8_t> src_span);
  static DataVector<uint8_t> RunLengthEncode(
      pdfium::span<const uint8_t> src_span);
  static DataAndBytesConsumed A85Decode(
      pdfium::span<const uint8_t> src_span);
  static DataAndBytesConsumed HexDecode(
      pdfium::span<const uint8_t> src_span);
  static DataAndBytesConsumed LZWDecode(pdfium::span<const uint8_t> src_span,
                                        bool early_change);
  static DataAndBytesConsumed RunLengthDecode(
      pdfium::span<const uint8_t> src_span);

  RustCodecAdapter() = delete;
  RustCodecAdapter(const RustCodecAdapter&) = delete;
  RustCodecAdapter& operator=(const RustCodecAdapter&) = delete;
};

}  // namespace fxcodec

#endif  // CORE_FXCODEC_RUST_RUST_CODEC_ADAPTER_H_
