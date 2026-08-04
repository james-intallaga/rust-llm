package com.amma.recognize.data

import kotlinx.serialization.Serializable
import java.util.UUID

/**
 * UI message for display in chat
 */
data class ChatMessageUI(
    val id: String = UUID.randomUUID().toString(),
    val role: ChatRole,
    val text: String,
    val images: List<String> = emptyList(), // URI strings for images
    val files: List<String> = emptyList()     // Filenames for file attachments
)

enum class ChatRole {
    USER,
    ASSISTANT
}

/**
 * Conversation message for persistence
 */
@Serializable
data class ConversationMessage(
    val role: String, // "user" or "assistant"
    val fullText: String,
    val displayText: String,
    val imageFilenames: List<String> = emptyList(),
    val fileNames: List<String> = emptyList()
)

/**
 * Chat session for history
 */
@Serializable
data class ChatSession(
    val id: String = UUID.randomUUID().toString(),
    var title: String,
    var history: List<ConversationMessage> = emptyList(),
    var updatedAt: Long = System.currentTimeMillis()
)

/**
 * Pending file attachment
 */
data class PendingFile(
    val fileName: String,
    val content: String
)

/**
 * Loading stage enum
 */
enum class LoadingStage {
    PREPARING,
    DOWNLOADING,
    LOADING,
    ERROR;

    val displayText: String
        get() = when (this) {
            PREPARING -> "Preparing..."
            DOWNLOADING -> "Downloading model..."
            LOADING -> "Loading into memory..."
            ERROR -> "Model unavailable"
        }
}
