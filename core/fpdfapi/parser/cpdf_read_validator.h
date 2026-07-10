// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef CORE_FPDFAPI_PARSER_CPDF_READ_VALIDATOR_H_
#define CORE_FPDFAPI_PARSER_CPDF_READ_VALIDATOR_H_

#include "core/fxcrt/fx_memory.h"
#include "core/fxcrt/fx_stream.h"
#include "core/fxcrt/retain_ptr.h"

class CPDF_ReadValidator final : public IFX_SeekableReadStream {
 public:
  class ScopedSession {
   public:
    FX_STACK_ALLOCATED();

    explicit ScopedSession(RetainPtr<CPDF_ReadValidator> validator);
    ScopedSession(const ScopedSession& that) = delete;
    ScopedSession& operator=(const ScopedSession& that) = delete;
    ~ScopedSession();

   private:
    RetainPtr<CPDF_ReadValidator> const validator_;
    const bool saved_read_error_;
  };

  CONSTRUCT_VIA_MAKE_RETAIN;

  bool read_error() const { return read_error_; }
  bool has_read_problems() const { return read_error(); }

  void ResetErrors();
  bool CheckDataRangeAndRequestIfUnavailable(FX_FILESIZE offset, size_t size);

  // IFX_SeekableReadStream overrides:
  bool ReadBlockAtOffset(pdfium::span<uint8_t> buffer,
                         FX_FILESIZE offset) override;
  FX_FILESIZE GetSize() override;

 protected:
  explicit CPDF_ReadValidator(RetainPtr<IFX_SeekableReadStream> file_read);
  ~CPDF_ReadValidator() override;

 private:
  RetainPtr<IFX_SeekableReadStream> const file_read_;
  bool read_error_ = false;
  const FX_FILESIZE file_size_;
};

#endif  // CORE_FPDFAPI_PARSER_CPDF_READ_VALIDATOR_H_
