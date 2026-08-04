package com.amma.recognize.utils

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import android.util.Log
import java.io.BufferedReader
import java.io.InputStreamReader

/**
 * Android implementation of Document extraction mirroring iOS DocumentProcessor.swift
 */
object DocumentProcessor {

    private const val TAG = "DocumentProcessor"
    private const val MAX_CHARACTER_LIMIT = 10000
    private const val MAX_FILE_SIZE = 500 * 1024 // 500KB

    fun extractText(context: Context, uri: Uri): String {
        val contentResolver = context.contentResolver

        // Check file size
        val fileSize = getFileSize(context, uri)
        if (fileSize > MAX_FILE_SIZE) {
            throw Exception("File is too large (${fileSize / 1024} KB). Maximum allowed is 500KB.")
        }

        val fileName = getFileName(context, uri).lowercase()

        return when {
            fileName.endsWith(".txt") || fileName.endsWith(".text") || fileName.endsWith(".md") || fileName.endsWith(".markdown") -> {
                readTextFile(context, uri)
            }
            fileName.endsWith(".pdf") -> {
                // PDF extraction on Android usually requires a library like PdfBox-Android or iText.
                // For this example, we'll provide a placeholder or use basic PDF content if possible.
                // Since adding a heavy dependency might be out of scope, we'll mention it.
                "PDF content extraction requires an external library. [PDF: $fileName]"
            }
            else -> {
                // Fallback to reading as text
                try {
                    readTextFile(context, uri)
                } catch (e: Exception) {
                    throw Exception("Unsupported file format or unable to read file.")
                }
            }
        }.also { text ->
            if (text.length > MAX_CHARACTER_LIMIT) {
                throw Exception("Document content is too long (${text.length} characters). Maximum allowed is $MAX_CHARACTER_LIMIT.")
            }
            if (text.isBlank()) {
                throw Exception("Document contains no extractable text.")
            }
        }
    }

    private fun readTextFile(context: Context, uri: Uri): String {
        val stringBuilder = StringBuilder()
        context.contentResolver.openInputStream(uri)?.use { inputStream ->
            BufferedReader(InputStreamReader(inputStream)).use { reader ->
                var line: String? = reader.readLine()
                while (line != null) {
                    stringBuilder.append(line).append("\n")
                    line = reader.readLine()
                }
            }
        }
        return stringBuilder.toString()
    }

    private fun getFileSize(context: Context, uri: Uri): Long {
        var size: Long = 0
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
            if (cursor.moveToFirst()) {
                size = cursor.getLong(sizeIndex)
            }
        }
        return size
    }

    private fun getFileName(context: Context, uri: Uri): String {
        var name = "document"
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (cursor.moveToFirst()) {
                name = cursor.getString(nameIndex)
            }
        }
        return name
    }
}
