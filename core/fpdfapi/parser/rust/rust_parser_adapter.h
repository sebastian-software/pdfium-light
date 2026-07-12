#ifndef CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_
#define CORE_FPDFAPI_PARSER_RUST_RUST_PARSER_ADAPTER_H_

#include <stdint.h>

#include <optional>
#include <vector>

#include "core/fxcrt/span.h"

namespace pdfium::rust {

struct RustCrossRefObjectInfo {
  uint8_t type;
  bool is_object_stream;
  uint16_t generation;
  int64_t position;
  uint32_t archive_object_number;
  uint32_t archive_object_index;
};

using RustCrossRefSnapshotCallback =
    bool (*)(void* context,
             uint32_t object_number,
             const RustCrossRefObjectInfo& info);

class RustCrossRefTable final {
 public:
  RustCrossRefTable();
  RustCrossRefTable(const RustCrossRefTable&) = delete;
  RustCrossRefTable& operator=(const RustCrossRefTable&) = delete;
  ~RustCrossRefTable();

  bool AddCompressed(uint32_t object_number,
                     uint32_t archive_object_number,
                     uint32_t archive_object_index);
  bool AddNormal(uint32_t object_number,
                 uint16_t generation,
                 bool is_object_stream,
                 int64_t position);
  bool SetFree(uint32_t object_number, uint16_t generation);
  bool SetSize(uint32_t size);
  bool OverlayFrom(RustCrossRefTable* top);
  bool Snapshot(void* context, RustCrossRefSnapshotCallback callback) const;

 private:
  void* state_;
};

struct RustIndirectObjectLookup {
  uint8_t status;
  uintptr_t handle;
};

struct RustIndirectObjectReplaceResult {
  bool applied;
  std::optional<uintptr_t> old_handle;
};

struct RustIndirectObjectAddResult {
  uint32_t object_number;
  std::optional<uintptr_t> old_handle;
};

using RustIndirectObjectSnapshotCallback = bool (*)(void* context,
                                                    uint32_t object_number,
                                                    uintptr_t handle);

class RustIndirectObjectIndex final {
 public:
  RustIndirectObjectIndex();
  RustIndirectObjectIndex(const RustIndirectObjectIndex&) = delete;
  RustIndirectObjectIndex& operator=(const RustIndirectObjectIndex&) = delete;
  ~RustIndirectObjectIndex();

  std::optional<RustIndirectObjectLookup> Lookup(uint32_t object_number) const;
  std::optional<RustIndirectObjectLookup> ReserveParse(uint32_t object_number);
  bool FinishParse(uint32_t object_number, uintptr_t handle);
  bool CancelParse(uint32_t object_number);
  std::optional<RustIndirectObjectAddResult> Add(uintptr_t handle);
  std::optional<RustIndirectObjectReplaceResult> Replace(
      uint32_t object_number,
      uintptr_t handle,
      uint32_t new_generation,
      std::optional<uint32_t> old_generation);
  std::optional<std::optional<uintptr_t>> Delete(uint32_t object_number);
  bool ContainsHandle(uintptr_t handle) const;
  std::optional<uint32_t> GetLastObjectNumber() const;
  bool SetLastObjectNumber(uint32_t object_number);
  bool Snapshot(void* context,
                RustIndirectObjectSnapshotCallback callback) const;

 private:
  void* state_;
};

class RustPdfNumber final {
 public:
  RustPdfNumber();
  explicit RustPdfNumber(int32_t value);
  explicit RustPdfNumber(float value);
  explicit RustPdfNumber(pdfium::span<const uint8_t> value);
  RustPdfNumber(const RustPdfNumber&) = delete;
  RustPdfNumber& operator=(const RustPdfNumber&) = delete;
  ~RustPdfNumber();

  bool IsInteger() const;
  int32_t GetSigned() const;
  float GetFloat() const;
  bool SetString(pdfium::span<const uint8_t> value);

 private:
  explicit RustPdfNumber(void* state);
  void* state_;
};

class RustPdfBoolean final {
 public:
  explicit RustPdfBoolean(bool value);
  RustPdfBoolean(const RustPdfBoolean&) = delete;
  RustPdfBoolean& operator=(const RustPdfBoolean&) = delete;
  ~RustPdfBoolean();

  bool Get() const;
  bool SetString(pdfium::span<const uint8_t> value);

 private:
  void* state_;
};

class RustPdfReference final {
 public:
  explicit RustPdfReference(uint32_t object_number);
  RustPdfReference(const RustPdfReference&) = delete;
  RustPdfReference& operator=(const RustPdfReference&) = delete;
  ~RustPdfReference();

  uint32_t GetObjectNumber() const;
  bool SetObjectNumber(uint32_t object_number);

 private:
  void* state_;
};

class RustPdfArray final {
 public:
  RustPdfArray();
  RustPdfArray(const RustPdfArray&) = delete;
  RustPdfArray& operator=(const RustPdfArray&) = delete;
  ~RustPdfArray();

  size_t size() const;
  std::optional<uintptr_t> Get(size_t index) const;
  bool Append(uintptr_t handle);
  std::optional<uintptr_t> Set(size_t index, uintptr_t handle);
  bool Insert(size_t index, uintptr_t handle);
  std::optional<uintptr_t> Remove(size_t index);
  bool Clear();
  bool ContainsHandle(uintptr_t handle) const;

 private:
  void* state_;
};

using RustPdfDictionarySnapshotCallback = bool (*)(void* context,
                                                   const uint8_t* key,
                                                   size_t key_len,
                                                   uintptr_t handle);

class RustPdfDictionary final {
 public:
  RustPdfDictionary();
  RustPdfDictionary(const RustPdfDictionary&) = delete;
  RustPdfDictionary& operator=(const RustPdfDictionary&) = delete;
  ~RustPdfDictionary();

  size_t size() const;
  std::optional<uintptr_t> Get(pdfium::span<const uint8_t> key) const;
  std::optional<uintptr_t> Set(pdfium::span<const uint8_t> key,
                               uintptr_t handle);
  std::optional<uintptr_t> Remove(pdfium::span<const uint8_t> key);
  bool ContainsHandle(uintptr_t handle) const;
  bool Snapshot(void* context,
                RustPdfDictionarySnapshotCallback callback) const;

 private:
  void* state_;
};

class RustPdfString final {
 public:
  RustPdfString(pdfium::span<const uint8_t> value, bool output_is_hex);
  RustPdfString(const RustPdfString&) = delete;
  RustPdfString& operator=(const RustPdfString&) = delete;
  ~RustPdfString();

  bool IsHex() const;
  bool Equals(pdfium::span<const uint8_t> value) const;
  bool Set(pdfium::span<const uint8_t> value);

 private:
  void* state_;
};

class RustPdfStreamData final {
 public:
  explicit RustPdfStreamData(pdfium::span<const uint8_t> value);
  RustPdfStreamData(const RustPdfStreamData&) = delete;
  RustPdfStreamData& operator=(const RustPdfStreamData&) = delete;
  ~RustPdfStreamData();

  pdfium::span<const uint8_t> GetSpan() const;

 private:
  void* state_;
};

class RustDocumentPageIndex final {
 public:
  RustDocumentPageIndex();
  RustDocumentPageIndex(const RustDocumentPageIndex&) = delete;
  RustDocumentPageIndex& operator=(const RustDocumentPageIndex&) = delete;
  ~RustDocumentPageIndex();

  size_t size() const;
  bool Resize(size_t size);
  std::optional<uint32_t> Get(size_t index) const;
  bool Set(size_t index, uint32_t object_number);
  bool Insert(size_t index, uint32_t object_number);
  bool Remove(size_t index);
  bool Contains(uint32_t object_number) const;

 private:
  void* state_;
};

std::optional<std::vector<int>> RustDocumentMovePageDeletionOrder(
    pdfium::span<const int> page_indices,
    size_t num_pages,
    int destination);

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
