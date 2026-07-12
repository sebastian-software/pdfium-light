// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#include "public/fpdf_doc.h"

#include <memory>
#include <optional>
#include <set>
#include <utility>

#include "constants/form_fields.h"
#include "core/fpdfapi/page/cpdf_annotcontext.h"
#include "core/fpdfapi/page/cpdf_page.h"
#include "core/fpdfapi/parser/cpdf_array.h"
#include "core/fpdfapi/parser/cpdf_dictionary.h"
#include "core/fpdfapi/parser/cpdf_document.h"
#include "core/fpdfapi/parser/cpdf_number.h"
#include "core/fpdfapi/parser/cpdf_string.h"
#include "core/fpdfapi/parser/fpdf_parser_decode.h"
#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "core/fpdfdoc/cpdf_aaction.h"
#include "core/fpdfdoc/cpdf_bookmark.h"
#include "core/fpdfdoc/cpdf_bookmarktree.h"
#include "core/fpdfdoc/cpdf_dest.h"
#include "core/fpdfdoc/cpdf_linklist.h"
#include "core/fpdfdoc/cpdf_pagelabel.h"
#include "core/fxcrt/check.h"
#include "core/fxcrt/compiler_specific.h"
#include "core/fxcrt/containers/contains.h"
#include "core/fxcrt/numerics/safe_conversions.h"
#include "core/fxcrt/span.h"
#include "core/fxcrt/span_util.h"
#include "core/fxge/dib/fx_dib.h"
#include "fpdfsdk/cpdfsdk_helpers.h"

namespace {

CPDF_Bookmark FindBookmark(const CPDF_BookmarkTree& tree,
                           CPDF_Bookmark bookmark,
                           const WideString& title,
                           std::set<const CPDF_Dictionary*>* visited) {
  // Return if already checked to avoid circular calling.
  if (pdfium::Contains(*visited, bookmark.GetDict())) {
    return CPDF_Bookmark();
  }
  visited->insert(bookmark.GetDict());

  if (bookmark.GetDict() &&
      bookmark.GetTitle().CompareNoCase(title.c_str()) == 0) {
    // First check this item.
    return bookmark;
  }

  // Go into children items.
  CPDF_Bookmark child = tree.GetFirstChild(bookmark);
  while (child.GetDict() && !pdfium::Contains(*visited, child.GetDict())) {
    // Check this item and its children.
    CPDF_Bookmark found = FindBookmark(tree, child, title, visited);
    if (found.GetDict()) {
      return found;
    }
    child = tree.GetNextSibling(child);
  }
  return CPDF_Bookmark();
}

CPDF_LinkList* GetLinkList(CPDF_Page* page) {
  CPDF_Document* doc = page->GetDocument();
  auto* pList = static_cast<CPDF_LinkList*>(doc->GetLinksContext());
  if (pList) {
    return pList;
  }

  auto pNewList = std::make_unique<CPDF_LinkList>();
  pList = pNewList.get();
  doc->SetLinksContext(std::move(pNewList));
  return pList;
}

}  // namespace

FPDF_EXPORT FPDF_BOOKMARK FPDF_CALLCONV
FPDFBookmark_GetFirstChild(FPDF_DOCUMENT document, FPDF_BOOKMARK bookmark) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }
  CPDF_BookmarkTree tree(doc);
  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  return FPDFBookmarkFromCPDFDictionary(
      tree.GetFirstChild(cBookmark).GetDict());
}

FPDF_EXPORT FPDF_BOOKMARK FPDF_CALLCONV
FPDFBookmark_GetNextSibling(FPDF_DOCUMENT document, FPDF_BOOKMARK bookmark) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }

  if (!bookmark) {
    return nullptr;
  }

  CPDF_BookmarkTree tree(doc);
  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  return FPDFBookmarkFromCPDFDictionary(
      tree.GetNextSibling(cBookmark).GetDict());
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFBookmark_GetTitle(FPDF_BOOKMARK bookmark,
                      void* buffer,
                      unsigned long buflen) {
  if (!bookmark) {
    return 0;
  }
  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  WideString title = cBookmark.GetTitle();
  // SAFETY: required from caller.
  return Utf16EncodeMaybeCopyAndReturnLength(
      title, UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen)));
}

FPDF_EXPORT int FPDF_CALLCONV FPDFBookmark_GetCount(FPDF_BOOKMARK bookmark) {
  if (!bookmark) {
    return 0;
  }
  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  return cBookmark.GetCount();
}

FPDF_EXPORT FPDF_BOOKMARK FPDF_CALLCONV
FPDFBookmark_Find(FPDF_DOCUMENT document, FPDF_WIDESTRING title) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }

  // SAFETY: required from caller.
  WideString encodedTitle = UNSAFE_BUFFERS(WideStringFromFPDFWideString(title));
  if (encodedTitle.IsEmpty()) {
    return nullptr;
  }

  CPDF_BookmarkTree tree(doc);
  if (pdfium::rust::UseRustParserCandidate()) {
    struct BookmarkContext {
      const CPDF_BookmarkTree* tree;
      const WideString* title;
    } context = {&tree, &encodedTitle};

    auto matches_title = [](void* opaque, uintptr_t handle, bool* matches) {
      auto* context = static_cast<BookmarkContext*>(opaque);
      CPDF_Bookmark bookmark(pdfium::WrapRetain(
          reinterpret_cast<const CPDF_Dictionary*>(handle)));
      *matches =
          bookmark.GetTitle().CompareNoCase(context->title->c_str()) == 0;
      return true;
    };
    auto first_child = [](void* opaque, uintptr_t handle, uintptr_t* output) {
      auto* context = static_cast<BookmarkContext*>(opaque);
      CPDF_Bookmark bookmark(pdfium::WrapRetain(
          reinterpret_cast<const CPDF_Dictionary*>(handle)));
      *output = reinterpret_cast<uintptr_t>(
          context->tree->GetFirstChild(bookmark).GetDict());
      return true;
    };
    auto next_sibling = [](void* opaque, uintptr_t handle, uintptr_t* output) {
      auto* context = static_cast<BookmarkContext*>(opaque);
      CPDF_Bookmark bookmark(pdfium::WrapRetain(
          reinterpret_cast<const CPDF_Dictionary*>(handle)));
      *output = reinterpret_cast<uintptr_t>(
          context->tree->GetNextSibling(bookmark).GetDict());
      return true;
    };
    std::optional<uintptr_t> result = pdfium::rust::RustFindBookmark(
        &context, matches_title, first_child, next_sibling);
    if (result.has_value()) {
      return FPDFBookmarkFromCPDFDictionary(
          reinterpret_cast<const CPDF_Dictionary*>(result.value()));
    }
  }

  std::set<const CPDF_Dictionary*> visited;
  return FPDFBookmarkFromCPDFDictionary(
      FindBookmark(tree, CPDF_Bookmark(), encodedTitle, &visited).GetDict());
}

FPDF_EXPORT FPDF_DEST FPDF_CALLCONV
FPDFBookmark_GetDest(FPDF_DOCUMENT document, FPDF_BOOKMARK bookmark) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }

  if (!bookmark) {
    return nullptr;
  }

  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  CPDF_Dest dest = cBookmark.GetDest(doc);
  if (dest.GetArray()) {
    return FPDFDestFromCPDFArray(dest.GetArray());
  }
  // If this bookmark is not directly associated with a dest, we try to get
  // action
  CPDF_Action action = cBookmark.GetAction();
  if (!action.HasDict()) {
    return nullptr;
  }
  return FPDFDestFromCPDFArray(action.GetDest(doc).GetArray());
}

FPDF_EXPORT FPDF_ACTION FPDF_CALLCONV
FPDFBookmark_GetAction(FPDF_BOOKMARK bookmark) {
  if (!bookmark) {
    return nullptr;
  }

  CPDF_Bookmark cBookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  return FPDFActionFromCPDFDictionary(cBookmark.GetAction().GetDict());
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFBookmark_GetColor(FPDF_BOOKMARK bookmark, float* R, float* G, float* B) {
  if (!bookmark || !R || !G || !B) {
    return false;
  }
  CPDF_Bookmark cpdf_bookmark(
      pdfium::WrapRetain(CPDFDictionaryFromFPDFBookmark(bookmark)));
  std::optional<FX_RGB_STRUCT<float>> color = cpdf_bookmark.GetColor();
  if (!color.has_value()) {
    return false;
  }
  if (color->red > 1 || color->green > 1 || color->blue > 1) {
    return false;
  }
  if (color->red < 0 || color->green < 0 || color->blue < 0) {
    return false;
  }
  *R = color->red;
  *G = color->green;
  *B = color->blue;
  return true;
}

FPDF_EXPORT unsigned long FPDF_CALLCONV FPDFAction_GetType(FPDF_ACTION action) {
  if (!action) {
    return PDFACTION_UNSUPPORTED;
  }

  CPDF_Action cAction(pdfium::WrapRetain(CPDFDictionaryFromFPDFAction(action)));
  if (pdfium::rust::UseRustParserCandidate()) {
    return pdfium::rust::RustPublicActionType(
        static_cast<uint8_t>(cAction.GetType()));
  }
  switch (cAction.GetType()) {
    case CPDF_Action::Type::kGoTo:
      return PDFACTION_GOTO;
    case CPDF_Action::Type::kGoToR:
      return PDFACTION_REMOTEGOTO;
    case CPDF_Action::Type::kGoToE:
      return PDFACTION_EMBEDDEDGOTO;
    case CPDF_Action::Type::kURI:
      return PDFACTION_URI;
    case CPDF_Action::Type::kLaunch:
      return PDFACTION_LAUNCH;
    default:
      return PDFACTION_UNSUPPORTED;
  }
}

FPDF_EXPORT FPDF_DEST FPDF_CALLCONV FPDFAction_GetDest(FPDF_DOCUMENT document,
                                                       FPDF_ACTION action) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }

  unsigned long type = FPDFAction_GetType(action);
  const bool allowed = pdfium::rust::UseRustParserCandidate()
                           ? pdfium::rust::RustPublicActionAllowsDestination(
                                 static_cast<uint8_t>(type))
                           : type == PDFACTION_GOTO ||
                                 type == PDFACTION_REMOTEGOTO ||
                                 type == PDFACTION_EMBEDDEDGOTO;
  if (!allowed) {
    return nullptr;
  }
  CPDF_Action cAction(pdfium::WrapRetain(CPDFDictionaryFromFPDFAction(action)));
  return FPDFDestFromCPDFArray(cAction.GetDest(doc).GetArray());
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAction_GetFilePath(FPDF_ACTION action, void* buffer, unsigned long buflen) {
  unsigned long type = FPDFAction_GetType(action);
  const bool allowed =
      pdfium::rust::UseRustParserCandidate()
          ? pdfium::rust::RustPublicActionAllowsFile(static_cast<uint8_t>(type))
          : type == PDFACTION_REMOTEGOTO || type == PDFACTION_EMBEDDEDGOTO ||
                type == PDFACTION_LAUNCH;
  if (!allowed) {
    return 0;
  }
  CPDF_Action cAction(pdfium::WrapRetain(CPDFDictionaryFromFPDFAction(action)));
  ByteString path = cAction.GetFilePath().ToUTF8();
  // SAFETY: required from caller.
  return NulTerminateMaybeCopyAndReturnLength(
      path, UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen)));
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAction_GetURIPath(FPDF_DOCUMENT document,
                      FPDF_ACTION action,
                      void* buffer,
                      unsigned long buflen) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return 0;
  }
  unsigned long type = FPDFAction_GetType(action);
  const bool allowed =
      pdfium::rust::UseRustParserCandidate()
          ? pdfium::rust::RustPublicActionAllowsUri(static_cast<uint8_t>(type))
          : type == PDFACTION_URI;
  if (!allowed) {
    return 0;
  }
  // SAFETY: required from caller.
  auto result_span = UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen));
  CPDF_Action cAction(pdfium::WrapRetain(CPDFDictionaryFromFPDFAction(action)));
  ByteString path = cAction.GetURI(doc);
  fxcrt::try_spancpy(result_span, path.span_with_terminator());
  return static_cast<unsigned long>(path.span_with_terminator().size());
}

FPDF_EXPORT int FPDF_CALLCONV FPDFDest_GetDestPageIndex(FPDF_DOCUMENT document,
                                                        FPDF_DEST dest) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return -1;
  }

  if (!dest) {
    return -1;
  }

  CPDF_Dest destination(pdfium::WrapRetain(CPDFArrayFromFPDFDest(dest)));
  return destination.GetDestPageIndex(doc);
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFDest_GetView(FPDF_DEST dest, unsigned long* pNumParams, FS_FLOAT* pParams) {
  if (!dest) {
    *pNumParams = 0;
    return 0;
  }
  CPDF_Dest destination(pdfium::WrapRetain(CPDFArrayFromFPDFDest(dest)));
  const unsigned long nParams =
      pdfium::checked_cast<unsigned long>(destination.GetNumParams());
  DCHECK(nParams <= 4);
  *pNumParams = nParams;
  for (unsigned long i = 0; i < nParams; ++i) {
    UNSAFE_TODO(pParams[i]) = destination.GetParam(i);
  }
  return destination.GetZoomMode();
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFDest_GetLocationInPage(FPDF_DEST dest,
                           FPDF_BOOL* hasXVal,
                           FPDF_BOOL* hasYVal,
                           FPDF_BOOL* hasZoomVal,
                           FS_FLOAT* x,
                           FS_FLOAT* y,
                           FS_FLOAT* zoom) {
  if (!dest) {
    return false;
  }

  CPDF_Dest destination(pdfium::WrapRetain(CPDFArrayFromFPDFDest(dest)));

  // FPDF_BOOL is an int, GetXYZ expects bools.
  bool bHasX;
  bool bHasY;
  bool bHasZoom;
  if (!destination.GetXYZ(&bHasX, &bHasY, &bHasZoom, x, y, zoom)) {
    return false;
  }

  *hasXVal = bHasX;
  *hasYVal = bHasY;
  *hasZoomVal = bHasZoom;
  return true;
}

FPDF_EXPORT FPDF_LINK FPDF_CALLCONV FPDFLink_GetLinkAtPoint(FPDF_PAGE page,
                                                            double x,
                                                            double y) {
  CPDF_Page* pPage = CPDFPageFromFPDFPage(page);
  if (!pPage) {
    return nullptr;
  }

  CPDF_LinkList* pLinkList = GetLinkList(pPage);
  if (!pLinkList) {
    return nullptr;
  }

  CPDF_Link link = pLinkList->GetLinkAtPoint(
      pPage, CFX_PointF(static_cast<float>(x), static_cast<float>(y)), nullptr);

  // Unretained reference in public API. NOLINTNEXTLINE
  return FPDFLinkFromCPDFDictionary(link.GetMutableDict());
}

FPDF_EXPORT int FPDF_CALLCONV FPDFLink_GetLinkZOrderAtPoint(FPDF_PAGE page,
                                                            double x,
                                                            double y) {
  CPDF_Page* pPage = CPDFPageFromFPDFPage(page);
  if (!pPage) {
    return -1;
  }

  CPDF_LinkList* pLinkList = GetLinkList(pPage);
  if (!pLinkList) {
    return -1;
  }

  int z_order = -1;
  pLinkList->GetLinkAtPoint(
      pPage, CFX_PointF(static_cast<float>(x), static_cast<float>(y)),
      &z_order);
  return z_order;
}

FPDF_EXPORT FPDF_DEST FPDF_CALLCONV FPDFLink_GetDest(FPDF_DOCUMENT document,
                                                     FPDF_LINK link) {
  if (!link) {
    return nullptr;
  }
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return nullptr;
  }
  CPDF_Link cLink(pdfium::WrapRetain(CPDFDictionaryFromFPDFLink(link)));
  FPDF_DEST dest = FPDFDestFromCPDFArray(cLink.GetDest(doc).GetArray());
  if (dest) {
    return dest;
  }
  // If this link is not directly associated with a dest, we try to get action
  CPDF_Action action = cLink.GetAction();
  if (!action.HasDict()) {
    return nullptr;
  }
  return FPDFDestFromCPDFArray(action.GetDest(doc).GetArray());
}

FPDF_EXPORT FPDF_ACTION FPDF_CALLCONV FPDFLink_GetAction(FPDF_LINK link) {
  if (!link) {
    return nullptr;
  }

  CPDF_Link cLink(pdfium::WrapRetain(CPDFDictionaryFromFPDFLink(link)));
  return FPDFActionFromCPDFDictionary(cLink.GetAction().GetDict());
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFLink_Enumerate(FPDF_PAGE page,
                                                       int* start_pos,
                                                       FPDF_LINK* link_annot) {
  if (!start_pos || !link_annot) {
    return false;
  }
  CPDF_Page* pPage = CPDFPageFromFPDFPage(page);
  if (!pPage) {
    return false;
  }
  RetainPtr<CPDF_Array> pAnnots = pPage->GetMutableAnnotsArray();
  if (!pAnnots) {
    return false;
  }
  if (pdfium::rust::UseRustParserCandidate()) {
    auto is_link = [](void* opaque, size_t index, bool* result) {
      auto* annotations = static_cast<CPDF_Array*>(opaque);
      RetainPtr<CPDF_Dictionary> dictionary =
          ToDictionary(annotations->GetMutableDirectObjectAt(index));
      *result =
          dictionary && dictionary->GetByteStringFor("Subtype") == "Link";
      return true;
    };
    std::optional<pdfium::rust::RustLinkEnumerationResult> result =
        pdfium::rust::RustFindNextLink(*start_pos, pAnnots->size(),
                                      pAnnots.Get(), is_link);
    if (result.has_value()) {
      if (!result->found) {
        return false;
      }
      RetainPtr<CPDF_Dictionary> dictionary = ToDictionary(
          pAnnots->GetMutableDirectObjectAt(result->index));
      if (!dictionary) {
        return false;
      }
      *start_pos = pdfium::checked_cast<int>(result->index + 1);
      *link_annot = FPDFLinkFromCPDFDictionary(dictionary.Get());
      return true;
    }
  }
  for (size_t i = *start_pos; i < pAnnots->size(); i++) {
    RetainPtr<CPDF_Dictionary> dict =
        ToDictionary(pAnnots->GetMutableDirectObjectAt(i));
    if (!dict) {
      continue;
    }
    if (dict->GetByteStringFor("Subtype") == "Link") {
      *start_pos = static_cast<int>(i + 1);
      *link_annot = FPDFLinkFromCPDFDictionary(dict.Get());
      return true;
    }
  }
  return false;
}

FPDF_EXPORT FPDF_ANNOTATION FPDF_CALLCONV
FPDFLink_GetAnnot(FPDF_PAGE page, FPDF_LINK link_annot) {
  CPDF_Page* pPage = CPDFPageFromFPDFPage(page);
  RetainPtr<CPDF_Dictionary> pAnnotDict(CPDFDictionaryFromFPDFLink(link_annot));
  if (!pPage || !pAnnotDict) {
    return nullptr;
  }

  auto pAnnotContext = std::make_unique<CPDF_AnnotContext>(
      std::move(pAnnotDict), IPDFPageFromFPDFPage(page));

  // Caller takes the ownership of the object.
  return FPDFAnnotationFromCPDFAnnotContext(pAnnotContext.release());
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFLink_GetAnnotRect(FPDF_LINK link_annot,
                                                          FS_RECTF* rect) {
  if (!link_annot || !rect) {
    return false;
  }

  CPDF_Dictionary* pAnnotDict = CPDFDictionaryFromFPDFLink(link_annot);
  *rect = FSRectFFromCFXFloatRect(pAnnotDict->GetRectFor("Rect"));
  return true;
}

FPDF_EXPORT int FPDF_CALLCONV FPDFLink_CountQuadPoints(FPDF_LINK link_annot) {
  RetainPtr<const CPDF_Array> pArray =
      GetQuadPointsArrayFromDictionary(CPDFDictionaryFromFPDFLink(link_annot));
  return pArray ? static_cast<int>(pArray->size() / 8) : 0;
}

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFLink_GetQuadPoints(FPDF_LINK link_annot,
                       int quad_index,
                       FS_QUADPOINTSF* quad_points) {
  if (!quad_points || quad_index < 0) {
    return false;
  }

  const CPDF_Dictionary* pLinkDict = CPDFDictionaryFromFPDFLink(link_annot);
  if (!pLinkDict) {
    return false;
  }

  RetainPtr<const CPDF_Array> pArray =
      GetQuadPointsArrayFromDictionary(pLinkDict);
  if (!pArray) {
    return false;
  }

  return GetQuadPointsAtIndex(std::move(pArray),
                              static_cast<size_t>(quad_index), quad_points);
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_GetFileIdentifier(FPDF_DOCUMENT document,
                       FPDF_FILEIDTYPE id_type,
                       void* buffer,
                       unsigned long buflen) {
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return 0;
  }

  // Check if |id_type| is valid.
  if (id_type != FILEIDTYPE_PERMANENT && id_type != FILEIDTYPE_CHANGING) {
    return 0;
  }

  RetainPtr<const CPDF_Array> pFileId = doc->GetFileIdentifier();
  if (!pFileId) {
    return 0;
  }

  size_t nIndex = id_type == FILEIDTYPE_PERMANENT ? 0 : 1;
  RetainPtr<const CPDF_String> pValue =
      ToString(pFileId->GetDirectObjectAt(nIndex));
  if (!pValue) {
    return 0;
  }

  // SAFETY: required from caller.
  return NulTerminateMaybeCopyAndReturnLength(
      pValue->GetString(), UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen)));
}

FPDF_EXPORT unsigned long FPDF_CALLCONV FPDF_GetMetaText(FPDF_DOCUMENT document,
                                                         FPDF_BYTESTRING tag,
                                                         void* buffer,
                                                         unsigned long buflen) {
  if (!tag) {
    return 0;
  }
  CPDF_Document* doc = CPDFDocumentFromFPDFDocument(document);
  if (!doc) {
    return 0;
  }

  RetainPtr<const CPDF_Dictionary> pInfo = doc->GetInfo();
  if (!pInfo) {
    return 0;
  }

  // SAFETY: required from caller.
  return Utf16EncodeMaybeCopyAndReturnLength(
      pInfo->GetUnicodeTextFor(tag),
      UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen)));
}

FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_GetPageLabel(FPDF_DOCUMENT document,
                  int page_index,
                  void* buffer,
                  unsigned long buflen) {
  if (page_index < 0) {
    return 0;
  }

  // CPDF_PageLabel can deal with NULL |document|.
  CPDF_PageLabel label(CPDFDocumentFromFPDFDocument(document));
  std::optional<WideString> str = label.GetLabel(page_index);
  if (!str.has_value()) {
    return 0;
  }
  // SAFETY: required from caller.
  return Utf16EncodeMaybeCopyAndReturnLength(
      str.value(), UNSAFE_BUFFERS(SpanFromFPDFApiArgs(buffer, buflen)));
}
