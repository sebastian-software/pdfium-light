// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef CORE_FXCODEC_FX_CODEC_DEF_H_
#define CORE_FXCODEC_FX_CODEC_DEF_H_

enum class FXCODEC_STATUS {
  kError = -1,
  kFrameReady,
  kFrameToBeContinued,
  kDecodeReady,
  kDecodeToBeContinued,
  kDecodeFinished,
};


#endif  // CORE_FXCODEC_FX_CODEC_DEF_H_
