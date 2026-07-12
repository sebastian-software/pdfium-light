// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_boolean.h"

#include <vector>

#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/notreached.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class BooleanArchiveStream final : public IFX_ArchiveStream {
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

struct BooleanSnapshot {
  int integer;
  ByteString string;
  ByteString clone_string;
  ByteString serialized;

  bool operator==(const BooleanSnapshot&) const = default;
};

std::vector<BooleanSnapshot> RunBooleanScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  std::vector<BooleanSnapshot> result;
  const auto append = [&result](const CPDF_Boolean& value) {
    BooleanArchiveStream archive;
    EXPECT_TRUE(value.WriteTo(&archive, nullptr));
    result.push_back({
        .integer = value.GetInteger(),
        .string = value.GetString(),
        .clone_string = value.Clone()->GetString(),
        .serialized = archive.output(),
    });
  };

  auto value = pdfium::MakeRetain<CPDF_Boolean>();
  append(*value);
  for (const ByteString& input :
       {ByteString("true"), ByteString("false"), ByteString("True"),
        ByteString("true\0", 5)}) {
    value->SetString(input);
    append(*value);
  }
  return result;
}

}  // namespace

TEST(CPDFBoolean, RustCandidateMatchesCppValueScenario) {
  EXPECT_EQ(RunBooleanScenario(false), RunBooleanScenario(true));
}
