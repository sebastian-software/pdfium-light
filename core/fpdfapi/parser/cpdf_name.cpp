// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_name.h"

#include "core/fpdfapi/parser/fpdf_parser_decode.h"
#include "core/fpdfapi/parser/fpdf_parser_utility.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/fx_stream.h"

CPDF_Name::CPDF_Name(WeakPtr<ByteStringPool> pPool, const ByteString& str)
    : name_(str) {
  if (pPool) {
    name_ = pPool->Intern(name_);
  }
  if (pdfium::rust::UseRustParserCandidate()) {
    rust_value_ = std::make_unique<pdfium::rust::RustPdfString>(
        name_.unsigned_span(), false);
  }
}

CPDF_Name::~CPDF_Name() = default;

CPDF_Object::Type CPDF_Name::GetType() const {
  return kName;
}

RetainPtr<CPDF_Object> CPDF_Name::Clone() const {
  return pdfium::MakeRetain<CPDF_Name>(nullptr, GetString());
}

ByteString CPDF_Name::GetString() const {
  if (rust_value_) {
    CHECK(rust_value_->Equals(name_.unsigned_span()));
  }
  return name_;
}

void CPDF_Name::SetString(const ByteString& str) {
  if (rust_value_) {
    CHECK(rust_value_->Set(str.unsigned_span()));
  }
  name_ = str;
}

CPDF_Name* CPDF_Name::AsMutableName() {
  return this;
}

WideString CPDF_Name::GetUnicodeText() const {
  return PDF_DecodeText(GetString().unsigned_span());
}

bool CPDF_Name::WriteTo(IFX_ArchiveStream* archive,
                        const CPDF_Encryptor* encryptor) const {
  if (!archive->WriteString("/")) {
    return false;
  }

  const ByteString name = PDF_NameEncode(GetString());
  return name.IsEmpty() || archive->WriteString(name.AsStringView());
}
