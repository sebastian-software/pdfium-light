// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_reference.h"

#include <vector>

#include "core/fpdfapi/parser/cpdf_indirect_object_holder.h"
#include "core/fpdfapi/parser/cpdf_number.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/notreached.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class ReferenceArchiveStream final : public IFX_ArchiveStream {
 public:
  bool WriteBlock(pdfium::span<const uint8_t> buffer) override {
    output_ += ByteStringView(buffer);
    return true;
  }
  FX_FILESIZE CurrentOffset() const override { NOTREACHED(); }

  const ByteString& output() const { return output_; }

 private:
  ByteString output_;
};

struct ReferenceSnapshot {
  uint32_t object_number;
  bool has_holder;
  int integer;
  float number;
  ByteString string;
  uint32_t clone_object_number;
  bool clone_has_holder;
  ByteString serialized;

  bool operator==(const ReferenceSnapshot&) const = default;
};

std::vector<ReferenceSnapshot> RunReferenceScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  CPDF_IndirectObjectHolder holder;
  auto target = holder.NewIndirect<CPDF_Number>(42);
  const uint32_t target_number = target->GetObjNum();
  std::vector<ReferenceSnapshot> result;
  const auto append = [&result](const CPDF_Reference& value) {
    ReferenceArchiveStream archive;
    EXPECT_TRUE(value.WriteTo(&archive, nullptr));
    RetainPtr<CPDF_Object> clone = value.Clone();
    const CPDF_Reference* clone_reference = ToReference(clone.Get());
    ASSERT_TRUE(clone_reference);
    result.push_back({
        .object_number = value.GetRefObjNum(),
        .has_holder = value.HasIndirectObjectHolder(),
        .integer = value.GetInteger(),
        .number = value.GetNumber(),
        .string = value.GetString(),
        .clone_object_number = clone_reference->GetRefObjNum(),
        .clone_has_holder = clone_reference->HasIndirectObjectHolder(),
        .serialized = archive.output(),
    });
  };

  auto value = pdfium::MakeRetain<CPDF_Reference>(nullptr, 0);
  append(*value);
  value->SetRef(&holder, target_number);
  append(*value);
  value->SetRef(&holder, UINT32_MAX);
  append(*value);
  value->SetRef(nullptr, 17);
  append(*value);
  return result;
}

}  // namespace

TEST(CPDFReference, RustCandidateMatchesCppValueScenario) {
  EXPECT_EQ(RunReferenceScenario(false), RunReferenceScenario(true));
}
