import Foundation
import PDFKit
import UniformTypeIdentifiers
import Compression

/// Document processor for extracting text from various file formats
/// Supports: PDF, TXT, MD, DOCX, XLSX
/// Note: File size is limited to ~10,000 characters to stay within model token limits
class DocumentProcessor {

    /// Maximum safe character count for file content (approximately 2,500 tokens)
    /// This leaves room for conversation history and model responses
    static let maxCharacterLimit = 10_000

    /// Extract text from a file URL
    /// Supports: PDF, TXT, MD, DOCX, XLSX
    /// Note: The caller is responsible for managing security-scoped resource access
    static func extractText(from url: URL) async throws -> String {
        // Get file extension
        let fileExtension = url.pathExtension.lowercased()

        // Check file size first (500KB limit)
        let fileSize = try getFileSize(url: url)
        if fileSize > 500_000 { // 500KB limit
            let fileSizeMB = Double(fileSize) / 1_000_000.0
            throw DocumentProcessingError.fileTooLarge(
                "File is too large (\(String(format: "%.2f", fileSizeMB)) MB). " +
                "Please use files smaller than 500KB."
            )
        }

        // Determine file type and extract text accordingly
        let extractedText: String
        switch fileExtension {
        case "pdf":
            extractedText = try await extractTextFromPDF(url: url)
        case "txt", "text":
            extractedText = try extractTextFromTextFile(url: url)
        case "md", "markdown":
            extractedText = try extractTextFromTextFile(url: url)
        case "docx":
            extractedText = try await extractTextFromDOCX(url: url)
        case "xlsx", "xls":
            extractedText = try await extractTextFromXLSX(url: url)
        default:
            // Try to read as plain text as fallback
            extractedText = try extractTextFromTextFile(url: url)
        }

        // Check character count limit
        if extractedText.count > maxCharacterLimit {
            let characterCount = extractedText.count
            let maxChars = maxCharacterLimit
            throw DocumentProcessingError.contentTooLong(
                "Document content is too long (\(characterCount) characters). " +
                "Maximum supported: \(maxChars) characters. " +
                "Please split the document into smaller parts or use a shorter document."
            )
        }

        if extractedText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            throw DocumentProcessingError.emptyDocument("Document contains no extractable text.")
        }

        return extractedText
    }

    // MARK: - File Size Check

    private static func getFileSize(url: URL) throws -> Int64 {
        // Try to get file size
        let resourceValues = try url.resourceValues(forKeys: [.fileSizeKey])
        if let fileSize = resourceValues.fileSize {
            return Int64(fileSize)
        }

        // Fallback: try to get file size from file attributes
        let attributes = try FileManager.default.attributesOfItem(atPath: url.path)
        if let fileSize = attributes[.size] as? Int64 {
            return fileSize
        }

        // If we can't determine file size, return 0 (will be checked later)
        return 0
    }

    // MARK: - PDF Text Extraction

    private static func extractTextFromPDF(url: URL) async throws -> String {
        guard let pdfDocument = PDFDocument(url: url) else {
            throw DocumentProcessingError.invalidFormat("Unable to read PDF file. The file may be corrupted or password-protected.")
        }

        var extractedText = ""
        let pageCount = pdfDocument.pageCount

        // Limit to first 5 pages to avoid exceeding token limits
        let maxPages = min(pageCount, 5)

        for pageIndex in 0..<maxPages {
            if let page = pdfDocument.page(at: pageIndex),
               let pageContent = page.string {
                extractedText += pageContent
                if pageIndex < maxPages - 1 {
                    extractedText += "\n\n"
                }
            }
        }

        if pageCount > maxPages {
            extractedText += "\n\n[Note: Document has \(pageCount) pages. Only first \(maxPages) pages were processed due to size limits.]"
        }

        if extractedText.isEmpty {
            throw DocumentProcessingError.emptyDocument("PDF file contains no extractable text. It may be image-based or scanned.")
        }

        return extractedText
    }

    // MARK: - Text File Extraction

    private static func extractTextFromTextFile(url: URL) throws -> String {
        // Try UTF-8 first
        if let text = try? String(contentsOf: url, encoding: .utf8) {
            return text
        }

        // Try UTF-16 if UTF-8 fails
        if let text = try? String(contentsOf: url, encoding: .utf16) {
            return text
        }

        // Try other encodings
        if let text = try? String(contentsOf: url, encoding: .ascii) {
            return text
        }

        throw DocumentProcessingError.invalidFormat("Unable to read text file. Unsupported encoding.")
    }

    // MARK: - DOCX Text Extraction

    /// Extract text from DOCX files
    /// DOCX files are ZIP archives containing XML files
    /// Main content is in word/document.xml
    private static func extractTextFromDOCX(url: URL) async throws -> String {
        guard let data = try? Data(contentsOf: url) else {
            throw DocumentProcessingError.invalidFormat("Unable to read DOCX file.")
        }

        // DOCX is a ZIP archive, extract word/document.xml
        do {
            let xmlData = try extractFileFromZip(data: data, filePath: "word/document.xml")
            let text = try extractTextFromXML(data: xmlData)
            if !text.isEmpty {
                return text
            }
        } catch {
            // If ZIP extraction fails, try fallback methods
        }

        // Fallback: Try as RTF
        if let attributedString = try? NSAttributedString(
            data: data,
            options: [.documentType: NSAttributedString.DocumentType.rtf],
            documentAttributes: nil
        ) {
            let text = attributedString.string
            if !text.isEmpty {
                return text
            }
        }

        throw DocumentProcessingError.unsupportedFormat(
            "Unable to extract text from this DOCX file. The file may be corrupted or have complex formatting."
        )
    }

    // MARK: - XLSX Text Extraction

    /// Extract text from XLSX files
    /// XLSX files are ZIP archives containing XML files
    /// Text content is in xl/sharedStrings.xml and xl/worksheets/sheet*.xml
    private static func extractTextFromXLSX(url: URL) async throws -> String {
        guard let data = try? Data(contentsOf: url) else {
            throw DocumentProcessingError.invalidFormat("Unable to read XLSX file.")
        }

        var allText: [String] = []

        // Extract shared strings (common text used across cells)
        if let sharedStringsData = try? extractFileFromZip(data: data, filePath: "xl/sharedStrings.xml") {
            if let sharedStrings = try? extractTextFromXML(data: sharedStringsData) {
                if !sharedStrings.isEmpty {
                    allText.append("Shared Strings:\n\(sharedStrings)")
                }
            }
        }

        // Extract text from worksheets (limit to first 3 sheets to avoid token limits)
        for sheetIndex in 1...3 {
            let sheetPath = "xl/worksheets/sheet\(sheetIndex).xml"
            if let sheetData = try? extractFileFromZip(data: data, filePath: sheetPath) {
                if let sheetText = try? extractTextFromXML(data: sheetData) {
                    if !sheetText.isEmpty {
                        allText.append("Sheet \(sheetIndex):\n\(sheetText)")
                    }
                }
            } else {
                // No more sheets
                break
            }
        }

        if allText.isEmpty {
            throw DocumentProcessingError.emptyDocument("XLSX file contains no extractable text.")
        }

        return allText.joined(separator: "\n\n")
    }

    // MARK: - ZIP Extraction Helper

    /// Extract a specific file from a ZIP archive
    /// This is a simplified ZIP parser that works for Office Open XML files
    private static func extractFileFromZip(data: Data, filePath: String) throws -> Data {
        // Simple ZIP file parser
        // ZIP format: Local File Header + File Data + (optional) Data Descriptor
        var position = 0
        let zipData = data

        // Look for Central Directory End (at the end of ZIP file)
        // Central Directory End signature: 0x06054b50 (little-endian: 50 4b 05 06)
        var centralDirStart: Int? = nil
        let searchStart = max(0, zipData.count - 65557) // Max comment length is 65535
        for i in stride(from: zipData.count - 22, through: searchStart, by: -1) {
            if i + 22 <= zipData.count {
                // Check signature (little-endian)
                if zipData[i] == 0x50 && zipData[i+1] == 0x4b && zipData[i+2] == 0x05 && zipData[i+3] == 0x06 {
                    // Read offset of central directory (little-endian, 4 bytes at offset 16)
                    let offset = Int(zipData[i+16]) | (Int(zipData[i+17]) << 8) | (Int(zipData[i+18]) << 16) | (Int(zipData[i+19]) << 24)
                    if offset < zipData.count {
                        centralDirStart = offset
                        break
                    }
                }
            }
        }

        guard let centralDirStart = centralDirStart, centralDirStart < zipData.count else {
            throw DocumentProcessingError.invalidFormat("Invalid ZIP file structure.")
        }

        // Parse Central Directory to find the file
        position = centralDirStart
        while position < zipData.count - 4 {
            // Check signature (little-endian: 50 4b 01 02)
            if position + 46 <= zipData.count &&
               zipData[position] == 0x50 && zipData[position+1] == 0x4b &&
               zipData[position+2] == 0x01 && zipData[position+3] == 0x02 {
                // Central file header found
                let fileNameLength = Int(zipData[position+28]) | (Int(zipData[position+29]) << 8)
                let extraFieldLength = Int(zipData[position+30]) | (Int(zipData[position+31]) << 8)
                let commentLength = Int(zipData[position+32]) | (Int(zipData[position+33]) << 8)

                if position + 46 + fileNameLength <= zipData.count {
                    let fileNameData = zipData.subdata(in: position+46..<position+46+fileNameLength)
                    if let fileName = String(data: fileNameData, encoding: .utf8), fileName == filePath {
                        // Found the file, get local file header offset
                        let localHeaderOffset = Int(zipData[position+42]) | (Int(zipData[position+43]) << 8) | (Int(zipData[position+44]) << 16) | (Int(zipData[position+45]) << 24)

                        // Read local file header
                        if localHeaderOffset + 30 <= zipData.count {
                            let localFileNameLength = Int(zipData[localHeaderOffset+26]) | (Int(zipData[localHeaderOffset+27]) << 8)
                            let localExtraFieldLength = Int(zipData[localHeaderOffset+28]) | (Int(zipData[localHeaderOffset+29]) << 8)
                            let fileDataOffset = localHeaderOffset + 30 + localFileNameLength + localExtraFieldLength

                            // Get compressed size and method
                            let compressedSize = Int(zipData[position+20]) | (Int(zipData[position+21]) << 8) | (Int(zipData[position+22]) << 16) | (Int(zipData[position+23]) << 24)
                            let compressionMethod = Int(zipData[position+10]) | (Int(zipData[position+11]) << 8)

                            if fileDataOffset + compressedSize <= zipData.count {
                                let fileData = zipData.subdata(in: fileDataOffset..<fileDataOffset+compressedSize)

                                // Decompress if needed (method 8 = deflate)
                                if compressionMethod == 8 {
                                    return try decompressDeflate(data: fileData)
                                } else if compressionMethod == 0 {
                                    // Stored (no compression)
                                    return fileData
                                } else {
                                    throw DocumentProcessingError.unsupportedFormat("Unsupported compression method: \(compressionMethod)")
                                }
                            }
                        }
                    }
                }

                // Move to next entry
                position += 46 + fileNameLength + extraFieldLength + commentLength
            } else {
                // Not a central file header, try next position
                position += 1
                if position >= zipData.count - 4 {
                    break
                }
            }
        }

        throw DocumentProcessingError.invalidFormat("File '\(filePath)' not found in ZIP archive.")
    }

    // MARK: - Compression Helper

    /// Decompress DEFLATE compressed data
    private static func decompressDeflate(data: Data) throws -> Data {
        let bufferSize = 32768
        let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: bufferSize)
        defer { buffer.deallocate() }

        var result = Data()
        let stream = UnsafeMutablePointer<compression_stream>.allocate(capacity: 1)
        defer { stream.deallocate() }

        var status = compression_stream_init(stream, COMPRESSION_STREAM_DECODE, COMPRESSION_ZLIB)
        guard status == COMPRESSION_STATUS_OK else {
            throw DocumentProcessingError.invalidFormat("Failed to initialize decompression stream.")
        }
        defer { compression_stream_destroy(stream) }

        data.withUnsafeBytes { bytes in
            stream.pointee.src_ptr = bytes.bindMemory(to: UInt8.self).baseAddress!
            stream.pointee.src_size = data.count

            repeat {
                stream.pointee.dst_ptr = buffer
                stream.pointee.dst_size = bufferSize

                status = compression_stream_process(stream, 0)

                if status == COMPRESSION_STATUS_OK || status == COMPRESSION_STATUS_END {
                    let outputSize = bufferSize - stream.pointee.dst_size
                    result.append(buffer, count: outputSize)
                }
            } while status == COMPRESSION_STATUS_OK
        }

        guard status == COMPRESSION_STATUS_END else {
            throw DocumentProcessingError.invalidFormat("Decompression failed.")
        }

        return result
    }

    // MARK: - XML Text Extraction

    /// Extract text content from XML data
    private static func extractTextFromXML(data: Data) throws -> String {
        let parser = XMLTextExtractor()
        let xmlParser = XMLParser(data: data)
        xmlParser.delegate = parser

        guard xmlParser.parse() else {
            throw DocumentProcessingError.invalidFormat("Failed to parse XML.")
        }

        let text = parser.extractedText.trimmingCharacters(in: .whitespacesAndNewlines)
        if text.isEmpty {
            throw DocumentProcessingError.emptyDocument("XML contains no text content.")
        }

        return text
    }
}

// MARK: - XML Parser Delegate

private class XMLTextExtractor: NSObject, XMLParserDelegate {
    var extractedText = ""
    private var currentText = ""

    func parser(_ parser: XMLParser, foundCharacters string: String) {
        currentText += string
    }

    func parser(_ parser: XMLParser, didEndElement elementName: String, namespaceURI: String?, qualifiedName qName: String?) {
        if !currentText.isEmpty {
            let trimmed = currentText.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty {
                if !extractedText.isEmpty {
                    extractedText += " "
                }
                extractedText += trimmed
                currentText = ""
            }
        }
    }
}

// MARK: - Error Types

enum DocumentProcessingError: LocalizedError {
    case accessDenied
    case invalidFormat(String)
    case emptyDocument(String)
    case fileTooLarge(String)
    case contentTooLong(String)
    case unsupportedFormat(String)

    var errorDescription: String? {
        switch self {
        case .accessDenied:
            return "Access to file was denied. Please grant file access permissions."
        case .invalidFormat(let message):
            return message
        case .emptyDocument(let message):
            return message
        case .fileTooLarge(let message):
            return message
        case .contentTooLong(let message):
            return message
        case .unsupportedFormat(let message):
            return message
        }
    }
}
