#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"

namespace {

extern "C" bool pdfium_rust_read_big_endian_var_int(const uint8_t* data,
                                                     size_t len,
                                                     uint32_t* output);
extern "C" bool pdfium_rust_cross_ref_object_type(uint32_t type_code,
                                                   uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_entry_type(bool has_type_field,
                                                  uint32_t type_code,
                                                  uint8_t* output);

thread_local bool g_use_rust_parser_candidate = true;

}  // namespace

namespace pdfium::rust {

std::optional<uint32_t> RustReadBigEndianVarInt(
    pdfium::span<const uint8_t> input) {
  uint32_t output = 0;
  if (!pdfium_rust_read_big_endian_var_int(input.data(), input.size(),
                                           &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<uint8_t> RustCrossRefObjectType(uint32_t type_code) {
  uint8_t output = 0;
  if (!pdfium_rust_cross_ref_object_type(type_code, &output) || output > 2) {
    return std::nullopt;
  }
  return output;
}

std::optional<uint8_t> RustCrossRefEntryType(bool has_type_field,
                                             uint32_t type_code) {
  uint8_t output = 0;
  if (!pdfium_rust_cross_ref_entry_type(has_type_field, type_code, &output) ||
      output > 2) {
    return std::nullopt;
  }
  return output;
}

bool UseRustParserCandidate() {
  return g_use_rust_parser_candidate;
}

bool SetUseRustParserCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_parser_candidate;
  g_use_rust_parser_candidate = use_candidate;
  return previous;
}

}  // namespace pdfium::rust
