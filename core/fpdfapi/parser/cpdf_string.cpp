// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_string.h"

#include <stdint.h>

#include <utility>

#include "core/fpdfapi/parser/cpdf_encryptor.h"
#include "core/fpdfapi/parser/fpdf_parser_decode.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/data_vector.h"
#include "core/fxcrt/fx_stream.h"

CPDF_String::CPDF_String() {
  if (pdfium::rust::UseRustParserCandidate()) {
    rust_value_ = std::make_unique<pdfium::rust::RustPdfString>(
        data_.unsigned_span(), false);
  }
}

CPDF_String::CPDF_String(WeakPtr<ByteStringPool> pool,
                         pdfium::span<const uint8_t> data,
                         DataType is_hex)
    : data_(ByteStringView(data)), output_is_hex_(true) {
  if (pool) {
    data_ = pool->Intern(data_);
  }
  if (pdfium::rust::UseRustParserCandidate()) {
    rust_value_ = std::make_unique<pdfium::rust::RustPdfString>(
        data_.unsigned_span(), true);
  }
}

CPDF_String::CPDF_String(WeakPtr<ByteStringPool> pool, const ByteString& str)
    : data_(str) {
  if (pool) {
    data_ = pool->Intern(data_);
  }
  if (pdfium::rust::UseRustParserCandidate()) {
    rust_value_ = std::make_unique<pdfium::rust::RustPdfString>(
        data_.unsigned_span(), false);
  }
}

CPDF_String::CPDF_String(WeakPtr<ByteStringPool> pool, WideStringView str)
    : CPDF_String(pool, PDF_EncodeText(str)) {
  // Delegates to ctor above.
}

CPDF_String::~CPDF_String() = default;

CPDF_Object::Type CPDF_String::GetType() const {
  return kString;
}

RetainPtr<CPDF_Object> CPDF_String::Clone() const {
  auto clone = pdfium::MakeRetain<CPDF_String>();
  clone->data_ = data_;
  clone->output_is_hex_ = output_is_hex_;
  if (clone->rust_value_) {
    clone->rust_value_ = std::make_unique<pdfium::rust::RustPdfString>(
        data_.unsigned_span(), IsHex());
  }
  return clone;
}

ByteString CPDF_String::GetString() const {
  if (rust_value_) {
    CHECK(rust_value_->Equals(data_.unsigned_span()));
  }
  return data_;
}

void CPDF_String::SetString(const ByteString& str) {
  if (rust_value_) {
    CHECK(rust_value_->Set(str.unsigned_span()));
  }
  data_ = str;
}

bool CPDF_String::IsHex() const {
  return rust_value_ ? rust_value_->IsHex() : output_is_hex_;
}

CPDF_String* CPDF_String::AsMutableString() {
  return this;
}

WideString CPDF_String::GetUnicodeText() const {
  return PDF_DecodeText(data_.unsigned_span());
}

bool CPDF_String::WriteTo(IFX_ArchiveStream* archive,
                          const CPDF_Encryptor* encryptor) const {
  DataVector<uint8_t> encrypted_data;
  ByteString value = GetString();
  pdfium::span<const uint8_t> data = value.unsigned_span();
  if (encryptor) {
    encrypted_data = encryptor->Encrypt(data);
    data = encrypted_data;
  }
  ByteStringView raw(data);
  ByteString content =
      IsHex() ? PDF_HexEncodeString(raw) : PDF_EncodeString(raw);
  return archive->WriteString(content.AsStringView());
}

ByteString CPDF_String::EncodeString() const {
  ByteString value = GetString();
  return IsHex() ? PDF_HexEncodeString(value.AsStringView())
                 : PDF_EncodeString(value.AsStringView());
}
