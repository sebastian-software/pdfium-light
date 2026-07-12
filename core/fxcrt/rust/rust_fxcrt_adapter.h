// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FXCRT_RUST_RUST_FXCRT_ADAPTER_H_
#define CORE_FXCRT_RUST_RUST_FXCRT_ADAPTER_H_

#include <stdint.h>

#include <optional>

#include "core/fxcrt/span.h"

namespace pdfium::rust {

class RustByteStringPoolIndex final {
 public:
  RustByteStringPoolIndex();
  RustByteStringPoolIndex(const RustByteStringPoolIndex&) = delete;
  RustByteStringPoolIndex& operator=(const RustByteStringPoolIndex&) = delete;
  ~RustByteStringPoolIndex();

  std::optional<uintptr_t> Get(pdfium::span<const uint8_t> value) const;
  bool Insert(pdfium::span<const uint8_t> value, uintptr_t handle);

 private:
  void* state_;
};

}  // namespace pdfium::rust

#endif  // CORE_FXCRT_RUST_RUST_FXCRT_ADAPTER_H_
