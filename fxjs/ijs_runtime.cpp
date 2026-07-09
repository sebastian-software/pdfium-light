// Copyright 2018 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include "fxjs/ijs_runtime.h"

#include "fxjs/cjs_runtimestub.h"

IJS_Runtime::ScopedEventContext::ScopedEventContext(IJS_Runtime* pRuntime)
    : runtime_(pRuntime), context_(pRuntime->NewEventContext()) {}

IJS_Runtime::ScopedEventContext::~ScopedEventContext() {
  runtime_->ReleaseEventContext(context_.ExtractAsDangling());
}

// static
std::unique_ptr<IJS_Runtime> IJS_Runtime::Create(
    CPDFSDK_FormFillEnvironment* pFormFillEnv) {
  return std::make_unique<CJS_RuntimeStub>(pFormFillEnv);
}

IJS_Runtime::~IJS_Runtime() = default;

IJS_Runtime::JS_Error::JS_Error(int line,
                                int column,
                                const WideString& exception)
    : line(line), column(column), exception(exception) {}
