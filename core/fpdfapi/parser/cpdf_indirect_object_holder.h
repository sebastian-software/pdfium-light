// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FPDFAPI_PARSER_CPDF_INDIRECT_OBJECT_HOLDER_H_
#define CORE_FPDFAPI_PARSER_CPDF_INDIRECT_OBJECT_HOLDER_H_

#include <stddef.h>
#include <stdint.h>

#include <map>
#include <memory>
#include <type_traits>
#include <utility>

#include "core/fpdfapi/parser/cpdf_object.h"
#include "core/fxcrt/bytestring_pool.h"
#include "core/fxcrt/retain_ptr.h"
#include "core/fxcrt/weak_ptr.h"

namespace pdfium::rust {
class RustIndirectObjectIndex;
}

class CPDF_IndirectObjectHolder {
 public:
  using const_iterator =
      std::map<uint32_t, RetainPtr<CPDF_Object>>::const_iterator;

  CPDF_IndirectObjectHolder();
  virtual ~CPDF_IndirectObjectHolder();

  RetainPtr<CPDF_Object> GetOrParseIndirectObject(uint32_t objnum);
  RetainPtr<const CPDF_Object> GetIndirectObject(uint32_t objnum) const;
  RetainPtr<CPDF_Object> GetMutableIndirectObject(uint32_t objnum);
  void DeleteIndirectObject(uint32_t objnum);

  // Creates and adds a new object retained by the indirect object holder,
  // and returns a retained pointer to it.
  template <typename T, typename... Args>
  RetainPtr<T> NewIndirect(Args&&... args) {
    auto obj = New<T>(std::forward<Args>(args)...);
    AddIndirectObject(obj);
    return obj;
  }

  // Creates and adds a new object not retained by the indirect object holder,
  // but which can intern strings from it. We have a special cast to handle
  // objects that can intern strings from our ByteStringPool.
  template <typename T, typename... Args>
    requires(CanInternStrings<T>::value)
  RetainPtr<T> New(Args&&... args) {
    return pdfium::MakeRetain<T>(byte_string_pool_,
                                 std::forward<Args>(args)...);
  }
  template <typename T, typename... Args>
    requires(!CanInternStrings<T>::value)
  RetainPtr<T> New(Args&&... args) {
    return pdfium::MakeRetain<T>(std::forward<Args>(args)...);
  }

  // Always Retains |pObj|, returns its new object number.
  uint32_t AddIndirectObject(RetainPtr<CPDF_Object> pObj);

  // If higher generation number, retains |pObj| and returns true.
  bool ReplaceIndirectObjectIfHigherGeneration(uint32_t objnum,
                                               RetainPtr<CPDF_Object> pObj);

  uint32_t GetLastObjNum() const;
  void SetLastObjNum(uint32_t objnum);

  WeakPtr<ByteStringPool> GetByteStringPool() const {
    return byte_string_pool_;
  }

  const_iterator begin() const;
  const_iterator end() const;

 protected:
  virtual RetainPtr<CPDF_Object> ParseIndirectObject(uint32_t objnum);

 private:
  friend class CPDF_Reference;

  struct ObjectHandle {
    RetainPtr<CPDF_Object> object;
    size_t reference_count;
  };

  const CPDF_Object* GetIndirectObjectInternal(uint32_t objnum) const;
  CPDF_Object* GetOrParseIndirectObjectInternal(uint32_t objnum);
  uintptr_t RegisterObject(RetainPtr<CPDF_Object> object);
  CPDF_Object* GetObjectForHandle(uintptr_t handle) const;
  void ReleaseObjectHandle(uintptr_t handle);
  void MarkIndirectObjectsViewDirty();
  void EnsureIndirectObjectsView() const;

  const bool use_rust_object_index_;
  std::unique_ptr<pdfium::rust::RustIndirectObjectIndex> rust_object_index_;
  uint32_t last_obj_num_ = 0;
  mutable std::map<uint32_t, RetainPtr<CPDF_Object>> indirect_objs_;
  std::map<uintptr_t, ObjectHandle> indirect_object_handles_;
  mutable bool indirect_objects_view_dirty_ = false;
  WeakPtr<ByteStringPool> byte_string_pool_;
};

#endif  // CORE_FPDFAPI_PARSER_CPDF_INDIRECT_OBJECT_HOLDER_H_
