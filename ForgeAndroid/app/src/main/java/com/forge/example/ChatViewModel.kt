package com.forge.example

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.forge.sdk.ForgeConfiguration
import com.forge.sdk.ForgeEngine
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.*
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

/**
 * ViewModel for the chat screen.
 *
 * This mirrors the VisionViewModel from example-swift, providing:
 * - Model loading and management
 * - Message history
 * - Streaming text generation
 */
class ChatViewModel(application: Application) : AndroidViewModel(application) {

    companion object {
        private const val TAG = "ChatViewModel"

        // Model configuration - update these paths to your models
        private const val MODEL_FILENAME = "LFM2-1.2B-Q4_0.gguf"
        private const val MODEL_URL = "https://huggingface.co/LiquidAI/LFM2-1.2B-GGUF/resolve/main/LFM2-1.2B-Q4_0.gguf"
    }

    private var engine: ForgeEngine? = null

    private val _messages = MutableStateFlow<List<ChatMessage>>(emptyList())
    val messages: StateFlow<List<ChatMessage>> = _messages.asStateFlow()

    private val _isGenerating = MutableStateFlow(false)
    val isGenerating: StateFlow<Boolean> = _isGenerating.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val context = application.applicationContext
    private val modelsDir = File(context.filesDir, "models")

    init {
        loadModel()
    }

    private fun loadModel() {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null

            try {
                modelsDir.mkdirs()
                val modelFile = File(modelsDir, MODEL_FILENAME)

                // Check if model exists
                if (!modelFile.exists()) {
                    _error.value = "Model not found. Please place $MODEL_FILENAME in ${modelsDir.absolutePath}"
                    Log.w(TAG, "Model file not found at: ${modelFile.absolutePath}")
                    Log.i(TAG, "Download from: $MODEL_URL")
                    return@launch
                }

                Log.i(TAG, "Loading model from: ${modelFile.absolutePath}")

                // Create engine with auto-configured settings
                val config = ForgeConfiguration.auto(context)
                engine = ForgeEngine.create(modelFile.absolutePath, config)

                Log.i(TAG, "Model loaded successfully")

            } catch (e: Exception) {
                Log.e(TAG, "Failed to load model", e)
                _error.value = "Failed to load model: ${e.message}"
            } finally {
                _isLoading.value = false
            }
        }
    }

    fun sendMessage(userMessage: String) {
        val currentEngine = engine
        if (currentEngine == null) {
            _error.value = "Model not loaded"
            return
        }

        viewModelScope.launch {
            // Add user message immediately
            _messages.update { it + ChatMessage("user", userMessage) }

            _isGenerating.value = true
            _error.value = null

            try {
                // Format the prompt
                val prompt = formatPrompt(userMessage)
                val isFirst = currentEngine.isFirstTurn

                Log.d(TAG, "Generating (isFirst=$isFirst): $prompt")

                // Add empty assistant message that we'll update
                val assistantIndex = _messages.value.size
                _messages.update { it + ChatMessage("assistant", "") }

                // Collect streaming tokens
                val responseBuilder = StringBuilder()

                currentEngine.generateTurnStream(prompt, addBOS = isFirst)
                    .catch { e ->
                        Log.e(TAG, "Generation error", e)
                        _error.value = "Generation failed: ${e.message}"
                    }
                    .collect { token ->
                        responseBuilder.append(token)
                        // Update the assistant message in place
                        _messages.update { messages ->
                            messages.toMutableList().also {
                                if (assistantIndex < it.size) {
                                    it[assistantIndex] = ChatMessage("assistant", responseBuilder.toString())
                                }
                            }
                        }
                    }

                Log.d(TAG, "Generated ${responseBuilder.length} chars")

            } catch (e: Exception) {
                Log.e(TAG, "Send failed", e)
                _error.value = "Error: ${e.message}"
            } finally {
                _isGenerating.value = false
            }
        }
    }

    private fun formatPrompt(message: String): String {
        // ChatML format
        val isFirst = engine?.isFirstTurn == true

        return buildString {
            if (isFirst) {
                append("<|im_start|>system\n")
                append("You are a helpful assistant.\n")
                append("<|im_end|>\n")
            }
            append("<|im_start|>user\n")
            append(message)
            append("<|im_end|>\n")
            append("<|im_start|>assistant\n")
        }
    }

    fun clearChat() {
        viewModelScope.launch {
            try {
                engine?.reset()
                _messages.value = emptyList()
                _error.value = null
            } catch (e: Exception) {
                Log.e(TAG, "Clear failed", e)
            }
        }
    }

    override fun onCleared() {
        super.onCleared()
        engine?.close()
    }
}
