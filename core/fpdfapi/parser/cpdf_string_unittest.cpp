// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_string.h"

#include <vector>

#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/notreached.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class StringArchiveStream final : public IFX_ArchiveStream {
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

struct StringSnapshot {
  ByteString value;
  ByteString encoded;
  ByteString clone_value;
  ByteString serialized;
  bool is_hex;
  bool clone_shares_buffer;

  bool operator==(const StringSnapshot&) const = default;
};

std::vector<StringSnapshot> RunStringScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  std::vector<StringSnapshot> result;
  const auto append = [&result](const CPDF_String& value) {
    StringArchiveStream archive;
    EXPECT_TRUE(value.WriteTo(&archive, nullptr));
    RetainPtr<CPDF_Object> clone = value.Clone();
    const CPDF_String* clone_string = ToString(clone.Get());
    ASSERT_TRUE(clone_string);
    ByteString original_value = value.GetString();
    ByteString clone_value = clone_string->GetString();
    result.push_back({
        .value = original_value,
        .encoded = value.EncodeString(),
        .clone_value = clone_value,
        .serialized = archive.output(),
        .is_hex = value.IsHex(),
        .clone_shares_buffer = original_value.IsEmpty() ||
                               original_value.c_str() == clone_value.c_str(),
    });
  };

  auto regular =
      pdfium::MakeRetain<CPDF_String>(nullptr, ByteString("a\0b", 3));
  append(*regular);
  regular->SetString("(next)\\");
  append(*regular);

  const uint8_t binary[] = {0, 1, 0xfe, 0xff};
  auto hex = pdfium::MakeRetain<CPDF_String>(nullptr, pdfium::span(binary),
                                             CPDF_String::DataType::kIsHex);
  append(*hex);
  hex->SetString(ByteString("x\0y", 3));
  append(*hex);
  return result;
}

}  // namespace

TEST(CPDFString, RustCandidateMatchesCppValueScenario) {
  EXPECT_EQ(RunStringScenario(false), RunStringScenario(true));
}
