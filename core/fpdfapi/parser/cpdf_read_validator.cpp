// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_read_validator.h"

#include <utility>

#include "core/fxcrt/fx_safe_types.h"

CPDF_ReadValidator::ScopedSession::ScopedSession(
    RetainPtr<CPDF_ReadValidator> validator)
    : validator_(std::move(validator)),
      saved_read_error_(validator_->read_error_) {
  validator_->ResetErrors();
}

CPDF_ReadValidator::ScopedSession::~ScopedSession() {
  validator_->read_error_ |= saved_read_error_;
}

CPDF_ReadValidator::CPDF_ReadValidator(
    RetainPtr<IFX_SeekableReadStream> file_read)
    : file_read_(std::move(file_read)), file_size_(file_read_->GetSize()) {}

CPDF_ReadValidator::~CPDF_ReadValidator() = default;

void CPDF_ReadValidator::ResetErrors() {
  read_error_ = false;
}

bool CPDF_ReadValidator::ReadBlockAtOffset(pdfium::span<uint8_t> buffer,
                                           FX_FILESIZE offset) {
  if (offset < 0) {
    return false;
  }

  FX_SAFE_FILESIZE end_offset = offset;
  end_offset += buffer.size();
  if (!end_offset.IsValid() || end_offset.ValueOrDie() > file_size_) {
    return false;
  }

  if (file_read_->ReadBlockAtOffset(buffer, offset)) {
    return true;
  }

  read_error_ = true;
  return false;
}

FX_FILESIZE CPDF_ReadValidator::GetSize() {
  return file_size_;
}

bool CPDF_ReadValidator::CheckDataRangeAndRequestIfUnavailable(
    FX_FILESIZE offset,
    size_t size) {
  if (offset < 0) {
    return false;
  }
  FX_SAFE_FILESIZE end_offset = offset;
  end_offset += size;
  return end_offset.IsValid() && end_offset.ValueOrDie() <= file_size_;
}
