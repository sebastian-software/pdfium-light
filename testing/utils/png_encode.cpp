// Copyright 2026 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "testing/utils/png_encode.h"

#include "core/fxcrt/check.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/fx_safe_types.h"
#include "core/fxcrt/notreached.h"
#include "public/fpdfview.h"
#include "testing/image_diff/image_diff_png.h"

std::vector<uint8_t> EncodePng(pdfium::span<const uint8_t> input,
                               int width,
                               int height,
                               int stride,
                               int format) {
  std::vector<uint8_t> png;
  switch (format) {
    case FPDFBitmap_Unknown:
      break;
    case FPDFBitmap_Gray:
      png = image_diff_png::EncodeGrayPNG(input, width, height, stride);
      break;
    case FPDFBitmap_BGR:
      png = image_diff_png::EncodeBGRPNG(input, width, height, stride);
      break;
    case FPDFBitmap_BGRx:
      png = image_diff_png::EncodeBGRAPNG(input, width, height, stride,
                                          /*discard_transparency=*/true);
      break;
    case FPDFBitmap_BGRA:
      png = image_diff_png::EncodeBGRAPNG(input, width, height, stride,
                                          /*discard_transparency=*/false);
      break;
    default:
      NOTREACHED();
  }
  return png;
}

std::vector<uint8_t> EncodePng(FPDF_BITMAP bitmap) {
  const int stride = FPDFBitmap_GetStride(bitmap);
  const int width = FPDFBitmap_GetWidth(bitmap);
  const int height = FPDFBitmap_GetHeight(bitmap);
  CHECK(stride >= 0);
  CHECK(width >= 0);
  CHECK(height >= 0);
  FX_SAFE_FILESIZE size = stride;
  size *= height;
  auto input = UNSAFE_TODO(
      pdfium::span(static_cast<const uint8_t*>(FPDFBitmap_GetBuffer(bitmap)),
                   pdfium::ValueOrDieForType<size_t>(size)));

  return EncodePng(input, width, height, stride, FPDFBitmap_GetFormat(bitmap));
}
