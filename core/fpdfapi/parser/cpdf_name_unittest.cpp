// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_name.h"

#include <vector>

#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/notreached.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class NameArchiveStream final : public IFX_ArchiveStream {
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

struct NameSnapshot {
  ByteString value;
  ByteString clone_value;
  ByteString serialized;
  bool clone_shares_buffer;

  bool operator==(const NameSnapshot&) const = default;
};

std::vector<NameSnapshot> RunNameScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  std::vector<NameSnapshot> result;
  auto value =
      pdfium::MakeRetain<CPDF_Name>(nullptr, ByteString("A B/#\0Z", 7));
  const auto append = [&result](const CPDF_Name& name) {
    NameArchiveStream archive;
    EXPECT_TRUE(name.WriteTo(&archive, nullptr));
    RetainPtr<CPDF_Object> clone = name.Clone();
    const CPDF_Name* clone_name = ToName(clone.Get());
    ASSERT_TRUE(clone_name);
    ByteString original = name.GetString();
    ByteString cloned = clone_name->GetString();
    result.push_back({
        .value = original,
        .clone_value = cloned,
        .serialized = archive.output(),
        .clone_shares_buffer =
            original.IsEmpty() || original.c_str() == cloned.c_str(),
    });
  };
  append(*value);
  value->SetString(ByteString("next\0name", 9));
  append(*value);
  return result;
}

}  // namespace

TEST(CPDFName, RustCandidateMatchesCppValueScenario) {
  EXPECT_EQ(RunNameScenario(false), RunNameScenario(true));
}
