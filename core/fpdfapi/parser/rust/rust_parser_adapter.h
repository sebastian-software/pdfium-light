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
using CrossRefMutationCallback = bool (*)(void* context, uint8_t action);
bool RunRustCrossRefEntryMutation(uint8_t type_code,
                                  bool normal_offset_fits,
                                  uint32_t generation,
                                  bool archive_object_valid,
                                  void* context,
                                  CrossRefMutationCallback callback);
using CrossRefMapSizeCallback = bool (*)(void* context,
                                         uint8_t action,
                                         uint32_t value);
bool RunRustCrossRefMapSize(uint32_t size,
                            void* context,
                            CrossRefMapSizeCallback callback);
std::optional<uint8_t> RustCrossRefMergeAction(bool has_new,
                                               uint8_t current_type,
                                               bool current_is_object_stream,
                                               uint8_t new_type);
std::optional<uint8_t> RustCrossRefTableAddAction(uint8_t object_type,
                                                  uint16_t current_generation,
                                                  bool current_is_object_stream,
                                                  uint16_t new_generation);
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
struct CrossRefEntryFields {
  uint32_t first;
  uint32_t second;
  uint32_t third;
};
std::optional<CrossRefEntryFields> RustReadCrossRefEntry(
    pdfium::span<const uint8_t> input,
    uint32_t first_width,
    uint32_t second_width,
    uint32_t third_width);
std::optional<uint8_t> RustSkipPdfSpacesAndComments(
    pdfium::span<const uint8_t> input,
    uint32_t* position);
struct PdfTokenScan {
  uint32_t position;
  bool has_word;
  uint32_t start;
  uint32_t len;
};
std::optional<PdfTokenScan> RustScanPdfToken(pdfium::span<const uint8_t> input,
                                             uint32_t position);

bool UseRustParserCandidate();
bool SetUseRustParserCandidateForTesting(bool use_candidate);

class ScopedRustParserImplementationForTesting final {
 public:
  explicit ScopedRustParserImplementationForTesting(bool use_candidate);
  ScopedRustParserImplementationForTesting(
      const ScopedRustParserImplementationForTesting&) = delete;
  ScopedRustParserImplementationForTesting& operator=(
      const ScopedRustParserImplementationForTesting&) = delete;
  ~ScopedRustParserImplementationForTesting();

 private:
  bool previous_;
};

}  // namespace pdfium::rust

#endif  // CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
