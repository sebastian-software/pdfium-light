// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_indirect_object_holder.h"

#include <algorithm>
#include <memory>
#include <utility>

#include "core/fpdfapi/parser/cpdf_object.h"
#include "core/fpdfapi/parser/cpdf_parser.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"

namespace {

const CPDF_Object* FilterInvalidObjNum(const CPDF_Object* obj) {
  return obj && obj->GetObjNum() != CPDF_Object::kInvalidObjNum ? obj : nullptr;
}

}  // namespace

CPDF_IndirectObjectHolder::CPDF_IndirectObjectHolder()
    : use_rust_object_index_(pdfium::rust::UseRustParserCandidate()),
      rust_object_index_(
          use_rust_object_index_
              ? std::make_unique<pdfium::rust::RustIndirectObjectIndex>()
              : nullptr),
      byte_string_pool_(std::make_unique<ByteStringPool>()) {}

CPDF_IndirectObjectHolder::~CPDF_IndirectObjectHolder() {
  byte_string_pool_.DeleteObject();  // Make weak.
}

RetainPtr<const CPDF_Object> CPDF_IndirectObjectHolder::GetIndirectObject(
    uint32_t objnum) const {
  return pdfium::WrapRetain(GetIndirectObjectInternal(objnum));
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::GetMutableIndirectObject(
    uint32_t objnum) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Object*>(GetIndirectObjectInternal(objnum)));
}

const CPDF_Object* CPDF_IndirectObjectHolder::GetIndirectObjectInternal(
    uint32_t objnum) const {
  if (use_rust_object_index_) {
    const auto lookup = rust_object_index_->Lookup(objnum);
    CHECK(lookup.has_value());
    return lookup->status == 2
               ? FilterInvalidObjNum(GetObjectForHandle(lookup->handle))
               : nullptr;
  }

  auto it = indirect_objs_.find(objnum);
  if (it == indirect_objs_.end()) {
    return nullptr;
  }

  return FilterInvalidObjNum(it->second.Get());
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::GetOrParseIndirectObject(
    uint32_t objnum) {
  return pdfium::WrapRetain(GetOrParseIndirectObjectInternal(objnum));
}

CPDF_Object* CPDF_IndirectObjectHolder::GetOrParseIndirectObjectInternal(
    uint32_t objnum) {
  if (objnum == 0 || objnum == CPDF_Object::kInvalidObjNum) {
    return nullptr;
  }

  if (use_rust_object_index_) {
    const auto lookup = rust_object_index_->ReserveParse(objnum);
    CHECK(lookup.has_value());
    if (lookup->status == 1) {
      return nullptr;
    }
    if (lookup->status == 2) {
      return FilterInvalidObjNum(GetObjectForHandle(lookup->handle));
    }

    MarkIndirectObjectsViewDirty();
    RetainPtr<CPDF_Object> new_object = ParseIndirectObject(objnum);
    if (!new_object) {
      CHECK(rust_object_index_->CancelParse(objnum));
      MarkIndirectObjectsViewDirty();
      return nullptr;
    }

    new_object->SetObjNum(objnum);
    CPDF_Object* result = new_object.Get();
    const uintptr_t handle = RegisterObject(std::move(new_object));
    CHECK(rust_object_index_->FinishParse(objnum, handle));
    MarkIndirectObjectsViewDirty();
    return result;
  }

  // Add item anyway to prevent recursively parsing of same object.
  auto insert_result = indirect_objs_.insert(std::make_pair(objnum, nullptr));
  if (!insert_result.second) {
    return const_cast<CPDF_Object*>(
        FilterInvalidObjNum(insert_result.first->second.Get()));
  }
  RetainPtr<CPDF_Object> pNewObj = ParseIndirectObject(objnum);
  if (!pNewObj) {
    indirect_objs_.erase(insert_result.first);
    return nullptr;
  }

  pNewObj->SetObjNum(objnum);
  last_obj_num_ = std::max(last_obj_num_, objnum);

  CPDF_Object* result = pNewObj.Get();
  insert_result.first->second = std::move(pNewObj);
  return result;
}

uint32_t CPDF_IndirectObjectHolder::GetLastObjNum() const {
  if (!use_rust_object_index_) {
    return last_obj_num_;
  }
  const auto result = rust_object_index_->GetLastObjectNumber();
  CHECK(result.has_value());
  return *result;
}

void CPDF_IndirectObjectHolder::SetLastObjNum(uint32_t objnum) {
  if (use_rust_object_index_) {
    CHECK(rust_object_index_->SetLastObjectNumber(objnum));
    return;
  }
  last_obj_num_ = objnum;
}

CPDF_IndirectObjectHolder::const_iterator CPDF_IndirectObjectHolder::begin()
    const {
  EnsureIndirectObjectsView();
  return indirect_objs_.begin();
}

CPDF_IndirectObjectHolder::const_iterator CPDF_IndirectObjectHolder::end()
    const {
  EnsureIndirectObjectsView();
  return indirect_objs_.end();
}

uintptr_t CPDF_IndirectObjectHolder::RegisterObject(
    RetainPtr<CPDF_Object> object) {
  CPDF_Object* raw_object = object.Get();
  const uintptr_t handle = reinterpret_cast<uintptr_t>(raw_object);
  CHECK(handle);
  auto [it, inserted] = indirect_object_handles_.try_emplace(
      handle, ObjectHandle{.object = std::move(object), .reference_count = 1});
  if (!inserted) {
    CHECK_EQ(raw_object, it->second.object.Get());
    ++it->second.reference_count;
  }
  return handle;
}

CPDF_Object* CPDF_IndirectObjectHolder::GetObjectForHandle(
    uintptr_t handle) const {
  const auto it = indirect_object_handles_.find(handle);
  CHECK(it != indirect_object_handles_.end());
  return it->second.object.Get();
}

void CPDF_IndirectObjectHolder::ReleaseObjectHandle(uintptr_t handle) {
  auto it = indirect_object_handles_.find(handle);
  CHECK(it != indirect_object_handles_.end());
  CHECK_GT(it->second.reference_count, 0u);
  if (--it->second.reference_count == 0) {
    indirect_object_handles_.erase(it);
  }
}

void CPDF_IndirectObjectHolder::MarkIndirectObjectsViewDirty() {
  CHECK(use_rust_object_index_);
  indirect_objs_.clear();
  indirect_objects_view_dirty_ = true;
}

void CPDF_IndirectObjectHolder::EnsureIndirectObjectsView() const {
  if (!use_rust_object_index_ || !indirect_objects_view_dirty_) {
    return;
  }
  indirect_objs_.clear();
  const auto append = [](void* raw_context, uint32_t object_number,
                         uintptr_t handle) -> bool {
    auto* holder = static_cast<CPDF_IndirectObjectHolder*>(raw_context);
    RetainPtr<CPDF_Object> object;
    if (handle != 0) {
      object = pdfium::WrapRetain(holder->GetObjectForHandle(handle));
    }
    holder->indirect_objs_.emplace(object_number, std::move(object));
    return true;
  };
  CHECK(rust_object_index_->Snapshot(
      const_cast<CPDF_IndirectObjectHolder*>(this), append));
  indirect_objects_view_dirty_ = false;
}

RetainPtr<CPDF_Object> CPDF_IndirectObjectHolder::ParseIndirectObject(
    uint32_t objnum) {
  return nullptr;
}

uint32_t CPDF_IndirectObjectHolder::AddIndirectObject(
    RetainPtr<CPDF_Object> pObj) {
  CHECK(!pObj->GetObjNum());
  if (use_rust_object_index_) {
    CPDF_Object* object = pObj.Get();
    const uintptr_t handle = RegisterObject(std::move(pObj));
    const auto add = rust_object_index_->Add(handle);
    CHECK(add.has_value());
    object->SetObjNum(add->object_number);
    if (add->old_handle.has_value()) {
      ReleaseObjectHandle(*add->old_handle);
    }
    MarkIndirectObjectsViewDirty();
    return add->object_number;
  }

  pObj->SetObjNum(++last_obj_num_);
  indirect_objs_[last_obj_num_] = std::move(pObj);
  return last_obj_num_;
}

bool CPDF_IndirectObjectHolder::ReplaceIndirectObjectIfHigherGeneration(
    uint32_t objnum,
    RetainPtr<CPDF_Object> pObj) {
  DCHECK(objnum);
  if (!pObj || objnum == CPDF_Object::kInvalidObjNum) {
    return false;
  }

  if (use_rust_object_index_) {
    const auto lookup = rust_object_index_->Lookup(objnum);
    CHECK(lookup.has_value());
    std::optional<uint32_t> old_generation;
    if (lookup->status == 2) {
      const CPDF_Object* old_object =
          FilterInvalidObjNum(GetObjectForHandle(lookup->handle));
      if (old_object) {
        old_generation = old_object->GetGenNum();
      }
    }
    const uintptr_t handle = reinterpret_cast<uintptr_t>(pObj.Get());
    const auto replace = rust_object_index_->Replace(
        objnum, handle, pObj->GetGenNum(), old_generation);
    CHECK(replace.has_value());
    if (!replace->applied) {
      return false;
    }

    pObj->SetObjNum(objnum);
    RegisterObject(std::move(pObj));
    if (replace->old_handle.has_value()) {
      ReleaseObjectHandle(*replace->old_handle);
    }
    MarkIndirectObjectsViewDirty();
    return true;
  }

  auto& obj_holder = indirect_objs_[objnum];
  const CPDF_Object* old_object = FilterInvalidObjNum(obj_holder.Get());
  if (old_object && pObj->GetGenNum() <= old_object->GetGenNum()) {
    return false;
  }

  pObj->SetObjNum(objnum);
  obj_holder = std::move(pObj);
  last_obj_num_ = std::max(last_obj_num_, objnum);
  return true;
}

void CPDF_IndirectObjectHolder::DeleteIndirectObject(uint32_t objnum) {
  if (use_rust_object_index_) {
    const auto lookup = rust_object_index_->Lookup(objnum);
    CHECK(lookup.has_value());
    if (lookup->status != 2 ||
        !FilterInvalidObjNum(GetObjectForHandle(lookup->handle))) {
      return;
    }
    const auto deleted = rust_object_index_->Delete(objnum);
    CHECK(deleted.has_value());
    CHECK(deleted->has_value());
    ReleaseObjectHandle(**deleted);
    MarkIndirectObjectsViewDirty();
    return;
  }

  auto it = indirect_objs_.find(objnum);
  if (it == indirect_objs_.end() || !FilterInvalidObjNum(it->second.Get())) {
    return;
  }

  indirect_objs_.erase(it);
}
