// Copyright 2026 Sebastian Werner
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
#define CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_

#include <stddef.h>
#include <stdint.h>

#include <optional>
#include <utility>

#include "core/fxcrt/span.h"
#include "core/fxcrt/widestring.h"

namespace pdfium::rust {

enum class RustTextOrientation : uint8_t {
  kUnknown = 0,
  kHorizontal = 1,
  kVertical = 2,
};

using RustTextOrientationObjectCallback = bool (*)(void* context,
                                                   size_t index,
                                                   bool* active,
                                                   bool* is_text,
                                                   float* left,
                                                   float* bottom,
                                                   float* right,
                                                   float* top);

std::optional<RustTextOrientation> RustTextFlowOrientation(
    int32_t page_width,
    int32_t page_height,
    size_t object_count,
    void* context,
    RustTextOrientationObjectCallback get_object);

std::optional<RustTextOrientation> RustTextObjectWritingMode(
    size_t character_count,
    RustTextOrientation fallback_orientation,
    float first_x,
    float first_y,
    float last_x,
    float last_y);

using RustTextRectCallback = bool (*)(void* context,
                                      size_t index,
                                      float* left,
                                      float* bottom,
                                      float* right,
                                      float* top);

std::optional<int> RustTextIndexAtPosition(
    size_t character_count,
    float point_x,
    float point_y,
    float tolerance_width,
    float tolerance_height,
    void* context,
    RustTextRectCallback get_rect);

class RustTextIndexMap final {
 public:
  explicit RustTextIndexMap(pdfium::span<const uint8_t> included);
  RustTextIndexMap(const RustTextIndexMap&) = delete;
  RustTextIndexMap& operator=(const RustTextIndexMap&) = delete;
  ~RustTextIndexMap();

  bool valid() const { return state_ != nullptr; }
  int CharacterFromText(int text_index) const;
  int TextFromCharacter(int character_index) const;
  std::optional<std::pair<int, int>> PageTextRange(int start,
                                                   int count,
                                                   int character_count) const;

 private:
  void* state_;
};

using RustTextIndexMapCallback = bool (*)(void* context,
                                          int32_t input,
                                          int32_t* output);

class RustTextPageFind final {
 public:
  RustTextPageFind(WideStringView page_text,
                   WideStringView query,
                   bool match_whole_word,
                   bool consecutive,
                   std::optional<size_t> start,
                   void* context,
                   RustTextIndexMapCallback text_to_character,
                   RustTextIndexMapCallback character_to_text);
  RustTextPageFind(const RustTextPageFind&) = delete;
  RustTextPageFind& operator=(const RustTextPageFind&) = delete;
  ~RustTextPageFind();

  bool valid() const { return state_ != nullptr; }
  bool FindFirst() const;
  std::optional<bool> FindNext();
  std::optional<bool> FindPrevious();
  std::optional<std::pair<size_t, size_t>> GetResult() const;

 private:
  void* state_;
  void* context_;
  RustTextIndexMapCallback text_to_character_;
  RustTextIndexMapCallback character_to_text_;
};

using RustTextIsAlphanumericCallback = bool (*)(void* context,
                                                uint32_t character);

struct RustTextLinkRange {
  size_t start;
  size_t count;
};

using RustTextSelectionRectCallback = bool (*)(void* context,
                                               size_t index,
                                               bool* generated,
                                               uintptr_t* text_object,
                                               float* left,
                                               float* bottom,
                                               float* right,
                                               float* top);

struct RustTextRect {
  float left;
  float bottom;
  float right;
  float top;
};

std::optional<bool> RustTextObjectsEndLine(
    RustTextOrientation writing_mode,
    const RustTextRect& this_rect,
    const RustTextRect& previous_rect,
    const RustTextRect& current_line_rect,
    float this_font_size,
    float previous_font_size);

std::optional<float> RustTextNormalizeThreshold(float threshold,
                                                int32_t first,
                                                int32_t second,
                                                int32_t third);

std::optional<bool> RustTextShouldGenerateSpace(float position_x,
                                                float last_position,
                                                float this_width,
                                                float last_width,
                                                float threshold);

using RustTextCharacterPredicateCallback = bool (*)(void* context,
                                                     uint32_t character,
                                                     uint8_t predicate);

std::optional<bool> RustTextIsHyphenJoin(
    WideStringView current_text,
    uint32_t current_character,
    bool has_previous_character,
    uint8_t previous_char_type,
    uint32_t previous_unicode,
    void* context,
    RustTextCharacterPredicateCallback character_predicate);

class RustTextSelectionRects final {
 public:
  RustTextSelectionRects(size_t character_count,
                         int start,
                         int count,
                         void* context,
                         RustTextSelectionRectCallback get_character);
  RustTextSelectionRects(const RustTextSelectionRects&) = delete;
  RustTextSelectionRects& operator=(const RustTextSelectionRects&) = delete;
  ~RustTextSelectionRects();

  bool valid() const { return state_ != nullptr; }
  size_t size() const;
  std::optional<RustTextRect> GetRect(size_t index) const;

 private:
  void* state_;
};

using RustTextPredicateCharacterCallback = bool (*)(void* context,
                                                    size_t index,
                                                    bool* included,
                                                    uint32_t* unicode,
                                                    float* origin_y);

class RustTextPredicateResult final {
 public:
  RustTextPredicateResult(size_t character_count,
                          void* context,
                          RustTextPredicateCharacterCallback get_character);
  RustTextPredicateResult(const RustTextPredicateResult&) = delete;
  RustTextPredicateResult& operator=(const RustTextPredicateResult&) = delete;
  ~RustTextPredicateResult();

  bool valid() const { return state_ != nullptr; }
  std::optional<WideString> GetText() const;

 private:
  void* state_;
};

enum class RustTextDirection : uint8_t {
  kNeutral = 0,
  kLeft = 1,
  kRight = 2,
  kLeftWeak = 3,
};

struct RustTextBidiSegment {
  size_t start;
  size_t count;
  RustTextDirection direction;
};

struct RustTextEmission {
  size_t character_index;
  bool is_rtl;
};

class RustTextLinePlan final {
 public:
  explicit RustTextLinePlan(WideStringView text);
  RustTextLinePlan(const RustTextLinePlan&) = delete;
  RustTextLinePlan& operator=(const RustTextLinePlan&) = delete;
  ~RustTextLinePlan();

  bool valid() const { return state_ != nullptr; }
  size_t kept_count() const;
  std::optional<size_t> GetKeptIndex(size_t index) const;
  bool SetSegments(RustTextDirection overall_direction,
                   pdfium::span<const RustTextBidiSegment> segments);
  size_t emission_count() const;
  std::optional<RustTextEmission> GetEmission(size_t index) const;

 private:
  void* state_;
};

struct RustTextCharacterEmission {
  uint32_t unicode;
  bool append_text;
  bool set_unicode;
  uint8_t char_type;
};

class RustTextAddCharacterPlan final {
 public:
  RustTextAddCharacterPlan(uint8_t char_type,
                           uint32_t char_code,
                           uint32_t info_unicode,
                           uint32_t display_unicode,
                           bool is_rtl);
  RustTextAddCharacterPlan(const RustTextAddCharacterPlan&) = delete;
  RustTextAddCharacterPlan& operator=(const RustTextAddCharacterPlan&) = delete;
  ~RustTextAddCharacterPlan();

  bool valid() const { return state_ != nullptr; }
  bool needs_display_unicode() const;
  bool SetDisplayUnicode(uint32_t display_unicode);
  bool needs_normalization() const;
  bool SetNormalization(pdfium::span<const wchar_t> normalized);
  size_t emission_count() const;
  std::optional<RustTextCharacterEmission> GetEmission(size_t index) const;

 private:
  void* state_;
};

class RustTextLinkExtract final {
 public:
  RustTextLinkExtract();
  RustTextLinkExtract(const RustTextLinkExtract&) = delete;
  RustTextLinkExtract& operator=(const RustTextLinkExtract&) = delete;
  ~RustTextLinkExtract();

  bool Extract(WideStringView page_text,
               pdfium::span<const uint32_t> characters,
               pdfium::span<const uint8_t> flags,
               void* context,
               RustTextIsAlphanumericCallback is_alphanumeric);
  size_t size() const;
  std::optional<RustTextLinkRange> GetRange(size_t index) const;
  std::optional<WideString> GetUrl(size_t index) const;

 private:
  void* state_;
};

}  // namespace pdfium::rust

#endif  // CORE_FPDFTEXT_RUST_RUST_TEXT_ADAPTER_H_
