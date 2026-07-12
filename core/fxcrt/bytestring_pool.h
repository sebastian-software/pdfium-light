// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXCRT_BYTESTRING_POOL_H_
#define CORE_FXCRT_BYTESTRING_POOL_H_

#include <stdint.h>

#include <map>
#include <memory>

#include "core/fxcrt/bytestring.h"

namespace pdfium::rust {
class RustByteStringPoolIndex;
}

namespace fxcrt {

class ByteStringPool {
 public:
  ByteStringPool();
  ~ByteStringPool();

  ByteString Intern(const ByteString& str);

 private:
  std::unique_ptr<pdfium::rust::RustByteStringPoolIndex> index_;
  std::map<uintptr_t, ByteString> strings_;
  uintptr_t next_handle_ = 1;
};

}  // namespace fxcrt

using fxcrt::ByteStringPool;

#endif  // CORE_FXCRT_BYTESTRING_POOL_H_
