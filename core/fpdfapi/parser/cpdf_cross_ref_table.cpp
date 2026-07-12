// Copyright 2018 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_cross_ref_table.h"

#include <utility>

#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_parser.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check_op.h"
#include "core/fxcrt/containers/contains.h"

// static
std::unique_ptr<CPDF_CrossRefTable> CPDF_CrossRefTable::MergeUp(
    std::unique_ptr<CPDF_CrossRefTable> current,
    std::unique_ptr<CPDF_CrossRefTable> top) {
  if (!current) {
    return top;
  }

  if (!top) {
    return current;
  }

  current->Update(std::move(top));
  return current;
}

CPDF_CrossRefTable::CPDF_CrossRefTable()
    : use_rust_objects_info_(pdfium::rust::UseRustParserCandidate()),
      rust_objects_info_(
          use_rust_objects_info_
              ? std::make_unique<pdfium::rust::RustCrossRefTable>()
              : nullptr) {}

CPDF_CrossRefTable::CPDF_CrossRefTable(RetainPtr<CPDF_Dictionary> trailer,
                                       uint32_t trailer_object_number)
    : trailer_(std::move(trailer)),
      trailer_object_number_(trailer_object_number),
      use_rust_objects_info_(pdfium::rust::UseRustParserCandidate()),
      rust_objects_info_(
          use_rust_objects_info_
              ? std::make_unique<pdfium::rust::RustCrossRefTable>()
              : nullptr) {}

CPDF_CrossRefTable::~CPDF_CrossRefTable() = default;

void CPDF_CrossRefTable::AddCompressed(uint32_t obj_num,
                                       uint32_t archive_obj_num,
                                       uint32_t archive_obj_index) {
  CHECK_LE(obj_num, CPDF_Parser::kMaxObjectNumber);
  CHECK_LE(archive_obj_num, CPDF_Parser::kMaxObjectNumber);

  if (use_rust_objects_info_) {
    CHECK(rust_objects_info_->AddCompressed(obj_num, archive_obj_num,
                                            archive_obj_index));
    MarkObjectsInfoViewDirty();
    return;
  }

  auto& info = objects_info_[obj_num];
  if (info.gennum > 0) {
    return;
  }
  // Don't add known object streams to object streams.
  if (info.is_object_stream_flag) {
    return;
  }

  info.type = ObjectType::kCompressed;
  info.archive.obj_num = archive_obj_num;
  info.archive.obj_index = archive_obj_index;
  info.gennum = 0;

  objects_info_[archive_obj_num].is_object_stream_flag = true;
}

void CPDF_CrossRefTable::AddNormal(uint32_t obj_num,
                                   uint16_t gen_num,
                                   bool is_object_stream,
                                   FX_FILESIZE pos) {
  CHECK_LE(obj_num, CPDF_Parser::kMaxObjectNumber);

  if (use_rust_objects_info_) {
    CHECK(
        rust_objects_info_->AddNormal(obj_num, gen_num, is_object_stream, pos));
    MarkObjectsInfoViewDirty();
    return;
  }

  auto& info = objects_info_[obj_num];
  if (info.gennum > gen_num) {
    return;
  }

  info.type = ObjectType::kNormal;
  info.is_object_stream_flag |= is_object_stream;
  info.gennum = gen_num;
  info.pos = pos;
}

void CPDF_CrossRefTable::SetFree(uint32_t obj_num, uint16_t gen_num) {
  CHECK_LE(obj_num, CPDF_Parser::kMaxObjectNumber);

  if (use_rust_objects_info_) {
    CHECK(rust_objects_info_->SetFree(obj_num, gen_num));
    MarkObjectsInfoViewDirty();
    return;
  }

  auto& info = objects_info_[obj_num];
  info.type = ObjectType::kFree;
  info.gennum = gen_num;
  info.pos = 0;
}

void CPDF_CrossRefTable::SetTrailer(RetainPtr<CPDF_Dictionary> trailer,
                                    uint32_t trailer_object_number) {
  trailer_ = std::move(trailer);
  trailer_object_number_ = trailer_object_number;
}

const CPDF_CrossRefTable::ObjectInfo* CPDF_CrossRefTable::GetObjectInfo(
    uint32_t obj_num) const {
  EnsureObjectsInfoView();
  const auto it = objects_info_.find(obj_num);
  return it != objects_info_.end() ? &it->second : nullptr;
}

const std::map<uint32_t, CPDF_CrossRefTable::ObjectInfo>&
CPDF_CrossRefTable::objects_info() const {
  EnsureObjectsInfoView();
  return objects_info_;
}

void CPDF_CrossRefTable::Update(
    std::unique_ptr<CPDF_CrossRefTable> new_cross_ref) {
  CHECK_EQ(use_rust_objects_info_, new_cross_ref->use_rust_objects_info_);
  if (use_rust_objects_info_) {
    CHECK(rust_objects_info_->OverlayFrom(
        new_cross_ref->rust_objects_info_.get()));
    MarkObjectsInfoViewDirty();
  } else {
    UpdateInfo(std::move(new_cross_ref->objects_info_));
  }
  UpdateTrailer(std::move(new_cross_ref->trailer_));
}

void CPDF_CrossRefTable::SetObjectMapSize(uint32_t size) {
  if (use_rust_objects_info_) {
    CHECK(rust_objects_info_->SetSize(size));
    MarkObjectsInfoViewDirty();
    return;
  }

  if (size == 0) {
    objects_info_.clear();
    return;
  }

  objects_info_.erase(objects_info_.lower_bound(size), objects_info_.end());

  if (!pdfium::Contains(objects_info_, size - 1)) {
    objects_info_[size - 1].pos = 0;
  }
}

void CPDF_CrossRefTable::UpdateInfo(
    std::map<uint32_t, ObjectInfo> new_objects_info) {
  if (new_objects_info.empty()) {
    return;
  }

  if (objects_info_.empty()) {
    objects_info_ = std::move(new_objects_info);
    return;
  }

  auto cur_it = objects_info_.begin();
  auto new_it = new_objects_info.begin();
  while (cur_it != objects_info_.end() && new_it != new_objects_info.end()) {
    if (cur_it->first == new_it->first) {
      if (new_it->second.type == ObjectType::kNormal &&
          cur_it->second.type == ObjectType::kNormal &&
          cur_it->second.is_object_stream_flag) {
        new_it->second.is_object_stream_flag = true;
      }
      ++cur_it;
      ++new_it;
    } else if (cur_it->first < new_it->first) {
      new_objects_info.insert(new_it, *cur_it);
      ++cur_it;
    } else {
      new_it = new_objects_info.lower_bound(cur_it->first);
    }
  }
  for (; cur_it != objects_info_.end(); ++cur_it) {
    new_objects_info.insert(new_objects_info.end(), *cur_it);
  }
  objects_info_ = std::move(new_objects_info);
}

void CPDF_CrossRefTable::EnsureObjectsInfoView() const {
  if (!use_rust_objects_info_ || !objects_info_view_dirty_) {
    return;
  }

  objects_info_.clear();
  const auto append =
      [](void* raw_context, uint32_t object_number,
         const pdfium::rust::RustCrossRefObjectInfo& rust_info) -> bool {
    auto* objects =
        static_cast<std::map<uint32_t, CPDF_CrossRefTable::ObjectInfo>*>(
            raw_context);
    if (rust_info.type > static_cast<uint8_t>(ObjectType::kCompressed)) {
      return false;
    }
    ObjectInfo info;
    info.type = static_cast<ObjectType>(rust_info.type);
    info.is_object_stream_flag = rust_info.is_object_stream;
    info.gennum = rust_info.generation;
    if (info.type == ObjectType::kCompressed) {
      info.archive.obj_num = rust_info.archive_object_number;
      info.archive.obj_index = rust_info.archive_object_index;
    } else {
      info.pos = rust_info.position;
    }
    objects->emplace(object_number, info);
    return true;
  };
  CHECK(rust_objects_info_->Snapshot(&objects_info_, append));
  objects_info_view_dirty_ = false;
}

void CPDF_CrossRefTable::MarkObjectsInfoViewDirty() {
  CHECK(use_rust_objects_info_);
  objects_info_view_dirty_ = true;
}

void CPDF_CrossRefTable::UpdateTrailer(RetainPtr<CPDF_Dictionary> new_trailer) {
  if (!new_trailer) {
    return;
  }

  if (!trailer_) {
    trailer_ = std::move(new_trailer);
    return;
  }

  new_trailer->SetFor("XRefStm", trailer_->RemoveFor("XRefStm"));
  new_trailer->SetFor("Prev", trailer_->RemoveFor("Prev"));

  for (const auto& key : new_trailer->GetKeys()) {
    trailer_->SetFor(key, new_trailer->RemoveFor(key.AsStringView()));
  }
}
