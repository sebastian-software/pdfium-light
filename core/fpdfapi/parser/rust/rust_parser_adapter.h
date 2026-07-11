#ifndef CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
#define CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_

#include <stdint.h>

#include <optional>

#include "core/fxcrt/span.h"

namespace pdfium::rust {

std::optional<uint32_t> RustReadBigEndianVarInt(
    pdfium::span<const uint8_t> input);
std::optional<uint8_t> RustCrossRefObjectType(uint32_t type_code);
std::optional<uint8_t> RustCrossRefEntryType(bool has_type_field,
                                             uint32_t type_code);
std::optional<uint8_t> RustCrossRefEntryAction(uint8_t type_code,
                                               bool normal_offset_fits,
                                               uint32_t generation,
                                               bool archive_object_valid);
struct CrossRefIndexPair {
  uint32_t start;
  uint32_t count;
};
std::optional<CrossRefIndexPair> RustCrossRefIndexPair(int32_t start,
                                                        int32_t count);
struct CrossRefSegmentRange {
  size_t offset;
  size_t len;
};
using CrossRefSegmentCallback = bool (*)(void* context, uint32_t entry_index);
std::optional<CrossRefSegmentRange> RustCrossRefSegmentRange(
    uint32_t segment_index,
    uint32_t object_count,
    uint32_t entry_width,
    size_t data_len);
bool RunRustCrossRefSegmentEntries(uint32_t entry_count,
                                   void* context,
                                   CrossRefSegmentCallback callback);
std::optional<uint32_t> RustCrossRefFieldWidth(int32_t value);

bool UseRustParserCandidate();
bool SetUseRustParserCandidateForTesting(bool use_candidate);

}  // namespace pdfium::rust

#endif  // CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
