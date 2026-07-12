// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_stream.h"

#include <vector>

#include "constants/stream_dict_common.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_name.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/notreached.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class StreamArchive final : public IFX_ArchiveStream {
 public:
  bool WriteBlock(pdfium::span<const uint8_t> buffer) override {
    output_.insert(output_.end(), buffer.begin(), buffer.end());
    return true;
  }
  FX_FILESIZE CurrentOffset() const override { NOTREACHED(); }
  const DataVector<uint8_t>& output() const { return output_; }

 private:
  DataVector<uint8_t> output_;
};

struct StreamSnapshot {
  DataVector<uint8_t> raw_data;
  DataVector<uint8_t> clone_data;
  DataVector<uint8_t> serialized;
  size_t raw_size;
  int dictionary_length;
  bool is_memory_based;
  bool is_file_based;
  bool has_filter;

  bool operator==(const StreamSnapshot&) const = default;
};

std::vector<StreamSnapshot> RunStreamScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  std::vector<StreamSnapshot> result;
  const auto append = [&result](const CPDF_Stream& value) {
    StreamArchive archive;
    EXPECT_TRUE(value.WriteTo(&archive, nullptr));
    RetainPtr<CPDF_Stream> clone = ToStream(value.Clone());
    ASSERT_TRUE(clone);
    result.push_back({
        .raw_data = DataVector<uint8_t>(value.GetInMemoryRawData().begin(),
                                        value.GetInMemoryRawData().end()),
        .clone_data = DataVector<uint8_t>(clone->GetInMemoryRawData().begin(),
                                          clone->GetInMemoryRawData().end()),
        .serialized = archive.output(),
        .raw_size = value.GetRawSize(),
        .dictionary_length = value.GetDict()->GetIntegerFor("Length"),
        .is_memory_based = value.IsMemoryBased(),
        .is_file_based = value.IsFileBased(),
        .has_filter = value.HasFilter(),
    });
  };

  const uint8_t initial[] = {0, 1, 0xfe, 0xff, 0};
  auto value = pdfium::MakeRetain<CPDF_Stream>(pdfium::span(initial));
  append(*value);

  value->GetMutableDict()->SetNewFor<CPDF_Name>(pdfium::stream::kFilter,
                                                "FlateDecode");
  value->SetDataAndRemoveFilter(pdfium::span(initial).first<3>());
  append(*value);

  value->TakeData(DataVector<uint8_t>{9, 8, 7, 0, 6});
  append(*value);

  fxcrt::ostringstream stream;
  stream << "stream-data";
  value->SetDataFromStringstream(&stream);
  append(*value);
  return result;
}

}  // namespace

TEST(CPDFStream, RustCandidateMatchesCppMemoryDataScenario) {
  EXPECT_EQ(RunStreamScenario(false), RunStreamScenario(true));
}
