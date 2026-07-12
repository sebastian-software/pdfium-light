// Copyright 2022 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_dictionary.h"

#include <utility>
#include <vector>

#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_number.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "testing/gtest/include/gtest/gtest.h"

TEST(DictionaryTest, Iterators) {
  auto dict = pdfium::MakeRetain<CPDF_Dictionary>();
  dict->SetNewFor<CPDF_Dictionary>("the-dictionary");
  dict->SetNewFor<CPDF_Array>("the-array");
  dict->SetNewFor<CPDF_Number>("the-number", 42);

  CPDF_DictionaryLocker locked_dict(dict);
  auto it = locked_dict.begin();
  EXPECT_NE(it, locked_dict.end());
  EXPECT_EQ(it->first, ByteString("the-array"));
  EXPECT_TRUE(it->second->IsArray());

  ++it;
  EXPECT_NE(it, locked_dict.end());
  EXPECT_EQ(it->first, ByteString("the-dictionary"));
  EXPECT_TRUE(it->second->IsDictionary());

  ++it;
  EXPECT_NE(it, locked_dict.end());
  EXPECT_EQ(it->first, ByteString("the-number"));
  EXPECT_TRUE(it->second->IsNumber());

  ++it;
  EXPECT_EQ(it, locked_dict.end());
}

namespace {

struct DictionaryOwnershipSnapshot {
  std::vector<ByteString> keys;
  std::vector<int> values;
  bool a_and_z_are_same;

  bool operator==(const DictionaryOwnershipSnapshot&) const = default;
};

std::vector<DictionaryOwnershipSnapshot> RunDictionaryOwnershipScenario(
    bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  std::vector<DictionaryOwnershipSnapshot> result;
  const auto append = [&result](const CPDF_Dictionary& dictionary) {
    DictionaryOwnershipSnapshot snapshot;
    CPDF_DictionaryLocker locker(&dictionary);
    for (const auto& [key, object] : locker) {
      snapshot.keys.push_back(key);
      snapshot.values.push_back(object->GetInteger());
    }
    snapshot.a_and_z_are_same =
        dictionary.GetObjectFor("a") == dictionary.GetObjectFor("z");
    result.push_back(std::move(snapshot));
  };

  auto dictionary = pdfium::MakeRetain<CPDF_Dictionary>();
  RetainPtr<CPDF_Number> shared = pdfium::MakeRetain<CPDF_Number>(10);
  dictionary->SetFor("z", shared);
  dictionary->SetFor("a", shared);
  dictionary->SetNewFor<CPDF_Number>(ByteString("a\0b", 3), 15);
  append(*dictionary);

  dictionary->SetNewFor<CPDF_Number>("a", 20);
  dictionary->SetNewFor<CPDF_Number>("m", 30);
  dictionary->ReplaceKey("z", "m");
  append(*dictionary);

  RetainPtr<CPDF_Object> removed =
      dictionary->RemoveFor(ByteString("a\0b", 3).AsStringView());
  EXPECT_EQ(15, removed->GetInteger());
  dictionary->SetFor("missing", RetainPtr<CPDF_Object>());
  dictionary->SetFor("a", RetainPtr<CPDF_Object>());
  append(*dictionary);

  RetainPtr<CPDF_Dictionary> clone = ToDictionary(dictionary->Clone());
  append(*clone);
  return result;
}

}  // namespace

TEST(DictionaryTest, RustCandidateMatchesCppOwnershipScenario) {
  EXPECT_EQ(RunDictionaryOwnershipScenario(false),
            RunDictionaryOwnershipScenario(true));
}
