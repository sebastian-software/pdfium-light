// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "core/fpdfapi/parser/cpdf_indirect_object_holder.h"

#include <tuple>
#include <vector>

#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_null.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fxcrt/check.h"
#include "testing/gmock/include/gmock/gmock.h"
#include "testing/gtest/include/gtest/gtest.h"

namespace {

class MockIndirectObjectHolder final : public CPDF_IndirectObjectHolder {
 public:
  MockIndirectObjectHolder() = default;
  ~MockIndirectObjectHolder() override = default;

  MOCK_METHOD(RetainPtr<CPDF_Object>, ParseIndirectObject, (uint32_t objnum));
};

struct ObjectIndexScenario {
  uint32_t last_object_number;
  bool rejected_lower_generation;
  bool accepted_higher_generation;
  bool deleted_object_is_absent;
  std::vector<std::tuple<uint32_t, CPDF_Object::Type, uint32_t>> objects;

  bool operator==(const ObjectIndexScenario&) const = default;
};

ObjectIndexScenario RunObjectIndexScenario(bool use_rust_candidate) {
  pdfium::rust::ScopedRustParserImplementationForTesting implementation(
      use_rust_candidate);
  CPDF_IndirectObjectHolder holder;
  holder.SetLastObjNum(3);
  auto added = pdfium::MakeRetain<CPDF_Null>();
  const uint32_t added_number = holder.AddIndirectObject(added);
  CHECK_EQ(4u, added_number);

  auto generation_two = pdfium::MakeRetain<CPDF_Null>();
  generation_two->SetGenNum(2);
  CHECK(holder.ReplaceIndirectObjectIfHigherGeneration(
      10, std::move(generation_two)));
  auto generation_one = pdfium::MakeRetain<CPDF_Null>();
  generation_one->SetGenNum(1);
  const bool rejected_lower = !holder.ReplaceIndirectObjectIfHigherGeneration(
      10, std::move(generation_one));
  auto generation_three = pdfium::MakeRetain<CPDF_Null>();
  generation_three->SetGenNum(3);
  const bool accepted_higher = holder.ReplaceIndirectObjectIfHigherGeneration(
      10, std::move(generation_three));

  holder.DeleteIndirectObject(added_number);
  ObjectIndexScenario result = {
      .last_object_number = holder.GetLastObjNum(),
      .rejected_lower_generation = rejected_lower,
      .accepted_higher_generation = accepted_higher,
      .deleted_object_is_absent = !holder.GetIndirectObject(added_number),
  };
  for (const auto& [object_number, object] : holder) {
    CHECK(object);
    result.objects.emplace_back(object_number, object->GetType(),
                                object->GetGenNum());
  }
  return result;
}

}  // namespace

TEST(IndirectObjectHolderTest, RustCandidateMatchesCppObjectIndexScenario) {
  EXPECT_EQ(RunObjectIndexScenario(false), RunObjectIndexScenario(true));
}

TEST(IndirectObjectHolderTest, RecursiveParseOfSameObject) {
  MockIndirectObjectHolder mock_holder;
  // ParseIndirectObject should not be called again on recursively same object
  // parse request.
  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_))
      .WillOnce(::testing::WithArg<0>(
          [&mock_holder](uint32_t objnum) -> RetainPtr<CPDF_Object> {
            RetainPtr<const CPDF_Object> same_parse =
                mock_holder.GetOrParseIndirectObject(objnum);
            CHECK(!same_parse);
            return pdfium::MakeRetain<CPDF_Null>();
          }));

  EXPECT_TRUE(mock_holder.GetOrParseIndirectObject(1000));
}

TEST(IndirectObjectHolderTest, GetObjectMethods) {
  static constexpr uint32_t kObjNum = 1000;
  MockIndirectObjectHolder mock_holder;

  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_)).Times(0);
  EXPECT_FALSE(mock_holder.GetIndirectObject(kObjNum));
  ::testing::Mock::VerifyAndClearExpectations(&mock_holder);

  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_))
      .WillOnce(
          ::testing::WithArg<0>([](uint32_t objnum) -> RetainPtr<CPDF_Object> {
            return pdfium::MakeRetain<CPDF_Null>();
          }));
  EXPECT_TRUE(mock_holder.GetOrParseIndirectObject(kObjNum));
  ::testing::Mock::VerifyAndClearExpectations(&mock_holder);

  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_)).Times(0);
  ASSERT_TRUE(mock_holder.GetIndirectObject(kObjNum));
  ::testing::Mock::VerifyAndClearExpectations(&mock_holder);

  EXPECT_EQ(kObjNum, mock_holder.GetIndirectObject(kObjNum)->GetObjNum());
}

TEST(IndirectObjectHolderTest, ParseInvalidObjNum) {
  MockIndirectObjectHolder mock_holder;

  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_)).Times(0);
  EXPECT_FALSE(
      mock_holder.GetOrParseIndirectObject(CPDF_Object::kInvalidObjNum));
}

TEST(IndirectObjectHolderTest, ReplaceObjectWithInvalidObjNum) {
  MockIndirectObjectHolder mock_holder;

  EXPECT_CALL(mock_holder, ParseIndirectObject(::testing::_)).Times(0);
  EXPECT_FALSE(mock_holder.ReplaceIndirectObjectIfHigherGeneration(
      CPDF_Object::kInvalidObjNum, pdfium::MakeRetain<CPDF_Null>()));
}

TEST(IndirectObjectHolderTest, TemplateNewMethods) {
  MockIndirectObjectHolder mock_holder;

  auto dict = mock_holder.NewIndirect<CPDF_Dictionary>();
  auto pArray = mock_holder.NewIndirect<CPDF_Array>();
  mock_holder.DeleteIndirectObject(dict->GetObjNum());
  mock_holder.DeleteIndirectObject(pArray->GetObjNum());

  // No longer UAF since NewIndirect<> returns retained objects.
  EXPECT_TRUE(dict->IsDictionary());
  EXPECT_TRUE(pArray->IsArray());
}
