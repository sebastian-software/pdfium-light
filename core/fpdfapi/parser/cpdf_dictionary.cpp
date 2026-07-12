// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_dictionary.h"

#include <set>
#include <utility>

#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_boolean.h"
#include "core/fpdfapi/parser/cpdf_crypto_handler.h"
#include "core/fpdfapi/parser/cpdf_name.h"
#include "core/fpdfapi/parser/cpdf_number.h"
#include "core/fpdfapi/parser/cpdf_reference.h"
#include "core/fpdfapi/parser/cpdf_stream.h"
#include "core/fpdfapi/parser/cpdf_string.h"
#include "core/fpdfapi/parser/fpdf_parser_utility.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/containers/contains.h"
#include "core/fxcrt/fx_stream.h"

CPDF_Dictionary::CPDF_Dictionary()
    : CPDF_Dictionary(WeakPtr<ByteStringPool>()) {}

CPDF_Dictionary::CPDF_Dictionary(const WeakPtr<ByteStringPool>& pPool)
    : pool_(pPool),
      use_rust_dictionary_(pdfium::rust::UseRustParserCandidate()) {
  if (use_rust_dictionary_) {
    rust_dictionary_ = std::make_unique<pdfium::rust::RustPdfDictionary>();
  }
}

CPDF_Dictionary::~CPDF_Dictionary() {
  // Mark the object as deleted so that it will not be deleted again,
  // and break cyclic references.
  obj_num_ = kInvalidObjNum;
  MarkMapViewDirty();
  if (use_rust_dictionary_) {
    for (auto& [handle, entry] : object_handles_) {
      if (entry.object->GetObjNum() == kInvalidObjNum) {
        entry.object.Leak();
      }
    }
  } else {
    for (auto& it : map_) {
      if (it.second->GetObjNum() == kInvalidObjNum) {
        it.second.Leak();
      }
    }
  }
}

CPDF_Object::Type CPDF_Dictionary::GetType() const {
  return kDictionary;
}

CPDF_Dictionary* CPDF_Dictionary::AsMutableDictionary() {
  return this;
}

RetainPtr<CPDF_Object> CPDF_Dictionary::Clone() const {
  return CloneObjectNonCyclic(false);
}

RetainPtr<CPDF_Object> CPDF_Dictionary::CloneNonCyclic(
    bool bDirect,
    std::set<const CPDF_Object*>* pVisited) const {
  pVisited->insert(this);
  auto pCopy = pdfium::MakeRetain<CPDF_Dictionary>(pool_);
  CPDF_DictionaryLocker locker(this);
  for (const auto& it : locker) {
    if (!pdfium::Contains(*pVisited, it.second.Get())) {
      std::set<const CPDF_Object*> visited(*pVisited);
      auto obj = it.second->CloneNonCyclic(bDirect, &visited);
      if (obj) {
        pCopy->SetFor(it.first, std::move(obj));
      }
    }
  }
  return pCopy;
}

const CPDF_Object* CPDF_Dictionary::GetObjectForInternal(
    ByteStringView key) const {
  if (use_rust_dictionary_) {
    std::optional<uintptr_t> handle =
        rust_dictionary_->Get(key.unsigned_span());
    return handle ? GetObjectForHandle(*handle) : nullptr;
  }
  auto it = map_.find(key);
  return it != map_.end() ? it->second.Get() : nullptr;
}

RetainPtr<const CPDF_Object> CPDF_Dictionary::GetObjectFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetObjectForInternal(key));
}

RetainPtr<CPDF_Object> CPDF_Dictionary::GetMutableObjectFor(
    ByteStringView key) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Object*>(GetObjectForInternal(key)));
}

const CPDF_Object* CPDF_Dictionary::GetDirectObjectForInternal(
    ByteStringView key) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetDirectInternal() : nullptr;
}

RetainPtr<const CPDF_Object> CPDF_Dictionary::GetDirectObjectFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetDirectObjectForInternal(key));
}

RetainPtr<CPDF_Object> CPDF_Dictionary::GetMutableDirectObjectFor(
    ByteStringView key) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Object*>(GetDirectObjectForInternal(key)));
}

ByteString CPDF_Dictionary::GetByteStringFor(ByteStringView key) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetString() : ByteString();
}

ByteString CPDF_Dictionary::GetByteStringFor(ByteStringView key,
                                             ByteStringView def) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetString() : ByteString(def);
}

WideString CPDF_Dictionary::GetUnicodeTextFor(ByteStringView key) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  if (const CPDF_Reference* pRef = ToReference(p)) {
    p = pRef->GetDirectInternal();
  }
  return p ? p->GetUnicodeText() : WideString();
}

ByteString CPDF_Dictionary::GetNameFor(ByteStringView key) const {
  const CPDF_Name* p = ToName(GetObjectForInternal(key));
  return p ? p->GetString() : ByteString();
}

bool CPDF_Dictionary::GetBooleanFor(ByteStringView key, bool bDefault) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return ToBoolean(p) ? p->GetInteger() != 0 : bDefault;
}

int CPDF_Dictionary::GetIntegerFor(ByteStringView key) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetInteger() : 0;
}

int CPDF_Dictionary::GetIntegerFor(ByteStringView key, int def) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetInteger() : def;
}

int CPDF_Dictionary::GetDirectIntegerFor(ByteStringView key) const {
  const CPDF_Number* p = ToNumber(GetObjectForInternal(key));
  return p ? p->GetInteger() : 0;
}

float CPDF_Dictionary::GetFloatFor(ByteStringView key) const {
  const CPDF_Object* p = GetObjectForInternal(key);
  return p ? p->GetNumber() : 0;
}

const CPDF_Dictionary* CPDF_Dictionary::GetDictInternal() const {
  return this;
}

const CPDF_Dictionary* CPDF_Dictionary::GetDictForInternal(
    ByteStringView key) const {
  const CPDF_Object* p = GetDirectObjectForInternal(key);
  return p ? p->GetDictInternal() : nullptr;
}

RetainPtr<const CPDF_Dictionary> CPDF_Dictionary::GetDictFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetDictForInternal(key));
}

RetainPtr<CPDF_Dictionary> CPDF_Dictionary::GetMutableDictFor(
    ByteStringView key) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Dictionary*>(GetDictForInternal(key)));
}

RetainPtr<CPDF_Dictionary> CPDF_Dictionary::GetOrCreateDictFor(
    ByteStringView key) {
  RetainPtr<CPDF_Dictionary> result = GetMutableDictFor(key);
  if (result) {
    return result;
  }
  return SetNewFor<CPDF_Dictionary>(ByteString(key));
}

const CPDF_Array* CPDF_Dictionary::GetArrayForInternal(
    ByteStringView key) const {
  return ToArray(GetDirectObjectForInternal(key));
}

RetainPtr<const CPDF_Array> CPDF_Dictionary::GetArrayFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetArrayForInternal(key));
}

RetainPtr<CPDF_Array> CPDF_Dictionary::GetMutableArrayFor(ByteStringView key) {
  return pdfium::WrapRetain(const_cast<CPDF_Array*>(GetArrayForInternal(key)));
}

RetainPtr<CPDF_Array> CPDF_Dictionary::GetOrCreateArrayFor(ByteStringView key) {
  RetainPtr<CPDF_Array> result = GetMutableArrayFor(key);
  if (result) {
    return result;
  }
  return SetNewFor<CPDF_Array>(ByteString(key));
}

const CPDF_Stream* CPDF_Dictionary::GetStreamForInternal(
    ByteStringView key) const {
  return ToStream(GetDirectObjectForInternal(key));
}

RetainPtr<const CPDF_Stream> CPDF_Dictionary::GetStreamFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetStreamForInternal(key));
}

RetainPtr<CPDF_Stream> CPDF_Dictionary::GetMutableStreamFor(
    ByteStringView key) {
  return pdfium::WrapRetain(
      const_cast<CPDF_Stream*>(GetStreamForInternal(key)));
}

const CPDF_Number* CPDF_Dictionary::GetNumberForInternal(
    ByteStringView key) const {
  return ToNumber(GetObjectForInternal(key));
}

RetainPtr<const CPDF_Number> CPDF_Dictionary::GetNumberFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetNumberForInternal(key));
}

const CPDF_String* CPDF_Dictionary::GetStringForInternal(
    ByteStringView key) const {
  return ToString(GetObjectForInternal(key));
}

RetainPtr<const CPDF_String> CPDF_Dictionary::GetStringFor(
    ByteStringView key) const {
  return pdfium::WrapRetain(GetStringForInternal(key));
}

CFX_FloatRect CPDF_Dictionary::GetRectFor(ByteStringView key) const {
  const CPDF_Array* pArray = GetArrayForInternal(key);
  if (pArray) {
    return pArray->GetRect();
  }
  return CFX_FloatRect();
}

CFX_Matrix CPDF_Dictionary::GetMatrixFor(ByteStringView key) const {
  const CPDF_Array* pArray = GetArrayForInternal(key);
  if (pArray) {
    return pArray->GetMatrix();
  }
  return CFX_Matrix();
}

bool CPDF_Dictionary::KeyExist(ByteStringView key) const {
  return !!GetObjectForInternal(key);
}

std::vector<ByteString> CPDF_Dictionary::GetKeys() const {
  std::vector<ByteString> result;
  CPDF_DictionaryLocker locker(this);
  for (const auto& item : locker) {
    result.push_back(item.first);
  }
  return result;
}

void CPDF_Dictionary::SetFor(const ByteString& key,
                             RetainPtr<CPDF_Object> object) {
  (void)SetForInternal(key, std::move(object));
}

CPDF_Object* CPDF_Dictionary::SetForInternal(const ByteString& key,
                                             RetainPtr<CPDF_Object> pObj) {
  CHECK(!IsLocked());
  if (!pObj) {
    if (!use_rust_dictionary_) {
      map_.erase(key);
    } else if (std::optional<uintptr_t> old_handle =
                   rust_dictionary_->Remove(key.unsigned_span())) {
      MarkMapViewDirty();
      ReleaseObjectHandle(*old_handle);
    }
    return nullptr;
  }
  CHECK(pObj->IsInline());
  CHECK(!pObj->IsStream());
  CPDF_Object* pRet = pObj.Get();
  if (!use_rust_dictionary_) {
    map_[MaybeIntern(key)] = std::move(pObj);
    return pRet;
  }
  uintptr_t handle = RegisterObject(std::move(pObj));
  std::optional<uintptr_t> old_handle =
      rust_dictionary_->Set(key.unsigned_span(), handle);
  MarkMapViewDirty();
  if (old_handle) {
    ReleaseObjectHandle(*old_handle);
  }
  return pRet;
}

void CPDF_Dictionary::ConvertToIndirectObjectFor(
    const ByteString& key,
    CPDF_IndirectObjectHolder* pHolder) {
  CHECK(!IsLocked());
  RetainPtr<CPDF_Object> object = GetMutableObjectFor(key);
  if (!object || object->IsReference()) {
    return;
  }

  pHolder->AddIndirectObject(object);
  SetFor(key, object->MakeReference(pHolder));
}

RetainPtr<CPDF_Object> CPDF_Dictionary::RemoveFor(ByteStringView key) {
  CHECK(!IsLocked());
  if (use_rust_dictionary_) {
    std::optional<uintptr_t> handle =
        rust_dictionary_->Remove(key.unsigned_span());
    if (!handle) {
      return nullptr;
    }
    RetainPtr<CPDF_Object> result =
        pdfium::WrapRetain(GetObjectForHandle(*handle));
    MarkMapViewDirty();
    ReleaseObjectHandle(*handle);
    return result;
  }
  auto it = map_.find(key);
  if (it == map_.end()) {
    return RetainPtr<CPDF_Object>();
  }
  auto node = map_.extract(it);
  return std::move(node.mapped());
}

void CPDF_Dictionary::ReplaceKey(const ByteString& oldkey,
                                 const ByteString& newkey) {
  CHECK(!IsLocked());
  if (use_rust_dictionary_) {
    RetainPtr<CPDF_Object> object = RemoveFor(oldkey.AsStringView());
    if (object) {
      SetFor(newkey, std::move(object));
    }
    return;
  }
  auto old_it = map_.find(oldkey);
  if (old_it == map_.end()) {
    return;
  }

  auto new_it = map_.find(newkey);
  if (new_it == old_it) {
    return;
  }

  map_[MaybeIntern(newkey)] = std::move(old_it->second);
  map_.erase(old_it);
}

void CPDF_Dictionary::SetRectFor(const ByteString& key,
                                 const CFX_FloatRect& rect) {
  auto pArray = SetNewFor<CPDF_Array>(key);
  pArray->AppendNew<CPDF_Number>(rect.left);
  pArray->AppendNew<CPDF_Number>(rect.bottom);
  pArray->AppendNew<CPDF_Number>(rect.right);
  pArray->AppendNew<CPDF_Number>(rect.top);
}

void CPDF_Dictionary::SetMatrixFor(const ByteString& key,
                                   const CFX_Matrix& matrix) {
  auto pArray = SetNewFor<CPDF_Array>(key);
  pArray->AppendNew<CPDF_Number>(matrix.a);
  pArray->AppendNew<CPDF_Number>(matrix.b);
  pArray->AppendNew<CPDF_Number>(matrix.c);
  pArray->AppendNew<CPDF_Number>(matrix.d);
  pArray->AppendNew<CPDF_Number>(matrix.e);
  pArray->AppendNew<CPDF_Number>(matrix.f);
}

ByteString CPDF_Dictionary::MaybeIntern(const ByteString& str) {
  return pool_ ? pool_->Intern(str) : str;
}

size_t CPDF_Dictionary::size() const {
  return use_rust_dictionary_ ? rust_dictionary_->size() : map_.size();
}

uintptr_t CPDF_Dictionary::RegisterObject(RetainPtr<CPDF_Object> object) {
  CHECK(object);
  const uintptr_t handle = reinterpret_cast<uintptr_t>(object.Get());
  auto [it, inserted] =
      object_handles_.try_emplace(handle, ObjectHandle{std::move(object), 0});
  ++it->second.reference_count;
  return handle;
}

CPDF_Object* CPDF_Dictionary::GetObjectForHandle(uintptr_t handle) const {
  auto it = object_handles_.find(handle);
  CHECK(it != object_handles_.end());
  return it->second.object.Get();
}

void CPDF_Dictionary::ReleaseObjectHandle(uintptr_t handle) {
  auto it = object_handles_.find(handle);
  CHECK(it != object_handles_.end());
  CHECK_GT(it->second.reference_count, 0u);
  if (--it->second.reference_count == 0) {
    object_handles_.erase(it);
  }
}

void CPDF_Dictionary::MarkMapViewDirty() {
  if (!use_rust_dictionary_) {
    return;
  }
  map_.clear();
  map_view_dirty_ = true;
}

void CPDF_Dictionary::EnsureMapView() const {
  if (!use_rust_dictionary_ || !map_view_dirty_) {
    return;
  }
  const auto append = [](void* context, const uint8_t* key, size_t key_len,
                         uintptr_t handle) {
    auto* dictionary = static_cast<const CPDF_Dictionary*>(context);
    ByteString key_string(ByteStringView(pdfium::span(key, key_len)));
    dictionary->map_.emplace(
        dictionary->pool_ ? dictionary->pool_->Intern(key_string)
                          : std::move(key_string),
        pdfium::WrapRetain(dictionary->GetObjectForHandle(handle)));
    return true;
  };
  CHECK(rust_dictionary_->Snapshot(const_cast<CPDF_Dictionary*>(this), append));
  map_view_dirty_ = false;
}

bool CPDF_Dictionary::WriteTo(IFX_ArchiveStream* archive,
                              const CPDF_Encryptor* encryptor) const {
  if (!archive->WriteString("<<")) {
    return false;
  }

  const bool is_signature = CPDF_CryptoHandler::IsSignatureDictionary(this);

  CPDF_DictionaryLocker locker(this);
  for (const auto& it : locker) {
    const ByteString& key = it.first;
    const RetainPtr<CPDF_Object>& pValue = it.second;
    if (!archive->WriteString("/") ||
        !archive->WriteString(PDF_NameEncode(key).AsStringView())) {
      return false;
    }
    if (!pValue->WriteTo(archive, !is_signature || key != "Contents"
                                      ? encryptor
                                      : nullptr)) {
      return false;
    }
  }
  return archive->WriteString(">>");
}

CPDF_DictionaryLocker::CPDF_DictionaryLocker(const CPDF_Dictionary* dict)
    : dict_(dict) {
  dict_->lock_count_++;
}

CPDF_DictionaryLocker::CPDF_DictionaryLocker(RetainPtr<CPDF_Dictionary> dict)
    : dict_(std::move(dict)) {
  dict_->lock_count_++;
}

CPDF_DictionaryLocker::CPDF_DictionaryLocker(
    RetainPtr<const CPDF_Dictionary> dict)
    : dict_(std::move(dict)) {
  dict_->lock_count_++;
}

CPDF_DictionaryLocker::~CPDF_DictionaryLocker() {
  dict_->lock_count_--;
}
