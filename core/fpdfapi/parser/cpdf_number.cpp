// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/parser/cpdf_number.h"

#include <sstream>
#include <variant>

#include "core/fpdfapi/edit/cpdf_contentstream_write_utils.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/fx_string_wrappers.h"

namespace {

ByteString FloatToString(float value) {
  fxcrt::ostringstream sstream;
  WriteFloat(sstream, value);
  return ByteString(sstream);
}

}  // namespace

CPDF_Number::CPDF_Number() {
  if (pdfium::rust::UseRustParserCandidate()) {
    number_ = std::make_unique<pdfium::rust::RustPdfNumber>();
  }
}

CPDF_Number::CPDF_Number(int value) : number_(FX_Number(value)) {
  if (pdfium::rust::UseRustParserCandidate()) {
    number_ = std::make_unique<pdfium::rust::RustPdfNumber>(value);
  }
}

CPDF_Number::CPDF_Number(float value) : number_(FX_Number(value)) {
  if (pdfium::rust::UseRustParserCandidate()) {
    number_ = std::make_unique<pdfium::rust::RustPdfNumber>(value);
  }
}

CPDF_Number::CPDF_Number(ByteStringView str) : number_(FX_Number(str)) {
  if (pdfium::rust::UseRustParserCandidate()) {
    number_ =
        std::make_unique<pdfium::rust::RustPdfNumber>(str.unsigned_span());
  }
}

CPDF_Number::~CPDF_Number() = default;

CPDF_Object::Type CPDF_Number::GetType() const {
  return kNumber;
}

RetainPtr<CPDF_Object> CPDF_Number::Clone() const {
  return IsInteger() ? pdfium::MakeRetain<CPDF_Number>(GetInteger())
                     : pdfium::MakeRetain<CPDF_Number>(GetNumber());
}

float CPDF_Number::GetNumber() const {
  if (const auto* cpp_number = std::get_if<FX_Number>(&number_)) {
    return cpp_number->GetFloat();
  }
  return std::get<std::unique_ptr<pdfium::rust::RustPdfNumber>>(number_)
      ->GetFloat();
}

int CPDF_Number::GetInteger() const {
  if (const auto* cpp_number = std::get_if<FX_Number>(&number_)) {
    return cpp_number->GetSigned();
  }
  return std::get<std::unique_ptr<pdfium::rust::RustPdfNumber>>(number_)
      ->GetSigned();
}

bool CPDF_Number::IsInteger() const {
  if (const auto* cpp_number = std::get_if<FX_Number>(&number_)) {
    return cpp_number->IsInteger();
  }
  return std::get<std::unique_ptr<pdfium::rust::RustPdfNumber>>(number_)
      ->IsInteger();
}

CPDF_Number* CPDF_Number::AsMutableNumber() {
  return this;
}

void CPDF_Number::SetString(const ByteString& str) {
  if (auto* cpp_number = std::get_if<FX_Number>(&number_)) {
    *cpp_number = FX_Number(str.AsStringView());
    return;
  }
  CHECK(std::get<std::unique_ptr<pdfium::rust::RustPdfNumber>>(number_)
            ->SetString(str.unsigned_span()));
}

ByteString CPDF_Number::GetString() const {
  return IsInteger() ? ByteString::FormatInteger(GetInteger())
                     : FloatToString(GetNumber());
}

bool CPDF_Number::WriteTo(IFX_ArchiveStream* archive,
                          const CPDF_Encryptor* encryptor) const {
  return archive->WriteString(" ") &&
         archive->WriteString(GetString().AsStringView());
}
