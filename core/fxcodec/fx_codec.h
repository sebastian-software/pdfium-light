// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXCODEC_FX_CODEC_H_
#define CORE_FXCODEC_FX_CODEC_H_

#include <stdint.h>

#include "core/fxcrt/span.h"

namespace fxcodec {


void ReverseRGB(pdfium::span<uint8_t> pDestBuf,
                pdfium::span<const uint8_t> pSrcBuf,
                int pixels);

// Can be called only after CFX_GEModule::Create().
void RegisterEncoders();

}  // namespace fxcodec


#endif  // CORE_FXCODEC_FX_CODEC_H_
