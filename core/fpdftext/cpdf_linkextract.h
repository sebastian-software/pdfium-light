// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FPDFTEXT_CPDF_LINKEXTRACT_H_
#define CORE_FPDFTEXT_CPDF_LINKEXTRACT_H_

#include <stddef.h>
#include <stdint.h>

#include <memory>
#include <optional>
#include <vector>

#include "core/fxcrt/fx_coordinates.h"
#include "core/fxcrt/unowned_ptr.h"
#include "core/fxcrt/widestring.h"

class CPDF_TextPage;

namespace pdfium::rust {
class RustTextLinkExtract;
}

class CPDF_LinkExtract {
 public:
  struct Range {
    size_t start_;
    size_t count_;
  };

  explicit CPDF_LinkExtract(const CPDF_TextPage* pTextPage);
  ~CPDF_LinkExtract();

  void ExtractLinks();
  size_t CountLinks() const;
  WideString GetURL(size_t index) const;
  std::vector<CFX_FloatRect> GetRects(size_t index) const;
  std::optional<Range> GetTextRange(size_t index) const;

 protected:
  struct Link : public Range {
    WideString url_;
  };

  std::optional<Link> CheckWebLink(const WideString& str);
  bool CheckMailLink(WideString* str);
  static bool IsAlphanumeric(void* context, uint32_t character);

  UnownedPtr<const CPDF_TextPage> const text_page_;
  const bool use_rust_;
  std::unique_ptr<pdfium::rust::RustTextLinkExtract> rust_links_;
  std::vector<Link> link_array_;
};

#endif  // CORE_FPDFTEXT_CPDF_LINKEXTRACT_H_
