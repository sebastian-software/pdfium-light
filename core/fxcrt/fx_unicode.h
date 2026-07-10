// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXCRT_FX_UNICODE_H_
#define CORE_FXCRT_FX_UNICODE_H_

#include <stdint.h>

// NOTE: Order matters, less-than/greater-than comparisons are used.
enum class FX_BIDICLASS : uint8_t {
  kON = 0,    // Other Neutral
  kL = 1,     // Left Letter
  kR = 2,     // Right Letter
  kAN = 3,    // Arabic Number
  kEN = 4,    // European Number
  kAL = 5,    // Arabic Letter
  kNSM = 6,   // Non-spacing Mark
  kCS = 7,    // Common Number Separator
  kES = 8,    // European Separator
  kET = 9,    // European Number Terminator
  kBN = 10,   // Boundary Neutral
  kS = 11,    // Segment Separator
  kWS = 12,   // Whitespace
  kB = 13,    // Paragraph Separator
  kRLO = 14,  // Right-to-Left Override
  kRLE = 15,  // Right-to-Left Embedding
  kLRO = 16,  // Left-to-Right Override
  kLRE = 17,  // Left-to-Right Embedding
  kPDF = 18,  // Pop Directional Format
  kN = kON,
};


namespace pdfium {
namespace unicode {

constexpr wchar_t kRightSingleQuotationMark = 0x2019;
constexpr wchar_t kLineSeparator = 0x2028;
constexpr wchar_t kParagraphSeparator = 0x2029;
constexpr wchar_t kBoxDrawingsLightVerical = 0x2502;
constexpr wchar_t kZeroWidthNoBreakSpace = 0xfeff;

wchar_t GetMirrorChar(wchar_t wch);
FX_BIDICLASS GetBidiClass(wchar_t wch);


}  // namespace unicode
}  // namespace pdfium

#endif  // CORE_FXCRT_FX_UNICODE_H_
