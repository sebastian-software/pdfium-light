// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_boolean.h"

#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/fx_stream.h"

CPDF_Boolean::CPDF_Boolean() : CPDF_Boolean(false) {}

CPDF_Boolean::CPDF_Boolean(bool value) : value_(value) {
  if (pdfium::rust::UseRustParserCandidate()) {
    value_ = std::make_unique<pdfium::rust::RustPdfBoolean>(value);
  }
}

CPDF_Boolean::~CPDF_Boolean() = default;

CPDF_Object::Type CPDF_Boolean::GetType() const {
  return kBoolean;
}

RetainPtr<CPDF_Object> CPDF_Boolean::Clone() const {
  return pdfium::MakeRetain<CPDF_Boolean>(GetValue());
}

ByteString CPDF_Boolean::GetString() const {
  return GetValue() ? "true" : "false";
}

int CPDF_Boolean::GetInteger() const {
  return GetValue();
}

void CPDF_Boolean::SetString(const ByteString& str) {
  if (auto* value = std::get_if<bool>(&value_)) {
    *value = (str == "true");
    return;
  }
  CHECK(std::get<std::unique_ptr<pdfium::rust::RustPdfBoolean>>(value_)
            ->SetString(str.unsigned_span()));
}

bool CPDF_Boolean::GetValue() const {
  if (const auto* value = std::get_if<bool>(&value_)) {
    return *value;
  }
  return std::get<std::unique_ptr<pdfium::rust::RustPdfBoolean>>(value_)->Get();
}

CPDF_Boolean* CPDF_Boolean::AsMutableBoolean() {
  return this;
}

bool CPDF_Boolean::WriteTo(IFX_ArchiveStream* archive,
                           const CPDF_Encryptor* encryptor) const {
  return archive->WriteString(" ") &&
         archive->WriteString(GetString().AsStringView());
}
