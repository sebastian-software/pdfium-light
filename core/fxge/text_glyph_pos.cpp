// Copyright 2019 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fxge/text_glyph_pos.h"

#include "core/fxcrt/fx_safe_types.h"
#include "core/fxge/cfx_glyphbitmap.h"
#include "core/fxge/freetype/rust/rust_glyph_adapter.h"

namespace {

std::optional<CFX_Point> GetGlyphOriginCppReference(
    const CFX_Point& origin,
    const CFX_GlyphBitmap& glyph,
    const CFX_Point& offset) {
  FX_SAFE_INT32 left = origin.x;
  left += glyph.left();
  left -= offset.x;
  if (!left.IsValid()) {
    return std::nullopt;
  }

  FX_SAFE_INT32 top = origin.y;
  top -= glyph.top();
  top -= offset.y;
  if (!top.IsValid()) {
    return std::nullopt;
  }

  return CFX_Point(left.ValueOrDie(), top.ValueOrDie());
}

}  // namespace

TextGlyphPos::TextGlyphPos() = default;

TextGlyphPos::TextGlyphPos(const TextGlyphPos&) = default;

TextGlyphPos::~TextGlyphPos() = default;

std::optional<CFX_Point> TextGlyphPos::GetOrigin(
    const CFX_Point& offset) const {
  if (fxge::UseRustGlyphCandidate()) {
    const auto plan =
        fxge::RustPlanGlyphOrigin(origin_.x, origin_.y, glyph_->left(),
                                  glyph_->top(), offset.x, offset.y);
    if (plan.has_value()) {
      if (!plan->valid) {
        fxge::RecordGlyphOriginForTesting(false, 0, 0);
        return std::nullopt;
      }
      fxge::RecordGlyphOriginForTesting(true, plan->x, plan->y);
      return CFX_Point(plan->x, plan->y);
    }
  }
  const auto result = GetGlyphOriginCppReference(origin_, *glyph_, offset);
  fxge::RecordGlyphOriginForTesting(result.has_value(),
                                    result.has_value() ? result->x : 0,
                                    result.has_value() ? result->y : 0);
  return result;
}
