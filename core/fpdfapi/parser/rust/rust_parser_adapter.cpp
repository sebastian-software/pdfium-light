#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"

#include <limits>

namespace {

extern "C" bool pdfium_rust_read_big_endian_var_int(const uint8_t* data,
                                                     size_t len,
                                                     uint32_t* output);
extern "C" bool pdfium_rust_cross_ref_object_type(uint32_t type_code,
                                                   uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_entry_type(bool has_type_field,
                                                  uint32_t type_code,
                                                  uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_entry_action(
    uint8_t type_code,
    bool normal_offset_fits,
    uint32_t generation,
    bool archive_object_valid,
    uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_index_pair(int32_t start,
                                                  int32_t count,
                                                  uint32_t* output_start,
                                                  uint32_t* output_count);
extern "C" bool pdfium_rust_cross_ref_segment_range(
    uint32_t segment_index,
    uint32_t object_count,
    uint32_t entry_width,
    uint64_t data_len,
    uint64_t* output_offset,
    uint64_t* output_len);
extern "C" bool pdfium_rust_run_cross_ref_segment_entries(
    uint32_t entry_count,
    void* context,
    pdfium::rust::CrossRefSegmentCallback callback);
extern "C" bool pdfium_rust_cross_ref_field_width(int32_t value,
                                                   uint32_t* output);
extern "C" bool pdfium_rust_skip_pdf_spaces_and_comments(
    const uint8_t* data,
    size_t len,
    uint32_t position,
    uint32_t* output_position,
    uint8_t* output_byte);

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

std::optional<uint8_t> RustCrossRefEntryAction(
    uint8_t type_code,
    bool normal_offset_fits,
    uint32_t generation,
    bool archive_object_valid) {
  uint8_t output = 0;
  if (!pdfium_rust_cross_ref_entry_action(
          type_code, normal_offset_fits, generation, archive_object_valid,
          &output) ||
      output > 3) {
    return std::nullopt;
  }
  return output;
}

std::optional<CrossRefIndexPair> RustCrossRefIndexPair(int32_t start,
                                                        int32_t count) {
  CrossRefIndexPair result = {};
  if (!pdfium_rust_cross_ref_index_pair(start, count, &result.start,
                                        &result.count)) {
    return std::nullopt;
  }
  return result;
}

std::optional<CrossRefSegmentRange> RustCrossRefSegmentRange(
    uint32_t segment_index,
    uint32_t object_count,
    uint32_t entry_width,
    size_t data_len) {
  uint64_t offset = 0;
  uint64_t len = 0;
  if (!pdfium_rust_cross_ref_segment_range(
          segment_index, object_count, entry_width, data_len, &offset, &len) ||
      offset > std::numeric_limits<size_t>::max() ||
      len > std::numeric_limits<size_t>::max()) {
    return std::nullopt;
  }
  return CrossRefSegmentRange{.offset = static_cast<size_t>(offset),
                              .len = static_cast<size_t>(len)};
}

bool RunRustCrossRefSegmentEntries(uint32_t entry_count,
                                   void* context,
                                   CrossRefSegmentCallback callback) {
  return context && callback &&
         pdfium_rust_run_cross_ref_segment_entries(entry_count, context,
                                                   callback);
}

std::optional<uint32_t> RustCrossRefFieldWidth(int32_t value) {
  uint32_t output = 0;
  if (!pdfium_rust_cross_ref_field_width(value, &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<uint8_t> RustSkipPdfSpacesAndComments(
    pdfium::span<const uint8_t> input,
    uint32_t* position) {
  if (!position) {
    return std::nullopt;
  }

  uint32_t next_position = *position;
  uint8_t byte = 0;
  const bool has_byte = pdfium_rust_skip_pdf_spaces_and_comments(
      input.data(), input.size(), *position, &next_position, &byte);
  *position = next_position;
  if (!has_byte) {
    return std::nullopt;
  }
  return byte;
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
