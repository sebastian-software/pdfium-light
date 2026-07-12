// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfdoc/cpdf_numbertree.h"

#include <optional>
#include <utility>

#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"

namespace {

bool DescribeNumberTreeNode(void*,
                            uintptr_t node,
                            bool* has_limits,
                            int32_t* lower_limit,
                            int32_t* upper_limit,
                            bool* has_numbers,
                            size_t* number_count,
                            size_t* kid_count) {
  const auto* dictionary =
      reinterpret_cast<const CPDF_Dictionary*>(node);
  RetainPtr<const CPDF_Array> limits = dictionary->GetArrayFor("Limits");
  *has_limits = !!limits;
  *lower_limit = limits ? limits->GetIntegerAt(0) : 0;
  *upper_limit = limits ? limits->GetIntegerAt(1) : 0;
  RetainPtr<const CPDF_Array> numbers = dictionary->GetArrayFor("Nums");
  *has_numbers = !!numbers;
  *number_count = numbers ? numbers->size() / 2 : 0;
  RetainPtr<const CPDF_Array> kids = dictionary->GetArrayFor("Kids");
  *kid_count = kids ? kids->size() : 0;
  return true;
}

bool ReadNumberTreeNumber(void*,
                          uintptr_t node,
                          size_t index,
                          int32_t* key,
                          uintptr_t* value) {
  const auto* dictionary =
      reinterpret_cast<const CPDF_Dictionary*>(node);
  RetainPtr<const CPDF_Array> numbers = dictionary->GetArrayFor("Nums");
  if (!numbers || index >= numbers->size() / 2) {
    return false;
  }
  *key = numbers->GetIntegerAt(index * 2);
  *value = reinterpret_cast<uintptr_t>(
      numbers->GetDirectObjectAt(index * 2 + 1).Get());
  return true;
}

bool ReadNumberTreeKid(void*,
                       uintptr_t node,
                       size_t index,
                       uintptr_t* kid) {
  const auto* dictionary =
      reinterpret_cast<const CPDF_Dictionary*>(node);
  RetainPtr<const CPDF_Array> kids = dictionary->GetArrayFor("Kids");
  if (!kids || index >= kids->size()) {
    return false;
  }
  *kid = reinterpret_cast<uintptr_t>(kids->GetDictAt(index).Get());
  return true;
}

RetainPtr<const CPDF_Object> FindNumberNode(const CPDF_Dictionary* node_dict,
                                            int num) {
  RetainPtr<const CPDF_Array> limits_array = node_dict->GetArrayFor("Limits");
  if (limits_array && (num < limits_array->GetIntegerAt(0) ||
                       num > limits_array->GetIntegerAt(1))) {
    return nullptr;
  }
  RetainPtr<const CPDF_Array> numbers_array = node_dict->GetArrayFor("Nums");
  if (numbers_array) {
    for (size_t i = 0; i < numbers_array->size() / 2; i++) {
      int index = numbers_array->GetIntegerAt(i * 2);
      if (num == index) {
        return numbers_array->GetDirectObjectAt(i * 2 + 1);
      }
      if (index > num) {
        break;
      }
    }
    return nullptr;
  }

  RetainPtr<const CPDF_Array> kids_array = node_dict->GetArrayFor("Kids");
  if (!kids_array) {
    return nullptr;
  }

  for (size_t i = 0; i < kids_array->size(); i++) {
    RetainPtr<const CPDF_Dictionary> kid_dict = kids_array->GetDictAt(i);
    if (!kid_dict) {
      continue;
    }

    RetainPtr<const CPDF_Object> result = FindNumberNode(kid_dict.Get(), num);
    if (result) {
      return result;
    }
  }
  return nullptr;
}

std::optional<CPDF_NumberTree::KeyValue> FindLowerBound(
    const CPDF_Dictionary* node_dict,
    int num) {
  RetainPtr<const CPDF_Array> limits_array = node_dict->GetArrayFor("Limits");
  if (limits_array) {
    if (num < limits_array->GetIntegerAt(0)) {
      return std::nullopt;
    }
    const int max_value = limits_array->GetIntegerAt(1);
    if (num >= max_value) {
      return CPDF_NumberTree::KeyValue(max_value,
                                       FindNumberNode(node_dict, max_value));
    }
  }

  RetainPtr<const CPDF_Array> numbers_array = node_dict->GetArrayFor("Nums");
  if (numbers_array) {
    for (size_t i = numbers_array->size() / 2; i > 0; --i) {
      const size_t key_index = (i - 1) * 2;
      const int key = numbers_array->GetIntegerAt(key_index);
      if (num >= key) {
        const size_t value_index = key_index + 1;
        return CPDF_NumberTree::KeyValue(
            key, numbers_array->GetDirectObjectAt(value_index));
      }
    }
    return std::nullopt;
  }

  RetainPtr<const CPDF_Array> kids_array = node_dict->GetArrayFor("Kids");
  if (!kids_array) {
    return std::nullopt;
  }

  for (size_t i = kids_array->size(); i > 0; --i) {
    RetainPtr<const CPDF_Dictionary> kid_dict = kids_array->GetDictAt(i - 1);
    if (!kid_dict) {
      continue;
    }

    std::optional<CPDF_NumberTree::KeyValue> result =
        FindLowerBound(kid_dict.Get(), num);
    if (result.has_value()) {
      return result;
    }
  }
  return std::nullopt;
}

}  // namespace

CPDF_NumberTree::CPDF_NumberTree(RetainPtr<const CPDF_Dictionary> root)
    : root_(std::move(root)) {}

CPDF_NumberTree::~CPDF_NumberTree() = default;

RetainPtr<const CPDF_Object> CPDF_NumberTree::LookupValue(int num) const {
  if (pdfium::rust::UseRustParserCandidate()) {
    std::optional<uintptr_t> result = pdfium::rust::RustNumberTreeLookup(
        reinterpret_cast<uintptr_t>(root_.Get()), num, nullptr,
        DescribeNumberTreeNode, ReadNumberTreeNumber, ReadNumberTreeKid);
    if (result.has_value()) {
      return pdfium::WrapRetain(
          reinterpret_cast<const CPDF_Object*>(result.value()));
    }
  }
  return FindNumberNode(root_.Get(), num);
}

std::optional<CPDF_NumberTree::KeyValue> CPDF_NumberTree::GetLowerBound(
    int num) const {
  if (pdfium::rust::UseRustParserCandidate()) {
    std::optional<pdfium::rust::RustNumberTreeLowerBoundResult> result =
        pdfium::rust::RustNumberTreeLowerBound(
            reinterpret_cast<uintptr_t>(root_.Get()), num, nullptr,
            DescribeNumberTreeNode, ReadNumberTreeNumber, ReadNumberTreeKid);
    if (result.has_value()) {
      if (!result->found) {
        return std::nullopt;
      }
      return KeyValue(
          result->key,
          pdfium::WrapRetain(
              reinterpret_cast<const CPDF_Object*>(result->value)));
    }
  }
  return FindLowerBound(root_.Get(), num);
}

CPDF_NumberTree::KeyValue::KeyValue(int key, RetainPtr<const CPDF_Object> value)
    : key(key), value(std::move(value)) {}

CPDF_NumberTree::KeyValue::KeyValue(CPDF_NumberTree::KeyValue&&) noexcept =
    default;

CPDF_NumberTree::KeyValue& CPDF_NumberTree::KeyValue::operator=(
    CPDF_NumberTree::KeyValue&&) noexcept = default;

CPDF_NumberTree::KeyValue::~KeyValue() = default;
