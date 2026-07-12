// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fxcrt/rust/rust_fxcrt_adapter.h"

#include "core/fxcrt/check.h"

extern "C" void* pdfium_rust_bytestring_pool_new();
extern "C" void pdfium_rust_bytestring_pool_destroy(void* state);
extern "C" bool pdfium_rust_bytestring_pool_get(const void* state,
                                                const uint8_t* data,
                                                size_t len,
                                                uintptr_t* output);
extern "C" bool pdfium_rust_bytestring_pool_insert(void* state,
                                                   const uint8_t* data,
                                                   size_t len,
                                                   uintptr_t handle);

namespace pdfium::rust {

RustByteStringPoolIndex::RustByteStringPoolIndex()
    : state_(pdfium_rust_bytestring_pool_new()) {
  CHECK(state_);
}

RustByteStringPoolIndex::~RustByteStringPoolIndex() {
  pdfium_rust_bytestring_pool_destroy(state_);
}

std::optional<uintptr_t> RustByteStringPoolIndex::Get(
    pdfium::span<const uint8_t> value) const {
  uintptr_t output = 0;
  if (!pdfium_rust_bytestring_pool_get(state_, value.data(), value.size(),
                                       &output)) {
    return std::nullopt;
  }
  return output;
}

bool RustByteStringPoolIndex::Insert(pdfium::span<const uint8_t> value,
                                     uintptr_t handle) {
  return pdfium_rust_bytestring_pool_insert(state_, value.data(), value.size(),
                                            handle);
}

}  // namespace pdfium::rust
