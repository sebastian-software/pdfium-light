// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fxcrt/bytestring_pool.h"

#include "core/fxcrt/check.h"
#include "core/fxcrt/rust/rust_fxcrt_adapter.h"

namespace fxcrt {

ByteStringPool::ByteStringPool()
    : index_(std::make_unique<pdfium::rust::RustByteStringPoolIndex>()) {}

ByteStringPool::~ByteStringPool() = default;

ByteString ByteStringPool::Intern(const ByteString& str) {
  if (str.IsEmpty()) {
    return str;
  }
  if (std::optional<uintptr_t> handle = index_->Get(str.unsigned_span())) {
    auto it = strings_.find(*handle);
    CHECK(it != strings_.end());
    return it->second;
  }
  const uintptr_t handle = next_handle_++;
  CHECK_NE(handle, 0u);
  auto [it, inserted] = strings_.emplace(handle, str);
  CHECK(inserted);
  CHECK(index_->Insert(str.unsigned_span(), handle));
  return it->second;
}

}  // namespace fxcrt
