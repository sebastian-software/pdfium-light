// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "fpdfsdk/cpdfsdk_renderpage.h"

#include <memory>
#include <utility>

#include "build/build_config.h"
#include "core/fpdfapi/page/cpdf_pageimagecache.h"
#include "core/fpdfapi/render/cpdf_pagerendercontext.h"
#include "core/fpdfapi/render/cpdf_rendercontext.h"
#include "core/fpdfapi/render/cpdf_renderoptions.h"
#include "core/fpdfapi/render/rust/rust_render_adapter.h"
#include "core/fpdfdoc/cpdf_annotlist.h"
#include "core/fxge/cfx_renderdevice.h"
#include "fpdfsdk/cpdfsdk_helpers.h"

namespace {

thread_local RenderImplementationForTesting g_render_implementation =
    RenderImplementationForTesting::kCandidate;

void RenderPageCppReference(CPDF_PageRenderContext* context,
                            CPDF_Page* pPage,
                            const CFX_Matrix& matrix,
                            const FX_RECT& clipping_rect,
                            int flags,
                            const FPDF_COLORSCHEME* color_scheme,
                            bool need_to_restore) {
  if (!context->options_) {
    context->options_ = std::make_unique<CPDF_RenderOptions>();
  }

  auto& options = context->options_->GetOptions();
  options.bClearType = !!(flags & FPDF_LCD_TEXT);
  options.bNoNativeText = !!(flags & FPDF_NO_NATIVETEXT);
  options.bLimitedImageCache = !!(flags & FPDF_RENDER_LIMITEDIMAGECACHE);
  options.bForceHalftone = !!(flags & FPDF_RENDER_FORCEHALFTONE);
  options.bNoTextSmooth = !!(flags & FPDF_RENDER_NO_SMOOTHTEXT);
  options.bNoImageSmooth = !!(flags & FPDF_RENDER_NO_SMOOTHIMAGE);
  options.bNoPathSmooth = !!(flags & FPDF_RENDER_NO_SMOOTHPATH);

  // Grayscale output
  if (flags & FPDF_GRAYSCALE) {
    context->options_->SetColorMode(CPDF_RenderOptions::kGray);
  }

  if (color_scheme) {
    context->options_->SetColorMode(CPDF_RenderOptions::kForcedColor);
    SetColorFromScheme(color_scheme, context->options_.get());
    options.bConvertFillToStroke = !!(flags & FPDF_CONVERT_FILL_TO_STROKE);
  }

  const CPDF_OCContext::UsageType usage =
      (flags & FPDF_PRINTING) ? CPDF_OCContext::kPrint : CPDF_OCContext::kView;
  context->options_->SetOCContext(
      pdfium::MakeRetain<CPDF_OCContext>(pPage->GetDocument(), usage));

  context->device_->SaveState();
  context->device_->SetBaseClip(clipping_rect);
  context->device_->SetClip_Rect(clipping_rect);
  context->context_ = std::make_unique<CPDF_RenderContext>(
      pPage->GetDocument(), pPage->GetMutablePageResources(),
      pPage->GetPageImageCache());

  context->context_->AppendLayer(pPage, matrix);

  if (flags & FPDF_ANNOT) {
    auto pOwnedList = std::make_unique<CPDF_AnnotList>(pPage);
    CPDF_AnnotList* pList = pOwnedList.get();
    context->annots_ = std::move(pOwnedList);
    bool is_printing = (flags & FPDF_PRINTING);
#if BUILDFLAG(IS_WIN)
    is_printing |= context->device_->GetDeviceType() == DeviceType::kPrinter;
#endif

    // TODO(https://crbug.com/42271964) - maybe pass true here.
    const bool bShowWidget = false;
    pList->DisplayAnnots(context->context_.get(), is_printing, matrix,
                         bShowWidget);
  }

  context->context_->Render(context->device_.get(), nullptr,
                            context->options_.get(), nullptr);
  if (need_to_restore) {
    context->device_->RestoreState(false);
  }
}

void RenderPageCandidate(CPDF_PageRenderContext* context,
                         CPDF_Page* pPage,
                         const CFX_Matrix& matrix,
                         const FX_RECT& clipping_rect,
                         int flags,
                         const FPDF_COLORSCHEME* color_scheme,
                         bool need_to_restore) {
  const auto plan = pdfium::rust::BuildRustRenderRequestPlan(
      flags, color_scheme != nullptr, need_to_restore);
  if (!plan.has_value()) {
    RenderPageCppReference(context, pPage, matrix, clipping_rect, flags,
                           color_scheme, need_to_restore);
    return;
  }

  if (!context->options_) {
    context->options_ = std::make_unique<CPDF_RenderOptions>();
  }

  auto& options = context->options_->GetOptions();
  options.bClearType = plan->Has(pdfium::rust::RenderPlanBit::kClearType);
  options.bNoNativeText = plan->Has(pdfium::rust::RenderPlanBit::kNoNativeText);
  options.bLimitedImageCache =
      plan->Has(pdfium::rust::RenderPlanBit::kLimitedImageCache);
  options.bForceHalftone =
      plan->Has(pdfium::rust::RenderPlanBit::kForceHalftone);
  options.bNoTextSmooth =
      plan->Has(pdfium::rust::RenderPlanBit::kNoTextSmoothing);
  options.bNoImageSmooth =
      plan->Has(pdfium::rust::RenderPlanBit::kNoImageSmoothing);
  options.bNoPathSmooth =
      plan->Has(pdfium::rust::RenderPlanBit::kNoPathSmoothing);

  if (plan->Has(pdfium::rust::RenderPlanBit::kGrayscale)) {
    context->options_->SetColorMode(CPDF_RenderOptions::kGray);
  }
  if (plan->Has(pdfium::rust::RenderPlanBit::kForcedColor)) {
    context->options_->SetColorMode(CPDF_RenderOptions::kForcedColor);
    SetColorFromScheme(color_scheme, context->options_.get());
    options.bConvertFillToStroke =
        plan->Has(pdfium::rust::RenderPlanBit::kConvertFillToStroke);
  }

  const CPDF_OCContext::UsageType usage =
      plan->Has(pdfium::rust::RenderPlanBit::kPrinting) ? CPDF_OCContext::kPrint
                                                        : CPDF_OCContext::kView;
  context->options_->SetOCContext(
      pdfium::MakeRetain<CPDF_OCContext>(pPage->GetDocument(), usage));

  context->device_->SaveState();
  context->device_->SetBaseClip(clipping_rect);
  context->device_->SetClip_Rect(clipping_rect);
  context->context_ = std::make_unique<CPDF_RenderContext>(
      pPage->GetDocument(), pPage->GetMutablePageResources(),
      pPage->GetPageImageCache());
  context->context_->AppendLayer(pPage, matrix);

  if (plan->Has(pdfium::rust::RenderPlanBit::kAnnotations)) {
    auto owned_list = std::make_unique<CPDF_AnnotList>(pPage);
    CPDF_AnnotList* list = owned_list.get();
    context->annots_ = std::move(owned_list);
    bool is_printing = plan->Has(pdfium::rust::RenderPlanBit::kPrinting);
#if BUILDFLAG(IS_WIN)
    is_printing |= context->device_->GetDeviceType() == DeviceType::kPrinter;
#endif
    list->DisplayAnnots(context->context_.get(), is_printing, matrix,
                        /*bShowWidget=*/false);
  }

  context->context_->Render(context->device_.get(), nullptr,
                            context->options_.get(), nullptr);
  if (plan->Has(pdfium::rust::RenderPlanBit::kRestoreDevice)) {
    context->device_->RestoreState(false);
  }
}

void RenderPageSelected(CPDF_PageRenderContext* context,
                        CPDF_Page* pPage,
                        const CFX_Matrix& matrix,
                        const FX_RECT& clipping_rect,
                        int flags,
                        const FPDF_COLORSCHEME* color_scheme,
                        bool need_to_restore) {
  switch (g_render_implementation) {
    case RenderImplementationForTesting::kCppReference:
      RenderPageCppReference(context, pPage, matrix, clipping_rect, flags,
                             color_scheme, need_to_restore);
      return;
    case RenderImplementationForTesting::kCandidate:
      RenderPageCandidate(context, pPage, matrix, clipping_rect, flags,
                          color_scheme, need_to_restore);
      return;
  }
}

}  // namespace

ScopedRenderImplementationForTesting::ScopedRenderImplementationForTesting(
    RenderImplementationForTesting implementation)
    : previous_(g_render_implementation),
      previous_core_candidate_(
          pdfium::rust::SetUseRustRenderCandidateForTesting(
              implementation == RenderImplementationForTesting::kCandidate)) {
  g_render_implementation = implementation;
}

ScopedRenderImplementationForTesting::~ScopedRenderImplementationForTesting() {
  pdfium::rust::SetUseRustRenderCandidateForTesting(previous_core_candidate_);
  g_render_implementation = previous_;
}

void CPDFSDK_RenderPage(CPDF_PageRenderContext* context,
                        CPDF_Page* pPage,
                        const CFX_Matrix& matrix,
                        const FX_RECT& clipping_rect,
                        int flags,
                        const FPDF_COLORSCHEME* color_scheme) {
  RenderPageSelected(context, pPage, matrix, clipping_rect, flags, color_scheme,
                     /*need_to_restore=*/true);
}

void CPDFSDK_RenderPageWithContext(CPDF_PageRenderContext* context,
                                   CPDF_Page* pPage,
                                   int start_x,
                                   int start_y,
                                   int size_x,
                                   int size_y,
                                   int rotate,
                                   int flags,
                                   const FPDF_COLORSCHEME* color_scheme,
                                   bool need_to_restore) {
  const FX_RECT rect(start_x, start_y, start_x + size_x, start_y + size_y);
  RenderPageSelected(context, pPage,
                     pPage->GetDisplayMatrixForRect(rect, rotate), rect, flags,
                     color_scheme, need_to_restore);
}
