// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXGE_CFX_FONTMGR_H_
#define CORE_FXGE_CFX_FONTMGR_H_

#include <stddef.h>
#include <stdint.h>

#include <map>
#include <memory>

#include "core/fxcrt/observed_ptr.h"
#include "core/fxcrt/retain_ptr.h"
#include "core/fxcrt/span.h"
#include "core/fxge/freetype/fx_freetype.h"


class CFX_Face;
class CFX_Font;
class CFX_FontMapper;
class CFX_GlyphCache;


class CFX_FontMgr {
 public:
  CFX_FontMgr();
  ~CFX_FontMgr();

  RetainPtr<CFX_GlyphCache> GetGlyphCache(const CFX_Font* font);

  // Always present.
  CFX_FontMapper* GetBuiltinMapper() const { return builtin_mapper_.get(); }

  FXFT_LibraryRec* GetFTLibrary() const { return ft_library_.get(); }

  bool FTLibrarySupportsHinting() const { return ft_library_supports_hinting_; }


 private:
  // Must come before `builtin_mapper_`.
  ScopedFXFTLibraryRec const ft_library_;
  std::unique_ptr<CFX_FontMapper> builtin_mapper_;
  std::map<CFX_Face*, ObservedPtr<CFX_GlyphCache>> glyph_cache_map_;
  const bool ft_library_supports_hinting_;
};


#endif  // CORE_FXGE_CFX_FONTMGR_H_
