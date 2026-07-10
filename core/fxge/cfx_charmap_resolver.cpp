// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fxge/cfx_charmap_resolver.h"

#include "core/fxcrt/fx_codepage.h"
#include "core/fxge/cfx_face.h"
#include "core/fxge/cfx_font.h"
#include "core/fxge/cfx_substfont.h"
#include "core/fxge/fx_font.h"
#include "core/fxge/fx_fontencoding.h"

namespace {

class UnicodeCharmapResolver : public CFX_CharmapResolver {
 public:
  explicit UnicodeCharmapResolver(const CFX_Font* font)
      : CFX_CharmapResolver(font) {}

  ~UnicodeCharmapResolver() override = default;

  uint32_t GlyphFromCharCode(uint32_t charcode) override {
    RetainPtr<CFX_Face> face = font_->GetFace();
    if (!face) {
      return charcode;
    }
    if (face->SelectCharMap(fxge::FontEncoding::kUnicode)) {
      return face->GetCharIndex(charcode);
    }
    if (font_->GetSubstFont() && font_->GetSubstFont()->IsSymbolic()) {
      uint32_t index = 0;
      if (face->SelectCharMap(fxge::FontEncoding::kSymbol)) {
        index = face->GetCharIndex(charcode);
      }
      if (!index && face->SelectCharMap(fxge::FontEncoding::kAppleRoman)) {
        return face->GetCharIndex(charcode);
      }
    }
    return charcode;
  }
};


}  // namespace

CFX_CharmapResolver::CFX_CharmapResolver(const CFX_Font* font) : font_(font) {}

CFX_CharmapResolver::~CFX_CharmapResolver() = default;

// static
std::unique_ptr<CFX_CharmapResolver> CFX_CharmapResolver::CreateUnicode(
    const CFX_Font* font) {
  return std::make_unique<UnicodeCharmapResolver>(font);
}
