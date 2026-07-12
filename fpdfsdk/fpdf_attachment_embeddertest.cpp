// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#include <string>
#include <vector>

#include "core/fpdfapi/parser/rust/rust_parser_adapter.h"
#include "public/fpdf_attachment.h"
#include "public/fpdf_save.h"
#include "public/fpdfview.h"
#include "testing/embedder_test.h"
#include "testing/fx_string_testhelpers.h"
#include "testing/gmock/include/gmock/gmock.h"
#include "testing/utils/hash.h"

static constexpr char kDateKey[] = "CreationDate";
static constexpr char kChecksumKey[] = "CheckSum";
static constexpr char kFacturXXml[] =
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
    "<rsm:CrossIndustryInvoice "
    "xmlns:rsm=\"urn:un:unece:uncefact:data:standard:"
    "CrossIndustryInvoice:100\">\n"
    "  <rsm:ExchangedDocument><rsm:ID>PDFIUM-LIGHT-TEST-001</rsm:ID>"
    "</rsm:ExchangedDocument>\n"
    "</rsm:CrossIndustryInvoice>";

class FPDFAttachmentEmbedderTest : public EmbedderTest {};

namespace {

struct AttachmentSnapshot {
  int count = 0;
  bool valid = true;
  std::vector<std::wstring> names;
  std::vector<std::string> contents;
  bool operator==(const AttachmentSnapshot&) const = default;
};

AttachmentSnapshot SnapshotAttachments(FPDF_DOCUMENT document) {
  AttachmentSnapshot result;
  result.count = FPDFDoc_GetAttachmentCount(document);
  for (int index = 0; index < result.count; ++index) {
    FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document, index);
    if (!attachment) {
      result.valid = false;
      continue;
    }
    unsigned long length = FPDFAttachment_GetName(attachment, nullptr, 0);
    std::vector<FPDF_WCHAR> name = GetFPDFWideStringBuffer(length);
    if (FPDFAttachment_GetName(attachment, name.data(), length) != length) {
      result.valid = false;
      continue;
    }
    result.names.push_back(GetPlatformWString(name.data()));

    if (!FPDFAttachment_GetFile(attachment, nullptr, 0, &length)) {
      result.valid = false;
      continue;
    }
    std::vector<char> content(length);
    unsigned long actual_length = 0;
    if (!FPDFAttachment_GetFile(attachment, content.data(), length,
                                &actual_length) ||
        actual_length != length) {
      result.valid = false;
      continue;
    }
    result.contents.emplace_back(content.data(), actual_length);
  }
  return result;
}

void ExpectFacturXAttachment(FPDF_DOCUMENT document) {
  ASSERT_EQ(1, FPDFDoc_GetAttachmentCount(document));
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document, 0);
  ASSERT_TRUE(attachment);

  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  std::vector<FPDF_WCHAR> name = GetFPDFWideStringBuffer(length_bytes);
  ASSERT_EQ(length_bytes,
            FPDFAttachment_GetName(attachment, name.data(), length_bytes));
  EXPECT_EQ(L"factur-x.xml", GetPlatformWString(name.data()));

  static constexpr char kDescriptionKey[] = "Desc";
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kDescriptionKey, nullptr, 0);
  std::vector<FPDF_WCHAR> description = GetFPDFWideStringBuffer(length_bytes);
  ASSERT_EQ(length_bytes, FPDFAttachment_GetStringValue(
                              attachment, kDescriptionKey, description.data(),
                              length_bytes));
  EXPECT_EQ(L"application/xml; profile=FACTUR-X",
            GetPlatformWString(description.data()));

  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  std::vector<uint8_t> content(length_bytes);
  unsigned long actual_length_bytes = 0;
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content.data(), length_bytes,
                                     &actual_length_bytes));
  ASSERT_EQ(sizeof(kFacturXXml) - 1, actual_length_bytes);
  EXPECT_EQ(kFacturXXml,
            std::string(reinterpret_cast<const char*>(content.data()),
                        content.size()));
}

}  // namespace

TEST_F(FPDFAttachmentEmbedderTest, ExtractAttachments) {
  // Open a file with two attachments.
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  // Try to retrieve attachments at bad indices.
  EXPECT_FALSE(FPDFDoc_GetAttachment(document(), -1));
  EXPECT_FALSE(FPDFDoc_GetAttachment(document(), 2));

  // Retrieve the first attachment.
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);

  // Check that the name of the first attachment is correct.
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"1.txt", GetPlatformWString(buf.data()));

  // Check some unsuccessful cases of FPDFAttachment_GetFile.
  EXPECT_FALSE(FPDFAttachment_GetFile(attachment, nullptr, 0, nullptr));
  EXPECT_FALSE(FPDFAttachment_GetFile(nullptr, nullptr, 0, &length_bytes));

  // Check that the content of the first attachment is correct.
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  std::vector<uint8_t> content_buf(length_bytes);
  unsigned long actual_length_bytes;
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_THAT(content_buf, testing::ElementsAre('t', 'e', 's', 't'));

  // Check that a non-existent key does not exist.
  EXPECT_FALSE(FPDFAttachment_HasKey(attachment, "none"));

  // Check that the string value of a non-string dictionary entry is empty.
  static constexpr char kSizeKey[] = "Size";
  EXPECT_EQ(FPDF_OBJECT_NUMBER,
            FPDFAttachment_GetValueType(attachment, kSizeKey));
  EXPECT_EQ(2u,
            FPDFAttachment_GetStringValue(attachment, kSizeKey, nullptr, 0));

  // Check that the creation date of the first attachment is correct.
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kDateKey, nullptr, 0);
  ASSERT_EQ(48u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(48u, FPDFAttachment_GetStringValue(attachment, kDateKey, buf.data(),
                                               length_bytes));
  EXPECT_EQ(L"D:20170712214438-07'00'", GetPlatformWString(buf.data()));

  // Retrieve the second attachment.
  attachment = FPDFDoc_GetAttachment(document(), 1);
  ASSERT_TRUE(attachment);

  // Retrieve the second attachment file.
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  content_buf.clear();
  content_buf.resize(length_bytes);
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(5869u, actual_length_bytes);

  // Check that the calculated checksum of the file data matches expectation.
  const char kCheckSum[] = "72afcddedf554dda63c0c88e06f1ce18";
  const wchar_t kCheckSumW[] = L"<72AFCDDEDF554DDA63C0C88E06F1CE18>";
  const std::string generated_checksum = GenerateMD5Base16(content_buf);
  EXPECT_EQ(kCheckSum, generated_checksum);

  // Check that the stored checksum matches expectation.
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kChecksumKey, nullptr, 0);
  ASSERT_EQ(70u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(70u, FPDFAttachment_GetStringValue(attachment, kChecksumKey,
                                               buf.data(), length_bytes));
  EXPECT_EQ(kCheckSumW, GetPlatformWString(buf.data()));
}

TEST_F(FPDFAttachmentEmbedderTest, FacturXAttachmentSurvivesSaveReload) {
  ASSERT_TRUE(OpenDocument("zugferd_facturx_attachment.pdf"));
  ExpectFacturXAttachment(document());

  ASSERT_TRUE(FPDF_SaveAsCopy(document(), this, 0));
  ScopedSavedDoc saved_document = OpenScopedSavedDocument();
  ASSERT_TRUE(saved_document);
  ExpectFacturXAttachment(saved_document.get());
}

TEST_F(FPDFAttachmentEmbedderTest, NoAttachmentToExtract) {
  // Open a file with no attachments.
  ASSERT_TRUE(OpenDocument("hello_world.pdf"));
  EXPECT_EQ(0, FPDFDoc_GetAttachmentCount(document()));

  // Try to retrieve attachments at bad indices.
  EXPECT_FALSE(FPDFDoc_GetAttachment(document(), -1));
  EXPECT_FALSE(FPDFDoc_GetAttachment(document(), 0));
}

TEST_F(FPDFAttachmentEmbedderTest, InvalidAttachmentData) {
  // Open a file with an attachment that is missing the embedded file (/EF).
  ASSERT_TRUE(OpenDocument("embedded_attachments_invalid_data.pdf"));
  ASSERT_EQ(1, FPDFDoc_GetAttachmentCount(document()));

  // Retrieve the first attachment.
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);

  // Check that the name of the attachment is correct.
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ("1.txt", GetPlatformString(buf.data()));

  // Check that is is not possible to retrieve the file data.
  EXPECT_FALSE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));

  // Check that the attachment can be deleted.
  EXPECT_TRUE(FPDFDoc_DeleteAttachment(document(), 0));
  EXPECT_EQ(0, FPDFDoc_GetAttachmentCount(document()));
}

TEST_F(FPDFAttachmentEmbedderTest, AddAttachments) {
  // Open a file with two attachments.
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  // Check that adding an attachment with an empty name would fail.
  EXPECT_FALSE(FPDFDoc_AddAttachment(document(), nullptr));

  // Add an attachment to the beginning of the embedded file list.
  ScopedFPDFWideString file_name = GetFPDFWideString(L"0.txt");
  FPDF_ATTACHMENT attachment =
      FPDFDoc_AddAttachment(document(), file_name.get());
  ASSERT_TRUE(attachment);

  // Check that writing to a file with nullptr but non-zero bytes would fail.
  EXPECT_FALSE(FPDFAttachment_SetFile(attachment, document(), nullptr, 10));

  // Set the new attachment's file.
  static constexpr char kContents1[] = "Hello!";
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), kContents1,
                                     strlen(kContents1)));
  EXPECT_EQ(3, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the new attachment (i.e. the first attachment).
  attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"0.txt", GetPlatformWString(buf.data()));

  // Verify the content of the new attachment (i.e. the first attachment).
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  std::vector<char> content_buf(length_bytes);
  unsigned long actual_length_bytes;
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(6u, actual_length_bytes);
  EXPECT_EQ(std::string(kContents1), std::string(content_buf.data(), 6));

  // Add an attachment to the end of the embedded file list and set its file.
  file_name = GetFPDFWideString(L"z.txt");
  attachment = FPDFDoc_AddAttachment(document(), file_name.get());
  ASSERT_TRUE(attachment);
  static constexpr char kContents2[] = "World!";
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), kContents2,
                                     strlen(kContents2)));
  EXPECT_EQ(4, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the new attachment (i.e. the fourth attachment).
  attachment = FPDFDoc_GetAttachment(document(), 3);
  ASSERT_TRUE(attachment);
  length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"z.txt", GetPlatformWString(buf.data()));

  // Verify the content of the new attachment (i.e. the fourth attachment).
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  content_buf.clear();
  content_buf.resize(length_bytes);
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(6u, actual_length_bytes);
  EXPECT_EQ(std::string(kContents2), std::string(content_buf.data(), 6));
}

TEST_F(FPDFAttachmentEmbedderTest,
       RustNameTreeInsertionMatchesCppAcrossSaveReload) {
  struct MutationSnapshot {
    bool duplicate_rejected = false;
    bool prepended = false;
    bool appended = false;
    bool prepended_file_set = false;
    bool appended_file_set = false;
    bool saved = false;
    AttachmentSnapshot before_save;
    AttachmentSnapshot after_reload;
    bool operator==(const MutationSnapshot&) const = default;
  };
  auto mutate = [&](bool use_rust) {
    pdfium::rust::ScopedRustParserImplementationForTesting implementation(
        use_rust);
    MutationSnapshot result;
    if (!OpenDocument("embedded_attachments.pdf")) {
      return result;
    }

    ScopedFPDFWideString existing_name = GetFPDFWideString(L"1.txt");
    result.duplicate_rejected =
        !FPDFDoc_AddAttachment(document(), existing_name.get());

    ScopedFPDFWideString first_name = GetFPDFWideString(L"0.txt");
    FPDF_ATTACHMENT first =
        FPDFDoc_AddAttachment(document(), first_name.get());
    result.prepended = !!first;
    static constexpr char kFirstContents[] = "First";
    result.prepended_file_set =
        first && FPDFAttachment_SetFile(first, document(), kFirstContents,
                                        strlen(kFirstContents));

    ScopedFPDFWideString last_name = GetFPDFWideString(L"z.txt");
    FPDF_ATTACHMENT last = FPDFDoc_AddAttachment(document(), last_name.get());
    result.appended = !!last;
    static constexpr char kLastContents[] = "Last";
    result.appended_file_set =
        last && FPDFAttachment_SetFile(last, document(), kLastContents,
                                       strlen(kLastContents));
    result.before_save = SnapshotAttachments(document());
    result.saved = !!FPDF_SaveAsCopy(document(), this, 0);
    if (result.saved) {
      ScopedSavedDoc saved_document = OpenScopedSavedDocument();
      if (saved_document) {
        result.after_reload = SnapshotAttachments(saved_document.get());
      }
    }
    CloseDocument();
    return result;
  };

  MutationSnapshot cpp = mutate(false);
  MutationSnapshot rust = mutate(true);
  EXPECT_TRUE(cpp.duplicate_rejected);
  EXPECT_TRUE(cpp.prepended);
  EXPECT_TRUE(cpp.appended);
  EXPECT_TRUE(cpp.prepended_file_set);
  EXPECT_TRUE(cpp.appended_file_set);
  EXPECT_TRUE(cpp.saved);
  EXPECT_TRUE(cpp.before_save.valid);
  EXPECT_TRUE(cpp.after_reload.valid);
  EXPECT_EQ(cpp.before_save, cpp.after_reload);
  EXPECT_EQ(cpp, rust);
}

TEST_F(FPDFAttachmentEmbedderTest, AddAttachmentsWithParams) {
  // Open a file with two attachments.
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  // Add an attachment to the embedded file list.
  ScopedFPDFWideString file_name = GetFPDFWideString(L"5.txt");
  FPDF_ATTACHMENT attachment =
      FPDFDoc_AddAttachment(document(), file_name.get());
  ASSERT_TRUE(attachment);
  static constexpr char kContents[] = "Hello World!";
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), kContents,
                                     strlen(kContents)));

  // Set the date to be an arbitrary value.
  static constexpr wchar_t kDateW[] = L"D:20170720161527-04'00'";
  ScopedFPDFWideString ws_date = GetFPDFWideString(kDateW);
  EXPECT_TRUE(
      FPDFAttachment_SetStringValue(attachment, kDateKey, ws_date.get()));

  // Set the checksum to be an arbitrary value.
  static constexpr wchar_t kCheckSumW[] = L"<ABCDEF01234567899876543210FEDCBA>";
  ScopedFPDFWideString ws_checksum = GetFPDFWideString(kCheckSumW);
  EXPECT_TRUE(FPDFAttachment_SetStringValue(attachment, kChecksumKey,
                                            ws_checksum.get()));

  // Verify the name of the new attachment (i.e. the second attachment).
  attachment = FPDFDoc_GetAttachment(document(), 1);
  ASSERT_TRUE(attachment);
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"5.txt", GetPlatformWString(buf.data()));

  // Verify the content of the new attachment.
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  std::vector<char> content_buf(length_bytes);
  unsigned long actual_length_bytes;
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(12u, actual_length_bytes);
  EXPECT_EQ(std::string(kContents), std::string(content_buf.data(), 12));

  // Verify the creation date of the new attachment.
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kDateKey, nullptr, 0);
  ASSERT_EQ(48u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(48u, FPDFAttachment_GetStringValue(attachment, kDateKey, buf.data(),
                                               length_bytes));
  EXPECT_EQ(kDateW, GetPlatformWString(buf.data()));

  // Verify the checksum of the new attachment.
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kChecksumKey, nullptr, 0);
  ASSERT_EQ(70u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(70u, FPDFAttachment_GetStringValue(attachment, kChecksumKey,
                                               buf.data(), length_bytes));
  EXPECT_EQ(kCheckSumW, GetPlatformWString(buf.data()));

  // Overwrite the existing file with empty content, and check that the checksum
  // gets updated to the correct value.
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), nullptr, 0));
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  EXPECT_EQ(0u, length_bytes);
  length_bytes =
      FPDFAttachment_GetStringValue(attachment, kChecksumKey, nullptr, 0);
  ASSERT_EQ(70u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(70u, FPDFAttachment_GetStringValue(attachment, kChecksumKey,
                                               buf.data(), length_bytes));
  EXPECT_EQ(L"<D41D8CD98F00B204E9800998ECF8427E>",
            GetPlatformWString(buf.data()));
}

TEST_F(FPDFAttachmentEmbedderTest, AddAttachmentsToFileWithNoAttachments) {
  // Open a file with no attachments.
  ASSERT_TRUE(OpenDocument("hello_world.pdf"));
  EXPECT_EQ(0, FPDFDoc_GetAttachmentCount(document()));

  // Add an attachment to the beginning of the embedded file list.
  ScopedFPDFWideString file_name = GetFPDFWideString(L"0.txt");
  FPDF_ATTACHMENT attachment =
      FPDFDoc_AddAttachment(document(), file_name.get());
  ASSERT_TRUE(attachment);

  // Set the new attachment's file.
  static constexpr char kContents1[] = "Hello!";
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), kContents1,
                                     strlen(kContents1)));
  EXPECT_EQ(1, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the new attachment (i.e. the first attachment).
  attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"0.txt", GetPlatformWString(buf.data()));

  // Verify the content of the new attachment (i.e. the first attachment).
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  std::vector<char> content_buf(length_bytes);
  unsigned long actual_length_bytes;
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(6u, actual_length_bytes);
  EXPECT_EQ(std::string(kContents1), std::string(content_buf.data(), 6));

  // Add an attachment to the end of the embedded file list and set its file.
  file_name = GetFPDFWideString(L"z.txt");
  attachment = FPDFDoc_AddAttachment(document(), file_name.get());
  ASSERT_TRUE(attachment);
  static constexpr char kContents2[] = "World!";
  EXPECT_TRUE(FPDFAttachment_SetFile(attachment, document(), kContents2,
                                     strlen(kContents2)));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the new attachment (i.e. the second attachment).
  attachment = FPDFDoc_GetAttachment(document(), 1);
  ASSERT_TRUE(attachment);
  length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"z.txt", GetPlatformWString(buf.data()));

  // Verify the content of the new attachment (i.e. the second attachment).
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, nullptr, 0, &length_bytes));
  content_buf.clear();
  content_buf.resize(length_bytes);
  ASSERT_TRUE(FPDFAttachment_GetFile(attachment, content_buf.data(),
                                     length_bytes, &actual_length_bytes));
  ASSERT_EQ(6u, actual_length_bytes);
  EXPECT_EQ(std::string(kContents2), std::string(content_buf.data(), 6));
}

TEST_F(FPDFAttachmentEmbedderTest, DeleteAttachment) {
  // Open a file with two attachments.
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the first attachment.
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);
  unsigned long length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(12u, length_bytes);
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(12u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"1.txt", GetPlatformWString(buf.data()));

  // Delete the first attachment.
  EXPECT_TRUE(FPDFDoc_DeleteAttachment(document(), 0));
  EXPECT_EQ(1, FPDFDoc_GetAttachmentCount(document()));

  // Verify the name of the new first attachment.
  attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);
  length_bytes = FPDFAttachment_GetName(attachment, nullptr, 0);
  ASSERT_EQ(26u, length_bytes);
  buf = GetFPDFWideStringBuffer(length_bytes);
  EXPECT_EQ(26u, FPDFAttachment_GetName(attachment, buf.data(), length_bytes));
  EXPECT_EQ(L"attached.pdf", GetPlatformWString(buf.data()));
}

TEST_F(FPDFAttachmentEmbedderTest, GetStringValueForChecksumNotString) {
  ASSERT_TRUE(OpenDocument("embedded_attachments_invalid_types.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);

  // The checksum key is a name, which violates the spec. This will still return
  // the value, but should not crash.
  static constexpr unsigned long kExpectedLength = 8u;
  ASSERT_EQ(kExpectedLength, FPDFAttachment_GetStringValue(
                                 attachment, kChecksumKey, nullptr, 0));
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(kExpectedLength);
  EXPECT_EQ(kExpectedLength,
            FPDFAttachment_GetStringValue(attachment, kChecksumKey, buf.data(),
                                          kExpectedLength));
  EXPECT_EQ(L"Bad", GetPlatformWString(buf.data()));
}

TEST_F(FPDFAttachmentEmbedderTest, GetStringValueForNotString) {
  ASSERT_TRUE(OpenDocument("embedded_attachments_invalid_types.pdf"));
  EXPECT_EQ(2, FPDFDoc_GetAttachmentCount(document()));

  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 1);
  ASSERT_TRUE(attachment);

  // The checksum key is a stream, while the API requires a string or name.
  static constexpr unsigned long kExpectedLength = 2u;
  ASSERT_EQ(kExpectedLength, FPDFAttachment_GetStringValue(
                                 attachment, kChecksumKey, nullptr, 0));
  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(kExpectedLength);
  EXPECT_EQ(kExpectedLength,
            FPDFAttachment_GetStringValue(attachment, kChecksumKey, buf.data(),
                                          kExpectedLength));
  EXPECT_EQ(L"", GetPlatformWString(buf.data()));
}

TEST_F(FPDFAttachmentEmbedderTest, GetSubtype) {
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);

  // Test getting Subtype (MIME type)
  constexpr char kExpectedSubtype[] = "text/plain";
  unsigned long length = FPDFAttachment_GetSubtype(attachment, nullptr, 0);
  ASSERT_EQ(2u * (strlen(kExpectedSubtype) + 1), length);

  std::vector<FPDF_WCHAR> buf = GetFPDFWideStringBuffer(length);
  EXPECT_EQ(length, FPDFAttachment_GetSubtype(attachment, buf.data(), length));
  EXPECT_EQ(kExpectedSubtype, GetPlatformString(buf.data()));

  // Test with buffer too small
  std::vector<FPDF_WCHAR> small_buf(length - 1);
  const FPDF_WCHAR kPattern = 0xDEAD;
  std::ranges::fill(small_buf, kPattern);
  EXPECT_EQ(length, FPDFAttachment_GetSubtype(attachment, small_buf.data(),
                                              length - 1));
  EXPECT_THAT(small_buf, testing::Each(kPattern));
}

TEST_F(FPDFAttachmentEmbedderTest, GetSubtypeInvalid) {
  ASSERT_TRUE(OpenDocument("embedded_attachments.pdf"));
  FPDF_ATTACHMENT attachment = FPDFDoc_GetAttachment(document(), 0);
  ASSERT_TRUE(attachment);

  std::vector<FPDF_WCHAR> buf(1);
  EXPECT_EQ(0u, FPDFAttachment_GetSubtype(nullptr, buf.data(), 1));

  constexpr char kExpectedSubtype[] = "text/plain";
  EXPECT_EQ(2u * (strlen(kExpectedSubtype) + 1),
            FPDFAttachment_GetSubtype(attachment, nullptr, 10));
}
