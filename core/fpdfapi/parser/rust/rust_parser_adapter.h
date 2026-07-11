#ifndef CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
#define CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_

#include <stdint.h>

#include <optional>

#include "core/fxcrt/span.h"

namespace pdfium::rust {

std::optional<uint32_t> RustReadBigEndianVarInt(
    pdfium::span<const uint8_t> input);

bool UseRustParserCandidate();
bool SetUseRustParserCandidateForTesting(bool use_candidate);

}  // namespace pdfium::rust

#endif  // CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
