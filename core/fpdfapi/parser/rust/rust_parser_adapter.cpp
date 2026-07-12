#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"

#include <limits>

#include "core/fxcrt/check.h"

namespace {

using RawCrossRefSnapshotCallback = bool (*)(void*,
                                             uint32_t,
                                             uint8_t,
                                             bool,
                                             uint16_t,
                                             int64_t,
                                             uint32_t,
                                             uint32_t);

extern "C" void* pdfium_rust_cross_ref_table_new();
extern "C" void pdfium_rust_cross_ref_table_destroy(void* state);
extern "C" bool pdfium_rust_cross_ref_table_add_compressed(
    void* state,
    uint32_t object_number,
    uint32_t archive_object_number,
    uint32_t archive_object_index);
extern "C" bool pdfium_rust_cross_ref_table_add_normal(void* state,
                                                       uint32_t object_number,
                                                       uint16_t generation,
                                                       bool is_object_stream,
                                                       int64_t position);
extern "C" bool pdfium_rust_cross_ref_table_set_free(void* state,
                                                     uint32_t object_number,
                                                     uint16_t generation);
extern "C" bool pdfium_rust_cross_ref_table_set_size(void* state,
                                                     uint32_t size);
extern "C" bool pdfium_rust_cross_ref_table_overlay(void* current, void* top);
extern "C" bool pdfium_rust_cross_ref_table_snapshot(
    const void* state,
    void* context,
    RawCrossRefSnapshotCallback callback);
extern "C" void* pdfium_rust_indirect_object_index_new();
extern "C" void pdfium_rust_indirect_object_index_destroy(void* state);
extern "C" bool pdfium_rust_indirect_object_index_lookup(const void* state,
                                                         uint32_t object_number,
                                                         uint8_t* output_status,
                                                         size_t* output_handle);
extern "C" bool pdfium_rust_indirect_object_index_reserve_parse(
    void* state,
    uint32_t object_number,
    uint8_t* output_status,
    size_t* output_handle);
extern "C" bool pdfium_rust_indirect_object_index_finish_parse(
    void* state,
    uint32_t object_number,
    size_t handle);
extern "C" bool pdfium_rust_indirect_object_index_cancel_parse(
    void* state,
    uint32_t object_number);
extern "C" bool pdfium_rust_indirect_object_index_add(
    void* state,
    size_t handle,
    uint32_t* output_object_number,
    bool* output_had_old_handle,
    size_t* output_old_handle);
extern "C" bool pdfium_rust_indirect_object_index_replace(
    void* state,
    uint32_t object_number,
    size_t handle,
    uint32_t new_generation,
    bool has_old_generation,
    uint32_t old_generation,
    bool* output_applied,
    bool* output_had_old_handle,
    size_t* output_old_handle);
extern "C" bool pdfium_rust_indirect_object_index_delete(void* state,
                                                         uint32_t object_number,
                                                         bool* output_deleted,
                                                         size_t* output_handle);
extern "C" bool pdfium_rust_indirect_object_index_contains_handle(
    const void* state,
    size_t handle);
extern "C" bool pdfium_rust_indirect_object_index_get_last(const void* state,
                                                           uint32_t* output);
extern "C" bool pdfium_rust_indirect_object_index_set_last(
    void* state,
    uint32_t object_number);
extern "C" bool pdfium_rust_indirect_object_index_snapshot(
    const void* state,
    void* context,
    pdfium::rust::RustIndirectObjectSnapshotCallback callback);
extern "C" void* pdfium_rust_pdf_number_new_default();
extern "C" void* pdfium_rust_pdf_number_new_signed(int32_t value);
extern "C" void* pdfium_rust_pdf_number_new_float(float value);
extern "C" void* pdfium_rust_pdf_number_new_string(const uint8_t* data,
                                                   size_t len);
extern "C" void pdfium_rust_pdf_number_destroy(void* state);
extern "C" bool pdfium_rust_pdf_number_is_integer(const void* state,
                                                  bool* output);
extern "C" bool pdfium_rust_pdf_number_get_signed(const void* state,
                                                  int32_t* output);
extern "C" bool pdfium_rust_pdf_number_get_float(const void* state,
                                                 float* output);
extern "C" bool pdfium_rust_pdf_number_set_string(void* state,
                                                  const uint8_t* data,
                                                  size_t len);
extern "C" void* pdfium_rust_pdf_boolean_new(bool value);
extern "C" void pdfium_rust_pdf_boolean_destroy(void* state);
extern "C" bool pdfium_rust_pdf_boolean_get(const void* state, bool* output);
extern "C" bool pdfium_rust_pdf_boolean_set_string(void* state,
                                                   const uint8_t* data,
                                                   size_t len);
extern "C" void* pdfium_rust_pdf_reference_new(uint32_t object_number);
extern "C" void pdfium_rust_pdf_reference_destroy(void* state);
extern "C" bool pdfium_rust_pdf_reference_get(const void* state,
                                              uint32_t* output);
extern "C" bool pdfium_rust_pdf_reference_set(void* state,
                                              uint32_t object_number);
extern "C" void* pdfium_rust_pdf_array_new();
extern "C" void pdfium_rust_pdf_array_destroy(void* state);
extern "C" bool pdfium_rust_pdf_array_len(const void* state, size_t* output);
extern "C" bool pdfium_rust_pdf_array_get(const void* state,
                                          size_t index,
                                          uintptr_t* output);
extern "C" bool pdfium_rust_pdf_array_append(void* state, uintptr_t handle);
extern "C" bool pdfium_rust_pdf_array_set(void* state,
                                          size_t index,
                                          uintptr_t handle,
                                          uintptr_t* old_handle);
extern "C" bool pdfium_rust_pdf_array_insert(void* state,
                                             size_t index,
                                             uintptr_t handle);
extern "C" bool pdfium_rust_pdf_array_remove(void* state,
                                             size_t index,
                                             uintptr_t* old_handle);
extern "C" bool pdfium_rust_pdf_array_clear(void* state);
extern "C" bool pdfium_rust_pdf_array_contains_handle(const void* state,
                                                      uintptr_t handle);
extern "C" void* pdfium_rust_pdf_dictionary_new();
extern "C" void pdfium_rust_pdf_dictionary_destroy(void* state);
extern "C" bool pdfium_rust_pdf_dictionary_len(const void* state,
                                               size_t* output);
extern "C" bool pdfium_rust_pdf_dictionary_get(const void* state,
                                               const uint8_t* key,
                                               size_t key_len,
                                               uintptr_t* output);
extern "C" bool pdfium_rust_pdf_dictionary_set(void* state,
                                               const uint8_t* key,
                                               size_t key_len,
                                               uintptr_t handle,
                                               uintptr_t* old_handle);
extern "C" bool pdfium_rust_pdf_dictionary_remove(void* state,
                                                  const uint8_t* key,
                                                  size_t key_len,
                                                  uintptr_t* old_handle);
extern "C" bool pdfium_rust_pdf_dictionary_snapshot(
    const void* state,
    void* context,
    pdfium::rust::RustPdfDictionarySnapshotCallback callback);
extern "C" bool pdfium_rust_pdf_dictionary_contains_handle(const void* state,
                                                           uintptr_t handle);
extern "C" void* pdfium_rust_pdf_string_new(const uint8_t* data,
                                            size_t len,
                                            bool output_is_hex);
extern "C" void pdfium_rust_pdf_string_destroy(void* state);
extern "C" bool pdfium_rust_pdf_string_is_hex(const void* state, bool* output);
extern "C" bool pdfium_rust_pdf_string_equals(const void* state,
                                              const uint8_t* data,
                                              size_t len);
extern "C" bool pdfium_rust_pdf_string_set(void* state,
                                           const uint8_t* data,
                                           size_t len);
extern "C" void* pdfium_rust_pdf_stream_data_new(const uint8_t* data,
                                                 size_t len);
extern "C" void pdfium_rust_pdf_stream_data_destroy(void* state);
extern "C" bool pdfium_rust_pdf_stream_data_span(const void* state,
                                                 const uint8_t** data,
                                                 size_t* len);
extern "C" void* pdfium_rust_document_page_index_new();
extern "C" void pdfium_rust_document_page_index_destroy(void* state);
extern "C" bool pdfium_rust_document_page_index_len(const void* state,
                                                    size_t* output);
extern "C" bool pdfium_rust_document_page_index_resize(void* state, size_t len);
extern "C" bool pdfium_rust_document_page_index_get(const void* state,
                                                    size_t index,
                                                    uint32_t* output);
extern "C" bool pdfium_rust_document_page_index_set(void* state,
                                                    size_t index,
                                                    uint32_t object_number);
extern "C" bool pdfium_rust_document_page_index_insert(void* state,
                                                       size_t index,
                                                       uint32_t object_number);
extern "C" bool pdfium_rust_document_page_index_remove(void* state,
                                                       size_t index);
extern "C" bool pdfium_rust_document_page_index_contains(
    const void* state,
    uint32_t object_number);
extern "C" void* pdfium_rust_document_page_traversal_new();
extern "C" void pdfium_rust_document_page_traversal_free(void* state);
extern "C" bool pdfium_rust_document_page_traversal_clear(
    void* state,
    void* context,
    pdfium::rust::RustDocumentPageTraversalReleaseCallback release);
extern "C" bool pdfium_rust_document_page_traversal_reset(
    void* state,
    uintptr_t root_handle,
    void* context,
    pdfium::rust::RustDocumentPageTraversalRetainCallback retain,
    pdfium::rust::RustDocumentPageTraversalReleaseCallback release);
extern "C" bool pdfium_rust_document_page_traversal_run(
    void* state,
    int32_t page_index,
    void* context,
    pdfium::rust::RustDocumentPageTraversalDescribeCallback describe,
    pdfium::rust::RustDocumentPageTraversalChildCallback child,
    pdfium::rust::RustDocumentPageTraversalCacheCallback cache,
    pdfium::rust::RustDocumentPageTraversalSelectCallback select,
    pdfium::rust::RustDocumentPageTraversalRetainCallback retain,
    pdfium::rust::RustDocumentPageTraversalReleaseCallback release,
    bool* found);
extern "C" bool pdfium_rust_document_move_page_plan(const int32_t* page_indices,
                                                    size_t len,
                                                    size_t num_pages,
                                                    int32_t destination,
                                                    int32_t* deletion_order);
extern "C" bool pdfium_rust_document_count_pages(
    uintptr_t root_handle,
    void* context,
    pdfium::rust::RustDocumentPageDescribeCallback describe,
    pdfium::rust::RustDocumentPageChildCallback child,
    pdfium::rust::RustDocumentPageNormalizeCallback normalize,
    pdfium::rust::RustDocumentPageSetCountCallback set_count,
    int32_t* output);
extern "C" bool pdfium_rust_document_find_page_index(
    uintptr_t root_handle,
    uint32_t target_object_number,
    uint32_t initial_skip_count,
    void* context,
    pdfium::rust::RustDocumentPageFindDescribeCallback describe,
    pdfium::rust::RustDocumentPageFindChildCallback child,
    int32_t* output);
extern "C" bool pdfium_rust_sdk_parse_page_range(const uint8_t* input,
                                                 size_t input_len,
                                                 uint32_t page_count,
                                                 uint32_t* output,
                                                 size_t output_capacity,
                                                 size_t* output_len);
extern "C" bool pdfium_rust_sdk_nul_terminate(const uint8_t* input,
                                              size_t input_len,
                                              uint8_t* output,
                                              size_t output_capacity,
                                              size_t* required_len);
extern "C" bool pdfium_rust_page_label_number(int32_t number,
                                              const uint8_t* style,
                                              size_t style_len,
                                              uint8_t* output,
                                              size_t output_capacity,
                                              size_t* output_len);
extern "C" void* pdfium_rust_redaction_plan_new(
    bool has_rects,
    size_t rect_count,
    size_t object_count,
    void* context,
    pdfium::rust::RustRedactionRectCallback get_rect,
    pdfium::rust::RustRedactionObjectCallback get_object);
extern "C" void pdfium_rust_redaction_plan_free(void* state);
extern "C" bool pdfium_rust_redaction_plan_status(const void* state,
                                                  int32_t* output);
extern "C" size_t pdfium_rust_redaction_plan_count(const void* state);
extern "C" bool pdfium_rust_redaction_plan_index(const void* state,
                                                 size_t index,
                                                 size_t* output);
extern "C" bool pdfium_rust_page_object_insert_plan(
    size_t index,
    size_t object_count,
    int32_t content_stream,
    void* context,
    pdfium::rust::RustPageObjectStreamCallback get_neighbor_stream,
    bool* allowed,
    int32_t* planned_content_stream,
    bool* mark_dirty);
extern "C" bool pdfium_rust_page_object_remove_plan(
    size_t object_count,
    uintptr_t target_handle,
    void* context,
    pdfium::rust::RustPageObjectDescribeCallback describe,
    bool* found,
    size_t* index,
    int32_t* content_stream);
extern "C" bool pdfium_rust_page_object_active_update(bool current,
                                                      bool requested,
                                                      bool* active,
                                                      bool* mark_dirty);
extern "C" bool pdfium_rust_page_object_active_count(
    size_t object_count,
    void* context,
    pdfium::rust::RustPageObjectActiveCallback get_active,
    size_t* output);
extern "C" bool pdfium_rust_page_object_matrix_route(uint8_t object_type,
                                                     uint8_t* output);
extern "C" bool pdfium_rust_page_object_matrix_dirty(uint8_t object_type,
                                                     const float* original,
                                                     const float* replacement,
                                                     bool* output);
extern "C" bool pdfium_rust_page_object_rotated_bounds(uint8_t object_type,
                                                       const float* matrix,
                                                       const float* bounds,
                                                       float* output);
extern "C" bool pdfium_rust_page_annotation_transform_rect(const float* matrix,
                                                           const float* rect,
                                                           float* output);
extern "C" int32_t pdfium_rust_page_rotation_degrees(int32_t rotation);
extern "C" uint8_t pdfium_rust_public_action_type(uint8_t internal_type);
extern "C" uint8_t pdfium_rust_public_action_capabilities(uint8_t public_type);
extern "C" bool pdfium_rust_public_bookmark_color_is_valid(float red,
                                                            float green,
                                                            float blue);
extern "C" uint8_t pdfium_rust_public_destination_source(bool has_direct,
                                                          bool has_action);
extern "C" uint8_t pdfium_rust_public_destination_zoom_mode(const uint8_t* mode,
                                                            size_t mode_len);
extern "C" size_t pdfium_rust_public_destination_num_params(uint8_t zoom_mode,
                                                            size_t array_size);
extern "C" bool pdfium_rust_public_destination_xyz_plan(bool array_present,
                                                        size_t array_size,
                                                        bool is_xyz,
                                                        bool has_x_input,
                                                        float x_input,
                                                        bool has_y_input,
                                                        float y_input,
                                                        bool has_zoom_input,
                                                        float zoom_input,
                                                        bool* valid,
                                                        bool* has_x,
                                                        bool* has_y,
                                                        bool* has_zoom,
                                                        float* x,
                                                        float* y,
                                                        float* zoom);
extern "C" bool pdfium_rust_find_bookmark(
    void* context,
    pdfium::rust::RustBookmarkMatchCallback matches_title,
    pdfium::rust::RustBookmarkNavigateCallback first_child,
    pdfium::rust::RustBookmarkNavigateCallback next_sibling,
    uintptr_t* output);
extern "C" bool pdfium_rust_find_next_link(
    int32_t start_position,
    size_t annotation_count,
    void* context,
    pdfium::rust::RustLinkEnumerationCallback is_link,
    bool* found,
    size_t* index);
extern "C" bool pdfium_rust_number_tree_lookup(
    uintptr_t root,
    int32_t number,
    void* context,
    pdfium::rust::RustNumberTreeDescribeCallback describe,
    pdfium::rust::RustNumberTreeNumberCallback read_number,
    pdfium::rust::RustNumberTreeKidCallback read_kid,
    uintptr_t* output);
extern "C" bool pdfium_rust_number_tree_lower_bound(
    uintptr_t root,
    int32_t number,
    void* context,
    pdfium::rust::RustNumberTreeDescribeCallback describe,
    pdfium::rust::RustNumberTreeNumberCallback read_number,
    pdfium::rust::RustNumberTreeKidCallback read_kid,
    bool* found,
    int32_t* key,
    uintptr_t* value);
extern "C" bool pdfium_rust_destination_page_index(
    uint8_t target_kind,
    int32_t direct_page,
    uint32_t object_number,
    void* context,
    pdfium::rust::RustDestinationPageCallback lookup_page,
    int32_t* output);
extern "C" bool pdfium_rust_name_tree_count(
    uintptr_t root,
    void* context,
    pdfium::rust::RustNameTreeDescribeCallback describe,
    pdfium::rust::RustNameTreeKidCallback read_kid,
    size_t* output);
extern "C" bool pdfium_rust_name_tree_find_index(
    uintptr_t root,
    size_t target,
    void* context,
    pdfium::rust::RustNameTreeDescribeCallback describe,
    pdfium::rust::RustNameTreeKidCallback read_kid,
    bool* found,
    uintptr_t* node,
    size_t* pair_index);
extern "C" bool pdfium_rust_name_tree_plan_insertion(
    uintptr_t root,
    void* context,
    pdfium::rust::RustNameTreeSearchDescribeCallback describe,
    pdfium::rust::RustNameTreeSearchTokenCallback read_token,
    pdfium::rust::RustNameTreeSearchLimitsCallback compare_limits,
    pdfium::rust::RustNameTreeSearchNameCallback read_name,
    pdfium::rust::RustNameTreeSearchKidCallback read_kid,
    bool* duplicate,
    uintptr_t* node,
    size_t* pair_index);
extern "C" bool pdfium_rust_name_tree_lookup(
    uintptr_t root,
    void* context,
    pdfium::rust::RustNameTreeSearchDescribeCallback describe,
    pdfium::rust::RustNameTreeSearchTokenCallback read_token,
    pdfium::rust::RustNameTreeSearchLimitsCallback compare_limits,
    pdfium::rust::RustNameTreeSearchNameCallback read_name,
    pdfium::rust::RustNameTreeSearchKidCallback read_kid,
    uintptr_t* output);
extern "C" bool pdfium_rust_find_link_at_point(
    size_t link_count,
    float x,
    float y,
    void* context,
    pdfium::rust::RustLinkRectCallback read_rect,
    bool* found,
    size_t* index);
extern "C" bool pdfium_rust_document_page_mutation_path(
    uintptr_t root_handle,
    int32_t pages_to_go,
    void* context,
    pdfium::rust::RustDocumentPageMutationDescribeCallback describe,
    pdfium::rust::RustDocumentPageMutationChildCallback child,
    size_t* output,
    size_t output_capacity,
    size_t* output_len);

extern "C" bool pdfium_rust_read_big_endian_var_int(const uint8_t* data,
                                                    size_t len,
                                                    uint32_t* output);
extern "C" bool pdfium_rust_cross_ref_object_type(uint32_t type_code,
                                                  uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_entry_type(bool has_type_field,
                                                 uint32_t type_code,
                                                 uint8_t* output);
extern "C" bool pdfium_rust_cross_ref_entry_action(uint8_t type_code,
                                                   bool normal_offset_fits,
                                                   uint32_t generation,
                                                   bool archive_object_valid,
                                                   uint8_t* output);
extern "C" bool pdfium_rust_run_cross_ref_entry_mutation(
    uint8_t type_code,
    bool normal_offset_fits,
    uint32_t generation,
    bool archive_object_valid,
    void* context,
    pdfium::rust::CrossRefMutationCallback callback);
extern "C" bool pdfium_rust_cross_ref_index_pair(int32_t start,
                                                 int32_t count,
                                                 uint32_t* output_start,
                                                 uint32_t* output_count);
extern "C" bool pdfium_rust_cross_ref_segment_range(uint32_t segment_index,
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
extern "C" bool pdfium_rust_read_cross_ref_entry(const uint8_t* data,
                                                 size_t len,
                                                 uint32_t first_width,
                                                 uint32_t second_width,
                                                 uint32_t third_width,
                                                 uint32_t* output_first,
                                                 uint32_t* output_second,
                                                 uint32_t* output_third);
extern "C" bool pdfium_rust_skip_pdf_spaces_and_comments(
    const uint8_t* data,
    size_t len,
    uint32_t position,
    uint32_t* output_position,
    uint8_t* output_byte);
extern "C" bool pdfium_rust_scan_pdf_token(const uint8_t* data,
                                           size_t len,
                                           uint32_t position,
                                           uint32_t* output_position,
                                           bool* output_has_word,
                                           uint32_t* output_start,
                                           uint32_t* output_len);

thread_local bool g_use_rust_parser_candidate = true;

}  // namespace

namespace pdfium::rust {

RustCrossRefTable::RustCrossRefTable()
    : state_(pdfium_rust_cross_ref_table_new()) {}

RustCrossRefTable::~RustCrossRefTable() {
  pdfium_rust_cross_ref_table_destroy(state_);
}

bool RustCrossRefTable::AddCompressed(uint32_t object_number,
                                      uint32_t archive_object_number,
                                      uint32_t archive_object_index) {
  return pdfium_rust_cross_ref_table_add_compressed(
      state_, object_number, archive_object_number, archive_object_index);
}

bool RustCrossRefTable::AddNormal(uint32_t object_number,
                                  uint16_t generation,
                                  bool is_object_stream,
                                  int64_t position) {
  return pdfium_rust_cross_ref_table_add_normal(
      state_, object_number, generation, is_object_stream, position);
}

bool RustCrossRefTable::SetFree(uint32_t object_number, uint16_t generation) {
  return pdfium_rust_cross_ref_table_set_free(state_, object_number,
                                              generation);
}

bool RustCrossRefTable::SetSize(uint32_t size) {
  return pdfium_rust_cross_ref_table_set_size(state_, size);
}

bool RustCrossRefTable::OverlayFrom(RustCrossRefTable* top) {
  return top && pdfium_rust_cross_ref_table_overlay(state_, top->state_);
}

bool RustCrossRefTable::Snapshot(void* context,
                                 RustCrossRefSnapshotCallback callback) const {
  if (!context || !callback) {
    return false;
  }
  struct SnapshotContext {
    void* outer_context;
    RustCrossRefSnapshotCallback outer_callback;
  } snapshot_context = {context, callback};
  const auto forward = [](void* raw_context, uint32_t object_number,
                          uint8_t type, bool is_object_stream,
                          uint16_t generation, int64_t position,
                          uint32_t archive_object_number,
                          uint32_t archive_object_index) -> bool {
    auto* context = static_cast<SnapshotContext*>(raw_context);
    const RustCrossRefObjectInfo info = {
        .type = type,
        .is_object_stream = is_object_stream,
        .generation = generation,
        .position = position,
        .archive_object_number = archive_object_number,
        .archive_object_index = archive_object_index,
    };
    return context->outer_callback(context->outer_context, object_number, info);
  };
  return pdfium_rust_cross_ref_table_snapshot(state_, &snapshot_context,
                                              forward);
}

RustIndirectObjectIndex::RustIndirectObjectIndex()
    : state_(pdfium_rust_indirect_object_index_new()) {}

RustIndirectObjectIndex::~RustIndirectObjectIndex() {
  pdfium_rust_indirect_object_index_destroy(state_);
}

std::optional<RustIndirectObjectLookup> RustIndirectObjectIndex::Lookup(
    uint32_t object_number) const {
  RustIndirectObjectLookup result = {};
  if (!pdfium_rust_indirect_object_index_lookup(
          state_, object_number, &result.status, &result.handle) ||
      result.status > 2 || (result.status == 2 && result.handle == 0)) {
    return std::nullopt;
  }
  return result;
}

std::optional<RustIndirectObjectLookup> RustIndirectObjectIndex::ReserveParse(
    uint32_t object_number) {
  RustIndirectObjectLookup result = {};
  if (!pdfium_rust_indirect_object_index_reserve_parse(
          state_, object_number, &result.status, &result.handle) ||
      result.status > 2 || (result.status == 2 && result.handle == 0)) {
    return std::nullopt;
  }
  return result;
}

bool RustIndirectObjectIndex::FinishParse(uint32_t object_number,
                                          uintptr_t handle) {
  return pdfium_rust_indirect_object_index_finish_parse(state_, object_number,
                                                        handle);
}

bool RustIndirectObjectIndex::CancelParse(uint32_t object_number) {
  return pdfium_rust_indirect_object_index_cancel_parse(state_, object_number);
}

std::optional<RustIndirectObjectAddResult> RustIndirectObjectIndex::Add(
    uintptr_t handle) {
  uint32_t object_number = 0;
  bool had_old_handle = false;
  uintptr_t old_handle = 0;
  if (!pdfium_rust_indirect_object_index_add(state_, handle, &object_number,
                                             &had_old_handle, &old_handle) ||
      (had_old_handle && old_handle == 0)) {
    return std::nullopt;
  }
  return RustIndirectObjectAddResult{
      .object_number = object_number,
      .old_handle = had_old_handle ? std::optional(old_handle) : std::nullopt,
  };
}

std::optional<RustIndirectObjectReplaceResult> RustIndirectObjectIndex::Replace(
    uint32_t object_number,
    uintptr_t handle,
    uint32_t new_generation,
    std::optional<uint32_t> old_generation) {
  bool applied = false;
  bool had_old_handle = false;
  uintptr_t old_handle = 0;
  if (!pdfium_rust_indirect_object_index_replace(
          state_, object_number, handle, new_generation,
          old_generation.has_value(), old_generation.value_or(0), &applied,
          &had_old_handle, &old_handle) ||
      (had_old_handle && old_handle == 0)) {
    return std::nullopt;
  }
  return RustIndirectObjectReplaceResult{
      .applied = applied,
      .old_handle = had_old_handle ? std::optional(old_handle) : std::nullopt,
  };
}

std::optional<std::optional<uintptr_t>> RustIndirectObjectIndex::Delete(
    uint32_t object_number) {
  bool deleted = false;
  uintptr_t handle = 0;
  if (!pdfium_rust_indirect_object_index_delete(state_, object_number, &deleted,
                                                &handle) ||
      (deleted && handle == 0)) {
    return std::nullopt;
  }
  return deleted ? std::optional<std::optional<uintptr_t>>(handle)
                 : std::optional<std::optional<uintptr_t>>(std::nullopt);
}

bool RustIndirectObjectIndex::ContainsHandle(uintptr_t handle) const {
  return pdfium_rust_indirect_object_index_contains_handle(state_, handle);
}

std::optional<uint32_t> RustIndirectObjectIndex::GetLastObjectNumber() const {
  uint32_t result = 0;
  if (!pdfium_rust_indirect_object_index_get_last(state_, &result)) {
    return std::nullopt;
  }
  return result;
}

bool RustIndirectObjectIndex::SetLastObjectNumber(uint32_t object_number) {
  return pdfium_rust_indirect_object_index_set_last(state_, object_number);
}

bool RustIndirectObjectIndex::Snapshot(
    void* context,
    RustIndirectObjectSnapshotCallback callback) const {
  return context && callback &&
         pdfium_rust_indirect_object_index_snapshot(state_, context, callback);
}

RustPdfNumber::RustPdfNumber(void* state) : state_(state) {
  CHECK(state_);
}

RustPdfNumber::RustPdfNumber()
    : RustPdfNumber(pdfium_rust_pdf_number_new_default()) {}

RustPdfNumber::RustPdfNumber(int32_t value)
    : RustPdfNumber(pdfium_rust_pdf_number_new_signed(value)) {}

RustPdfNumber::RustPdfNumber(float value)
    : RustPdfNumber(pdfium_rust_pdf_number_new_float(value)) {}

RustPdfNumber::RustPdfNumber(pdfium::span<const uint8_t> value)
    : RustPdfNumber(
          pdfium_rust_pdf_number_new_string(value.data(), value.size())) {}

RustPdfNumber::~RustPdfNumber() {
  pdfium_rust_pdf_number_destroy(state_);
}

bool RustPdfNumber::IsInteger() const {
  bool result = false;
  CHECK(pdfium_rust_pdf_number_is_integer(state_, &result));
  return result;
}

int32_t RustPdfNumber::GetSigned() const {
  int32_t result = 0;
  CHECK(pdfium_rust_pdf_number_get_signed(state_, &result));
  return result;
}

float RustPdfNumber::GetFloat() const {
  float result = 0;
  CHECK(pdfium_rust_pdf_number_get_float(state_, &result));
  return result;
}

bool RustPdfNumber::SetString(pdfium::span<const uint8_t> value) {
  return pdfium_rust_pdf_number_set_string(state_, value.data(), value.size());
}

RustPdfBoolean::RustPdfBoolean(bool value)
    : state_(pdfium_rust_pdf_boolean_new(value)) {
  CHECK(state_);
}

RustPdfBoolean::~RustPdfBoolean() {
  pdfium_rust_pdf_boolean_destroy(state_);
}

bool RustPdfBoolean::Get() const {
  bool result = false;
  CHECK(pdfium_rust_pdf_boolean_get(state_, &result));
  return result;
}

bool RustPdfBoolean::SetString(pdfium::span<const uint8_t> value) {
  return pdfium_rust_pdf_boolean_set_string(state_, value.data(), value.size());
}

RustPdfReference::RustPdfReference(uint32_t object_number)
    : state_(pdfium_rust_pdf_reference_new(object_number)) {
  CHECK(state_);
}

RustPdfReference::~RustPdfReference() {
  pdfium_rust_pdf_reference_destroy(state_);
}

uint32_t RustPdfReference::GetObjectNumber() const {
  uint32_t result = 0;
  CHECK(pdfium_rust_pdf_reference_get(state_, &result));
  return result;
}

bool RustPdfReference::SetObjectNumber(uint32_t object_number) {
  return pdfium_rust_pdf_reference_set(state_, object_number);
}

RustPdfArray::RustPdfArray() : state_(pdfium_rust_pdf_array_new()) {
  CHECK(state_);
}

RustPdfArray::~RustPdfArray() {
  pdfium_rust_pdf_array_destroy(state_);
}

size_t RustPdfArray::size() const {
  size_t result = 0;
  CHECK(pdfium_rust_pdf_array_len(state_, &result));
  return result;
}

std::optional<uintptr_t> RustPdfArray::Get(size_t index) const {
  uintptr_t result = 0;
  if (!pdfium_rust_pdf_array_get(state_, index, &result)) {
    return std::nullopt;
  }
  return result;
}

bool RustPdfArray::Append(uintptr_t handle) {
  return pdfium_rust_pdf_array_append(state_, handle);
}

std::optional<uintptr_t> RustPdfArray::Set(size_t index, uintptr_t handle) {
  uintptr_t old_handle = 0;
  if (!pdfium_rust_pdf_array_set(state_, index, handle, &old_handle)) {
    return std::nullopt;
  }
  return old_handle;
}

bool RustPdfArray::Insert(size_t index, uintptr_t handle) {
  return pdfium_rust_pdf_array_insert(state_, index, handle);
}

std::optional<uintptr_t> RustPdfArray::Remove(size_t index) {
  uintptr_t old_handle = 0;
  if (!pdfium_rust_pdf_array_remove(state_, index, &old_handle)) {
    return std::nullopt;
  }
  return old_handle;
}

bool RustPdfArray::Clear() {
  return pdfium_rust_pdf_array_clear(state_);
}

bool RustPdfArray::ContainsHandle(uintptr_t handle) const {
  return pdfium_rust_pdf_array_contains_handle(state_, handle);
}

RustPdfDictionary::RustPdfDictionary()
    : state_(pdfium_rust_pdf_dictionary_new()) {
  CHECK(state_);
}

RustPdfDictionary::~RustPdfDictionary() {
  pdfium_rust_pdf_dictionary_destroy(state_);
}

size_t RustPdfDictionary::size() const {
  size_t result = 0;
  CHECK(pdfium_rust_pdf_dictionary_len(state_, &result));
  return result;
}

std::optional<uintptr_t> RustPdfDictionary::Get(
    pdfium::span<const uint8_t> key) const {
  uintptr_t result = 0;
  if (!pdfium_rust_pdf_dictionary_get(state_, key.data(), key.size(),
                                      &result)) {
    return std::nullopt;
  }
  return result;
}

std::optional<uintptr_t> RustPdfDictionary::Set(pdfium::span<const uint8_t> key,
                                                uintptr_t handle) {
  uintptr_t old_handle = 0;
  CHECK(pdfium_rust_pdf_dictionary_set(state_, key.data(), key.size(), handle,
                                       &old_handle));
  return old_handle ? std::optional<uintptr_t>(old_handle) : std::nullopt;
}

std::optional<uintptr_t> RustPdfDictionary::Remove(
    pdfium::span<const uint8_t> key) {
  uintptr_t old_handle = 0;
  if (!pdfium_rust_pdf_dictionary_remove(state_, key.data(), key.size(),
                                         &old_handle)) {
    return std::nullopt;
  }
  return old_handle;
}

bool RustPdfDictionary::ContainsHandle(uintptr_t handle) const {
  return pdfium_rust_pdf_dictionary_contains_handle(state_, handle);
}

bool RustPdfDictionary::Snapshot(
    void* context,
    RustPdfDictionarySnapshotCallback callback) const {
  return callback &&
         pdfium_rust_pdf_dictionary_snapshot(state_, context, callback);
}

RustPdfString::RustPdfString(pdfium::span<const uint8_t> value,
                             bool output_is_hex)
    : state_(pdfium_rust_pdf_string_new(value.data(),
                                        value.size(),
                                        output_is_hex)) {
  CHECK(state_);
}

RustPdfString::~RustPdfString() {
  pdfium_rust_pdf_string_destroy(state_);
}

bool RustPdfString::IsHex() const {
  bool result = false;
  CHECK(pdfium_rust_pdf_string_is_hex(state_, &result));
  return result;
}

bool RustPdfString::Equals(pdfium::span<const uint8_t> value) const {
  return pdfium_rust_pdf_string_equals(state_, value.data(), value.size());
}

bool RustPdfString::Set(pdfium::span<const uint8_t> value) {
  return pdfium_rust_pdf_string_set(state_, value.data(), value.size());
}

RustPdfStreamData::RustPdfStreamData(pdfium::span<const uint8_t> value)
    : state_(pdfium_rust_pdf_stream_data_new(value.data(), value.size())) {
  CHECK(state_);
}

RustPdfStreamData::~RustPdfStreamData() {
  pdfium_rust_pdf_stream_data_destroy(state_);
}

pdfium::span<const uint8_t> RustPdfStreamData::GetSpan() const {
  const uint8_t* data = nullptr;
  size_t len = 0;
  CHECK(pdfium_rust_pdf_stream_data_span(state_, &data, &len));
  return pdfium::span(data, len);
}

RustDocumentPageIndex::RustDocumentPageIndex()
    : state_(pdfium_rust_document_page_index_new()) {
  CHECK(state_);
}

RustDocumentPageIndex::~RustDocumentPageIndex() {
  pdfium_rust_document_page_index_destroy(state_);
}

size_t RustDocumentPageIndex::size() const {
  size_t result = 0;
  CHECK(pdfium_rust_document_page_index_len(state_, &result));
  return result;
}

bool RustDocumentPageIndex::Resize(size_t size) {
  return pdfium_rust_document_page_index_resize(state_, size);
}

std::optional<uint32_t> RustDocumentPageIndex::Get(size_t index) const {
  uint32_t result = 0;
  if (!pdfium_rust_document_page_index_get(state_, index, &result)) {
    return std::nullopt;
  }
  return result;
}

bool RustDocumentPageIndex::Set(size_t index, uint32_t object_number) {
  return pdfium_rust_document_page_index_set(state_, index, object_number);
}

bool RustDocumentPageIndex::Insert(size_t index, uint32_t object_number) {
  return pdfium_rust_document_page_index_insert(state_, index, object_number);
}

bool RustDocumentPageIndex::Remove(size_t index) {
  return pdfium_rust_document_page_index_remove(state_, index);
}

bool RustDocumentPageIndex::Contains(uint32_t object_number) const {
  return pdfium_rust_document_page_index_contains(state_, object_number);
}

RustDocumentPageTraversal::RustDocumentPageTraversal(
    void* context,
    RustDocumentPageTraversalRetainCallback retain,
    RustDocumentPageTraversalReleaseCallback release,
    RustDocumentPageTraversalDescribeCallback describe,
    RustDocumentPageTraversalChildCallback child,
    RustDocumentPageTraversalCacheCallback cache,
    RustDocumentPageTraversalSelectCallback select)
    : state_(pdfium_rust_document_page_traversal_new()),
      context_(context),
      retain_(retain),
      release_(release),
      describe_(describe),
      child_(child),
      cache_(cache),
      select_(select) {
  CHECK(state_);
}

RustDocumentPageTraversal::~RustDocumentPageTraversal() {
  CHECK(Clear());
  pdfium_rust_document_page_traversal_free(state_);
}

bool RustDocumentPageTraversal::Reset(uintptr_t root_handle) {
  return pdfium_rust_document_page_traversal_reset(state_, root_handle,
                                                   context_, retain_, release_);
}

bool RustDocumentPageTraversal::Clear() {
  return pdfium_rust_document_page_traversal_clear(state_, context_, release_);
}

std::optional<bool> RustDocumentPageTraversal::Traverse(int page_index) {
  bool found = false;
  if (!pdfium_rust_document_page_traversal_run(
          state_, page_index, context_, describe_, child_, cache_, select_,
          retain_, release_, &found)) {
    return std::nullopt;
  }
  return found;
}

std::optional<std::vector<int>> RustDocumentMovePageDeletionOrder(
    pdfium::span<const int> page_indices,
    size_t num_pages,
    int destination) {
  std::vector<int> result(page_indices.size());
  if (!pdfium_rust_document_move_page_plan(page_indices.data(),
                                           page_indices.size(), num_pages,
                                           destination, result.data())) {
    return std::nullopt;
  }
  return result;
}

std::optional<int> RustDocumentCountPages(
    uintptr_t root_handle,
    void* context,
    RustDocumentPageDescribeCallback describe,
    RustDocumentPageChildCallback child,
    RustDocumentPageNormalizeCallback normalize,
    RustDocumentPageSetCountCallback set_count) {
  int32_t result = 0;
  if (!pdfium_rust_document_count_pages(root_handle, context, describe, child,
                                        normalize, set_count, &result)) {
    return std::nullopt;
  }
  return result;
}

std::optional<int> RustDocumentFindPageIndex(
    uintptr_t root_handle,
    uint32_t target_object_number,
    uint32_t initial_skip_count,
    void* context,
    RustDocumentPageFindDescribeCallback describe,
    RustDocumentPageFindChildCallback child) {
  int32_t result = -1;
  if (!pdfium_rust_document_find_page_index(root_handle, target_object_number,
                                            initial_skip_count, context,
                                            describe, child, &result)) {
    return std::nullopt;
  }
  return result;
}

std::optional<std::vector<uint32_t>> RustSdkParsePageRange(
    pdfium::span<const uint8_t> input,
    uint32_t page_count) {
  size_t output_len = 0;
  if (!pdfium_rust_sdk_parse_page_range(input.data(), input.size(), page_count,
                                        nullptr, 0, &output_len)) {
    return std::nullopt;
  }
  std::vector<uint32_t> result(output_len);
  if (output_len != 0 && !pdfium_rust_sdk_parse_page_range(
                             input.data(), input.size(), page_count,
                             result.data(), result.size(), &output_len)) {
    return std::nullopt;
  }
  return result;
}

std::optional<size_t> RustSdkNulTerminate(pdfium::span<const uint8_t> input,
                                          pdfium::span<char> output) {
  size_t required_len = 0;
  if (!pdfium_rust_sdk_nul_terminate(input.data(), input.size(),
                                     reinterpret_cast<uint8_t*>(output.data()),
                                     output.size(), &required_len)) {
    return std::nullopt;
  }
  return required_len;
}

RustRedactionPlan::RustRedactionPlan(bool has_rects,
                                     size_t rect_count,
                                     size_t object_count,
                                     void* context,
                                     RustRedactionRectCallback get_rect,
                                     RustRedactionObjectCallback get_object)
    : state_(pdfium_rust_redaction_plan_new(has_rects,
                                            rect_count,
                                            object_count,
                                            context,
                                            get_rect,
                                            get_object)) {}

RustRedactionPlan::~RustRedactionPlan() {
  pdfium_rust_redaction_plan_free(state_);
}

std::optional<int> RustRedactionPlan::status() const {
  int32_t output = 0;
  if (!pdfium_rust_redaction_plan_status(state_, &output)) {
    return std::nullopt;
  }
  return output;
}

size_t RustRedactionPlan::size() const {
  return pdfium_rust_redaction_plan_count(state_);
}

std::optional<size_t> RustRedactionPlan::GetIndex(size_t index) const {
  size_t output = 0;
  if (!pdfium_rust_redaction_plan_index(state_, index, &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<RustPageObjectInsertPlan> RustPlanPageObjectInsert(
    size_t index,
    size_t object_count,
    int32_t content_stream,
    void* context,
    RustPageObjectStreamCallback get_neighbor_stream) {
  RustPageObjectInsertPlan plan = {};
  if (!pdfium_rust_page_object_insert_plan(
          index, object_count, content_stream, context, get_neighbor_stream,
          &plan.allowed, &plan.content_stream, &plan.mark_dirty)) {
    return std::nullopt;
  }
  return plan;
}

std::optional<RustPageObjectRemovePlan> RustPlanPageObjectRemove(
    size_t object_count,
    uintptr_t target_handle,
    void* context,
    RustPageObjectDescribeCallback describe) {
  RustPageObjectRemovePlan plan = {};
  if (!pdfium_rust_page_object_remove_plan(object_count, target_handle, context,
                                           describe, &plan.found, &plan.index,
                                           &plan.content_stream)) {
    return std::nullopt;
  }
  return plan;
}

std::optional<RustPageObjectActiveUpdate> RustPlanPageObjectActiveUpdate(
    bool current,
    bool requested) {
  RustPageObjectActiveUpdate update = {};
  if (!pdfium_rust_page_object_active_update(current, requested, &update.active,
                                             &update.mark_dirty)) {
    return std::nullopt;
  }
  return update;
}

std::optional<size_t> RustCountActivePageObjects(
    size_t object_count,
    void* context,
    RustPageObjectActiveCallback get_active) {
  size_t output = 0;
  if (!pdfium_rust_page_object_active_count(object_count, context, get_active,
                                            &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<uint8_t> RustPageObjectMatrixRoute(uint8_t object_type) {
  uint8_t output = 0;
  if (!pdfium_rust_page_object_matrix_route(object_type, &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<bool> RustPageObjectMatrixDirty(
    uint8_t object_type,
    const RustPageObjectMatrix& original,
    const RustPageObjectMatrix& replacement) {
  bool output = false;
  if (!pdfium_rust_page_object_matrix_dirty(object_type, original.values.data(),
                                            replacement.values.data(),
                                            &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<std::array<float, 8>> RustPageObjectRotatedBounds(
    uint8_t object_type,
    const RustPageObjectMatrix& matrix,
    const std::array<float, 4>& bounds) {
  std::array<float, 8> output = {};
  if (!pdfium_rust_page_object_rotated_bounds(object_type, matrix.values.data(),
                                              bounds.data(), output.data())) {
    return std::nullopt;
  }
  return output;
}

std::optional<std::array<float, 4>> RustTransformPageAnnotationRect(
    const RustPageObjectMatrix& matrix,
    const std::array<float, 4>& rect) {
  std::array<float, 4> output = {};
  if (!pdfium_rust_page_annotation_transform_rect(matrix.values.data(),
                                                  rect.data(), output.data())) {
    return std::nullopt;
  }
  return output;
}

int RustPageRotationDegrees(int rotation) {
  return pdfium_rust_page_rotation_degrees(rotation);
}

uint8_t RustPublicActionType(uint8_t internal_type) {
  return pdfium_rust_public_action_type(internal_type);
}

bool RustPublicActionAllowsDestination(uint8_t public_type) {
  return pdfium_rust_public_action_capabilities(public_type) & 1;
}

bool RustPublicActionAllowsFile(uint8_t public_type) {
  return pdfium_rust_public_action_capabilities(public_type) & 2;
}

bool RustPublicActionAllowsUri(uint8_t public_type) {
  return pdfium_rust_public_action_capabilities(public_type) & 4;
}

bool RustPublicBookmarkColorIsValid(float red, float green, float blue) {
  return pdfium_rust_public_bookmark_color_is_valid(red, green, blue);
}

uint8_t RustPublicDestinationSource(bool has_direct, bool has_action) {
  return pdfium_rust_public_destination_source(has_direct, has_action);
}

uint8_t RustPublicDestinationZoomMode(pdfium::span<const uint8_t> mode) {
  return pdfium_rust_public_destination_zoom_mode(mode.data(), mode.size());
}

size_t RustPublicDestinationNumParams(uint8_t zoom_mode, size_t array_size) {
  return pdfium_rust_public_destination_num_params(zoom_mode, array_size);
}

std::optional<RustPublicDestinationXyzPlan> RustPlanPublicDestinationXyz(
    bool array_present,
    size_t array_size,
    bool is_xyz,
    std::optional<float> x,
    std::optional<float> y,
    std::optional<float> zoom) {
  RustPublicDestinationXyzPlan plan = {};
  if (!pdfium_rust_public_destination_xyz_plan(
          array_present, array_size, is_xyz, x.has_value(), x.value_or(0.0f),
          y.has_value(), y.value_or(0.0f), zoom.has_value(),
          zoom.value_or(0.0f), &plan.valid, &plan.has_x, &plan.has_y,
          &plan.has_zoom, &plan.x, &plan.y, &plan.zoom)) {
    return std::nullopt;
  }
  return plan;
}

std::optional<uintptr_t> RustFindBookmark(
    void* context,
    RustBookmarkMatchCallback matches_title,
    RustBookmarkNavigateCallback first_child,
    RustBookmarkNavigateCallback next_sibling) {
  uintptr_t output = 0;
  if (!pdfium_rust_find_bookmark(context, matches_title, first_child,
                                 next_sibling, &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<std::vector<uint8_t>> RustPageLabelNumber(
    int32_t number,
    pdfium::span<const uint8_t> style) {
  size_t output_len = 0;
  if (!pdfium_rust_page_label_number(number, style.data(), style.size(),
                                     nullptr, 0, &output_len)) {
    return std::nullopt;
  }
  std::vector<uint8_t> result(output_len);
  if (output_len != 0 &&
      !pdfium_rust_page_label_number(number, style.data(), style.size(),
                                     result.data(), result.size(),
                                     &output_len)) {
    return std::nullopt;
  }
  return result;
}

std::optional<RustLinkEnumerationResult> RustFindNextLink(
    int32_t start_position,
    size_t annotation_count,
    void* context,
    RustLinkEnumerationCallback is_link) {
  RustLinkEnumerationResult result = {};
  if (!pdfium_rust_find_next_link(start_position, annotation_count, context,
                                  is_link, &result.found, &result.index)) {
    return std::nullopt;
  }
  return result;
}

std::optional<uintptr_t> RustNumberTreeLookup(
    uintptr_t root,
    int32_t number,
    void* context,
    RustNumberTreeDescribeCallback describe,
    RustNumberTreeNumberCallback read_number,
    RustNumberTreeKidCallback read_kid) {
  uintptr_t output = 0;
  if (!pdfium_rust_number_tree_lookup(root, number, context, describe,
                                      read_number, read_kid, &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<RustNumberTreeLowerBoundResult> RustNumberTreeLowerBound(
    uintptr_t root,
    int32_t number,
    void* context,
    RustNumberTreeDescribeCallback describe,
    RustNumberTreeNumberCallback read_number,
    RustNumberTreeKidCallback read_kid) {
  RustNumberTreeLowerBoundResult result = {};
  if (!pdfium_rust_number_tree_lower_bound(
          root, number, context, describe, read_number, read_kid, &result.found,
          &result.key, &result.value)) {
    return std::nullopt;
  }
  return result;
}

std::optional<int32_t> RustDestinationPageIndex(
    uint8_t target_kind,
    int32_t direct_page,
    uint32_t object_number,
    void* context,
    RustDestinationPageCallback lookup_page) {
  int32_t output = -1;
  if (!pdfium_rust_destination_page_index(target_kind, direct_page,
                                          object_number, context, lookup_page,
                                          &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<size_t> RustNameTreeCount(
    uintptr_t root,
    void* context,
    RustNameTreeDescribeCallback describe,
    RustNameTreeKidCallback read_kid) {
  size_t output = 0;
  if (!pdfium_rust_name_tree_count(root, context, describe, read_kid,
                                   &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<RustNameTreeIndexResult> RustNameTreeFindIndex(
    uintptr_t root,
    size_t target,
    void* context,
    RustNameTreeDescribeCallback describe,
    RustNameTreeKidCallback read_kid) {
  RustNameTreeIndexResult result = {};
  if (!pdfium_rust_name_tree_find_index(
          root, target, context, describe, read_kid, &result.found,
          &result.node, &result.pair_index)) {
    return std::nullopt;
  }
  return result;
}

std::optional<RustNameTreeInsertionResult> RustNameTreePlanInsertion(
    uintptr_t root,
    void* context,
    RustNameTreeSearchDescribeCallback describe,
    RustNameTreeSearchTokenCallback read_token,
    RustNameTreeSearchLimitsCallback compare_limits,
    RustNameTreeSearchNameCallback read_name,
    RustNameTreeSearchKidCallback read_kid) {
  RustNameTreeInsertionResult result = {};
  if (!pdfium_rust_name_tree_plan_insertion(
          root, context, describe, read_token, compare_limits, read_name,
          read_kid, &result.duplicate, &result.node, &result.pair_index)) {
    return std::nullopt;
  }
  return result;
}

std::optional<uintptr_t> RustNameTreeLookup(
    uintptr_t root,
    void* context,
    RustNameTreeSearchDescribeCallback describe,
    RustNameTreeSearchTokenCallback read_token,
    RustNameTreeSearchLimitsCallback compare_limits,
    RustNameTreeSearchNameCallback read_name,
    RustNameTreeSearchKidCallback read_kid) {
  uintptr_t output = 0;
  if (!pdfium_rust_name_tree_lookup(root, context, describe, read_token,
                                    compare_limits, read_name, read_kid,
                                    &output)) {
    return std::nullopt;
  }
  return output;
}

std::optional<RustLinkEnumerationResult> RustFindLinkAtPoint(
    size_t link_count,
    float x,
    float y,
    void* context,
    RustLinkRectCallback read_rect) {
  RustLinkEnumerationResult result = {};
  if (!pdfium_rust_find_link_at_point(link_count, x, y, context, read_rect,
                                      &result.found, &result.index)) {
    return std::nullopt;
  }
  return result;
}

std::optional<std::vector<size_t>> RustDocumentPageMutationPath(
    uintptr_t root_handle,
    int pages_to_go,
    void* context,
    RustDocumentPageMutationDescribeCallback describe,
    RustDocumentPageMutationChildCallback child) {
  size_t output_len = 0;
  if (!pdfium_rust_document_page_mutation_path(root_handle, pages_to_go,
                                               context, describe, child,
                                               nullptr, 0, &output_len)) {
    return std::nullopt;
  }
  std::vector<size_t> result(output_len);
  if (output_len != 0 && !pdfium_rust_document_page_mutation_path(
                             root_handle, pages_to_go, context, describe, child,
                             result.data(), result.size(), &output_len)) {
    return std::nullopt;
  }
  return result;
}

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

std::optional<uint8_t> RustCrossRefEntryAction(uint8_t type_code,
                                               bool normal_offset_fits,
                                               uint32_t generation,
                                               bool archive_object_valid) {
  uint8_t output = 0;
  if (!pdfium_rust_cross_ref_entry_action(type_code, normal_offset_fits,
                                          generation, archive_object_valid,
                                          &output) ||
      output > 3) {
    return std::nullopt;
  }
  return output;
}

bool RunRustCrossRefEntryMutation(uint8_t type_code,
                                  bool normal_offset_fits,
                                  uint32_t generation,
                                  bool archive_object_valid,
                                  void* context,
                                  CrossRefMutationCallback callback) {
  return context && callback &&
         pdfium_rust_run_cross_ref_entry_mutation(
             type_code, normal_offset_fits, generation, archive_object_valid,
             context, callback);
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

std::optional<CrossRefEntryFields> RustReadCrossRefEntry(
    pdfium::span<const uint8_t> input,
    uint32_t first_width,
    uint32_t second_width,
    uint32_t third_width) {
  CrossRefEntryFields fields = {};
  if (!pdfium_rust_read_cross_ref_entry(
          input.data(), input.size(), first_width, second_width, third_width,
          &fields.first, &fields.second, &fields.third)) {
    return std::nullopt;
  }
  return fields;
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

std::optional<PdfTokenScan> RustScanPdfToken(pdfium::span<const uint8_t> input,
                                             uint32_t position) {
  PdfTokenScan result = {};
  if (!pdfium_rust_scan_pdf_token(input.data(), input.size(), position,
                                  &result.position, &result.has_word,
                                  &result.start, &result.len) ||
      (result.has_word && (result.start > input.size() ||
                           result.len > input.size() - result.start))) {
    return std::nullopt;
  }
  return result;
}

bool UseRustParserCandidate() {
  return g_use_rust_parser_candidate;
}

bool SetUseRustParserCandidateForTesting(bool use_candidate) {
  const bool previous = g_use_rust_parser_candidate;
  g_use_rust_parser_candidate = use_candidate;
  return previous;
}

ScopedRustParserImplementationForTesting::
    ScopedRustParserImplementationForTesting(bool use_candidate)
    : previous_(SetUseRustParserCandidateForTesting(use_candidate)) {}

ScopedRustParserImplementationForTesting::
    ~ScopedRustParserImplementationForTesting() {
  SetUseRustParserCandidateForTesting(previous_);
}

}  // namespace pdfium::rust
