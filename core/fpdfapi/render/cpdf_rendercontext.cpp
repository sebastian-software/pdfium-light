// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "core/fpdfapi/render/cpdf_rendercontext.h"

#include <memory>
#include <optional>
#include <utility>

#include "build/build_config.h"
#include "core/fpdfapi/page/cpdf_pageimagecache.h"
#include "core/fpdfapi/page/cpdf_pageobject.h"
#include "core/fpdfapi/page/cpdf_pageobjectholder.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_document.h"
#include "core/fpdfapi/render/cpdf_renderoptions.h"
#include "core/fpdfapi/render/cpdf_renderstatus.h"
#include "core/fpdfapi/render/cpdf_textrenderer.h"
#include "core/fpdfapi/render/rust/rust_render_adapter.h"
#include "core/fxcrt/check.h"
#include "core/fxge/cfx_renderdevice.h"
#include "core/fxge/dib/cfx_dibitmap.h"
#include "core/fxge/dib/fx_dib.h"

namespace {

pdfium::rust::RenderLayerPlan BuildCppRenderLayerPlan(bool has_options,
                                                      bool has_last_matrix) {
  uint8_t bits = 0;
  if (has_options) {
    bits |= static_cast<uint8_t>(pdfium::rust::RenderLayerPlanBit::kSetOptions);
  }
  if (has_last_matrix) {
    bits |= static_cast<uint8_t>(
        pdfium::rust::RenderLayerPlanBit::kApplyLastMatrix);
  }
  return pdfium::rust::RenderLayerPlan(bits);
}

pdfium::rust::RenderLayerCompletion BuildCppRenderLayerCompletion(
    bool limited_image_cache,
    bool stopped) {
  uint8_t bits = 0;
  if (limited_image_cache) {
    bits |= static_cast<uint8_t>(
        pdfium::rust::RenderLayerCompletionBit::kOptimizeCache);
  }
  if (stopped) {
    bits |= static_cast<uint8_t>(pdfium::rust::RenderLayerCompletionBit::kStop);
  }
  return pdfium::rust::RenderLayerCompletion(bits);
}

}  // namespace

CPDF_RenderContext::CPDF_RenderContext(
    CPDF_Document* doc,
    RetainPtr<CPDF_Dictionary> pPageResources,
    CPDF_PageImageCache* pPageCache)
    : document_(doc),
      page_resources_(std::move(pPageResources)),
      page_cache_(pPageCache) {}

CPDF_RenderContext::~CPDF_RenderContext() = default;

void CPDF_RenderContext::GetBackgroundToDevice(
    CFX_RenderDevice* device,
    const CPDF_PageObject* object,
    const CPDF_RenderOptions* options,
    const CFX_Matrix& matrix) {
  device->FillRect(FX_RECT(0, 0, device->GetWidth(), device->GetHeight()),
                   0xffffffff);
  Render(device, object, options, &matrix);
}

#if BUILDFLAG(IS_WIN)
void CPDF_RenderContext::GetBackgroundToBitmap(RetainPtr<CFX_DIBitmap> bitmap,
                                               const CPDF_PageObject* object,
                                               const CFX_Matrix& matrix) {
  std::unique_ptr<CFX_RenderDevice> device =
      CFX_RenderDevice::CreateForBitmap(std::move(bitmap));
  if (!device) {
    return;
  }
  GetBackgroundToDevice(device.get(), object, /*options=*/nullptr, matrix);
}
#endif

void CPDF_RenderContext::AppendLayer(CPDF_PageObjectHolder* pObjectHolder,
                                     const CFX_Matrix& mtObject2Device) {
  layers_.emplace_back(pObjectHolder, mtObject2Device);
}

void CPDF_RenderContext::Render(CFX_RenderDevice* pDevice,
                                const CPDF_PageObject* pStopObj,
                                const CPDF_RenderOptions* pOptions,
                                const CFX_Matrix* pLastMatrix) {
  for (auto& layer : layers_) {
    CFX_RenderDevice::StateRestorer restorer(pDevice);
    CPDF_RenderStatus status(this, pDevice);
    const bool has_options = pOptions != nullptr;
    const bool has_last_matrix = pLastMatrix != nullptr;
    const auto rust_plan = pdfium::rust::UseRustRenderCandidate()
                               ? pdfium::rust::BuildRustRenderLayerPlan(
                                     has_options, has_last_matrix)
                               : std::nullopt;
    const pdfium::rust::RenderLayerPlan plan = rust_plan.value_or(
        BuildCppRenderLayerPlan(has_options, has_last_matrix));
    if (plan.Has(pdfium::rust::RenderLayerPlanBit::kSetOptions)) {
      status.SetOptions(*pOptions);
    }
    status.SetStopObject(pStopObj);
    status.SetTransparency(layer.GetObjectHolder()->GetTransparency());
    CFX_Matrix final_matrix = layer.GetMatrix();
    if (plan.Has(pdfium::rust::RenderLayerPlanBit::kApplyLastMatrix)) {
      final_matrix *= *pLastMatrix;
      status.SetDeviceMatrix(*pLastMatrix);
    }
    status.Initialize(nullptr, nullptr);
    status.RenderObjectList(layer.GetObjectHolder(), final_matrix);
    const bool limited_image_cache =
        status.GetRenderOptions().GetOptions().bLimitedImageCache;
    const bool stopped = status.IsStopped();
    const auto rust_completion =
        pdfium::rust::UseRustRenderCandidate()
            ? pdfium::rust::BuildRustRenderLayerCompletion(limited_image_cache,
                                                           stopped)
            : std::nullopt;
    const pdfium::rust::RenderLayerCompletion completion =
        rust_completion.value_or(
            BuildCppRenderLayerCompletion(limited_image_cache, stopped));
    if (completion.Has(
            pdfium::rust::RenderLayerCompletionBit::kOptimizeCache)) {
      page_cache_->CacheOptimization(
          status.GetRenderOptions().GetCacheSizeLimit());
    }
    if (completion.Has(pdfium::rust::RenderLayerCompletionBit::kStop)) {
      break;
    }
  }
}

CPDF_RenderContext::Layer::Layer(CPDF_PageObjectHolder* pHolder,
                                 const CFX_Matrix& matrix)
    : object_holder_(pHolder), matrix_(matrix) {}

CPDF_RenderContext::Layer::Layer(const Layer& that) = default;

CPDF_RenderContext::Layer::~Layer() = default;
