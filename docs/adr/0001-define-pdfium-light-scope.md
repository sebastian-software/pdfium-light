# ADR 0001: Define the pdfium-light scope

## Status

Accepted

## Context

PDFium is a broad PDF runtime. It supports static page rendering, text
extraction, page object editing, saving, annotations, forms, JavaScript, XFA,
embedding callbacks, progressive loading, and several testing and sample
surfaces.

pdfium-light exists to keep the static document pipeline while removing
interactive PDF runtime features. The target use cases are:

- render PDF pages to bitmaps for PNG/JPEG encoding by the caller or a thin
  utility layer;
- inspect text and page objects;
- preserve and edit ordinary, non-interactive annotations;
- remove, replace, or insert page objects for small document corrections;
- apply conservative real redactions that remove underlying content instead of
  only covering it visually;
- save the modified PDF.

The project does not need AcroForm filling, widget interaction, embedded
JavaScript execution, XFA forms, OCR, or application-embedding event handling.

## Decision

pdfium-light will be a reduced-scope PDFium-compatible C API library, not a new
high-level wrapper API.

Public APIs that remain in scope keep PDFium-style C signatures and handle
types. Public APIs for removed features will be removed from headers and build
outputs rather than retained as compatibility stubs.

The hard product boundary is:

- static PDF reading, rendering, inspection, annotation handling, page-object
  editing, redaction, and saving are in scope;
- interactive PDF runtime features are out of scope.

## In Scope

The intended retained API surface is:

- `fpdfview.h` for library lifecycle, document loading, page loading, page
  geometry, bitmap creation, and bitmap rendering;
- `fpdf_text.h` and the useful text-index helpers for text extraction,
  positions, search, and OCR-overlay alignment workflows;
- `fpdf_edit.h` for page object inspection, insertion, removal, text/image/path
  edits, content regeneration, and document creation;
- `fpdf_save.h` for writing modified documents;
- non-interactive parts of `fpdf_annot.h`, including highlights, text notes,
  freetext, ink, links, stamps, geometric annotations, and redaction
  annotations;
- `fpdf_transformpage.h` for page boxes, transforms, and clipping support;
- selected document-navigation and metadata helpers where they do not pull in
  interactive runtime behavior.

Rendering should continue to support ordinary non-interactive annotations.
Widget annotations are not required to render unless they have already been
flattened into static page content.

## Out of Scope

The following features should be removed from public headers, implementation
targets, tests, samples, and dependencies:

- AcroForm filling and form widget interaction;
- XFA forms and the XFA runtime;
- embedded JavaScript execution and V8 integration;
- JavaScript platform callbacks and document/page/form action execution;
- focus, mouse, keyboard, timer, cursor, and selection event handling for
  interactive forms;
- form-specific annotation helpers that require `FPDF_FORMHANDLE`;
- compatibility stubs for removed features.

The project should not expose removed features as supported-but-failing APIs.
Callers should discover unsupported features at compile time.

## Redaction Policy

Redaction is in scope only when it removes the underlying content. Visual
overlays alone are not sufficient.

The initial redaction implementation should be conservative:

- remove text, images, paths, and page objects only when the implementation can
  prove the target content is safely covered by the redaction region;
- report partially intersecting or nested content that cannot yet be handled
  safely;
- avoid silently producing a document where redacted text remains extractable;
- validate successful redactions through both rendering and text extraction.

More advanced handling, such as partial image rewriting or recursive form
XObject editing, can be added after the conservative path is correct.

## Platform Policy

The platform scope is documented for users in [`docs/platform-support.md`](../platform-support.md). The current policy is:

- primary: Linux x64 glibc, Linux arm64 glibc, Linux musl, macOS arm64,
  Windows x64;
- probe: Windows arm64. The current local probe is blocked by missing GN/build metadata, so a full gclient/CI probe is still required before promoting or rejecting it;
- out of scope: macOS x64/Intel, Windows x86/32-bit, mobile platforms,
  big-endian targets, and legacy MSVC support.

Windows-specific code should not be removed just because it is Windows-specific.
Only Windows x86/32-bit-only support is out of scope.

## Removal Method

Removals should be staged:

1. Remove or gate public headers and exported declarations for out-of-scope
   features.
2. Split monolithic targets where needed so retained APIs do not depend on
   removed features.
3. Remove implementation directories, tests, samples, and dependencies that are
   exclusively tied to out-of-scope features.
4. Keep a removal audit log for every substantial removal.
5. Validate each stage with focused build and API checks before deleting more
   code.

## Validation Expectations

Each removal stage should prove:

- the retained public headers compile for supported platforms;
- the retained library builds without V8, XFA, form filler, and PWL targets;
- static rendering still works for ordinary PDFs;
- ordinary non-interactive annotations still render and can be inspected;
- retained edit/save APIs can remove and add page objects and persist changes;
- redaction tests prove that removed text is not extractable after save;
- removed headers are no longer part of the installed/exported public API.

## Consequences

This decision reduces binary size, dependency surface, security exposure, and
maintenance burden for the intended static-document use case.

It also makes pdfium-light intentionally incompatible with consumers that need
forms, XFA, JavaScript, or PDF viewer-style interaction. Those consumers should
use full PDFium instead.

