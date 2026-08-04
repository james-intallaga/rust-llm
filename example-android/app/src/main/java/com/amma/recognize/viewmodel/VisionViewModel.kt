package com.amma.recognize.viewmodel

import android.app.Application
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.amma.recognize.data.*
import com.amma.recognize.utils.DocumentProcessor
import com.amma.recognize.utils.SpeechRecognizerHelper
import com.forge.sdk.ForgeConfiguration
import android.provider.OpenableColumns
import com.forge.sdk.ForgeEngine
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.*
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

/**
 * ViewModel for the vision chat screen.
 *
 * This mirrors VisionViewModel.swift from example-swift, providing:
 * - Model loading with download progress
 * - Chat session management
 * - Image recognition with streaming
 * - Conversation history
 */
class VisionViewModel(application: Application) : AndroidViewModel(application) {

    companion object {
        private const val TAG = "VisionViewModel"
        private const val PREFS_NAME = "amma_prefs"
        private const val KEY_SESSIONS = "chat_sessions"
        private const val KEY_USER_NAME = "user_name"
        private const val KEY_USER_BIRTHDAY = "user_birthday"

        // Model configuration
        private const val MODEL_SLUG = "LFM2.5-VL-450M"
        private const val QUANTIZATION_SLUG = "Q4_K_M"
        private const val MODEL_FILENAME = "$MODEL_SLUG-$QUANTIZATION_SLUG.gguf"
        private const val CLIP_FILENAME = "mmproj-LFM2.5-VL-450m-Q8_0.gguf"
        private const val MODEL_SIZE = 229_313_568L
        private const val CLIP_SIZE = 102_815_168L
        private const val MODEL_URL = "https://huggingface.co/LiquidAI/$MODEL_SLUG-GGUF/resolve/main/$MODEL_FILENAME?download=true"
        private const val CLIP_URL = "https://huggingface.co/LiquidAI/$MODEL_SLUG-GGUF/resolve/main/$CLIP_FILENAME?download=true"
    }

    private val context = application.applicationContext
    private val prefs = context.getSharedPreferences(PREFS_NAME, 0)
    private val modelsDir = File(context.filesDir, "models")
    private val imagesDir = File(context.filesDir, "chat_images")

    private var engine: ForgeEngine? = null
    private var visionEngine: ForgeEngine? = null
    private var generationJob: Job? = null
    private val speechRecognizerHelper = SpeechRecognizerHelper(context)

    // UI State
    private val _isLoading = MutableStateFlow(true)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _loadingStatus = MutableStateFlow("Loading...")
    val loadingStatus: StateFlow<String> = _loadingStatus.asStateFlow()

    private val _loadingProgress = MutableStateFlow(0.0)
    val loadingProgress: StateFlow<Double> = _loadingProgress.asStateFlow()

    private val _loadingStage = MutableStateFlow(LoadingStage.PREPARING)
    val loadingStage: StateFlow<LoadingStage> = _loadingStage.asStateFlow()

    private val _messages = MutableStateFlow<List<ChatMessageUI>>(emptyList())
    val messages: StateFlow<List<ChatMessageUI>> = _messages.asStateFlow()

    private val _currentResponse = MutableStateFlow("")
    val currentResponse: StateFlow<String> = _currentResponse.asStateFlow()

    private val _isGenerating = MutableStateFlow(false)
    val isGenerating: StateFlow<Boolean> = _isGenerating.asStateFlow()

    val isRecording: StateFlow<Boolean> = speechRecognizerHelper.isRecording
    val transcript: StateFlow<String> = speechRecognizerHelper.transcript

    private val _pendingImages = MutableStateFlow<List<Uri>>(emptyList())
    val pendingImages: StateFlow<List<Uri>> = _pendingImages.asStateFlow()

    private val _pendingFile = MutableStateFlow<PendingFile?>(null)
    val pendingFile: StateFlow<PendingFile?> = _pendingFile.asStateFlow()

    // Session management
    private val _sessions = MutableStateFlow<List<ChatSession>>(emptyList())
    val sessions: StateFlow<List<ChatSession>> = _sessions.asStateFlow()

    private val _currentSessionId = MutableStateFlow<String?>(null)
    val currentSessionId: StateFlow<String?> = _currentSessionId.asStateFlow()

    private val _searchText = MutableStateFlow("")
    val searchText: StateFlow<String> = _searchText.asStateFlow()

    // Conversation history for prompt formatting
    private var conversationHistory = mutableListOf<ConversationMessage>()

    val isModelLoaded: Boolean
        get() = engine != null

    val filteredSessions: Flow<List<ChatSession>> = combine(sessions, searchText) { sessions, query ->
        val meaningful = sessions.filter { session ->
            session.history.any { it.role == "user" }
        }
        if (query.isBlank()) {
            meaningful
        } else {
            meaningful.filter { session ->
                session.title.contains(query, ignoreCase = true) ||
                session.history.any { it.fullText.contains(query, ignoreCase = true) }
            }
        }
    }

    private val systemPrompt: String
        get() {
            val userName = prefs.getString(KEY_USER_NAME, "") ?: ""
            return if (userName.isBlank()) {
                "You are Amma, a private personal assistant running on the user's device. Help with writing, planning, explanations, brainstorming, documents, and images. Be concise, practical, and honest. Never claim to have current or live information unless the user provides it. Ask a clarifying question only when it is necessary."
            } else {
                "You are Amma, $userName's private personal assistant running on their device. Help with writing, planning, explanations, brainstorming, documents, and images. Be concise, practical, and honest. Never claim to have current or live information unless the user provides it. Use $userName's name occasionally, and ask a clarifying question only when it is necessary."
            }
        }

    private val greetingMessage: String
        get() {
            val userName = prefs.getString(KEY_USER_NAME, "") ?: ""
            return if (userName.isBlank()) {
                "Hello! How can I help?"
            } else {
                "Hello, $userName. How can I help?"
            }
        }

    init {
        modelsDir.mkdirs()
        imagesDir.mkdirs()
        loadSessions()
    }

    // MARK: - Session Management

    private fun loadSessions() {
        try {
            val json = prefs.getString(KEY_SESSIONS, null)
            if (json != null) {
                _sessions.value = Json.decodeFromString<List<ChatSession>>(json).map { session ->
                    session.copy(history = session.history.map { message ->
                        if (message.role == "assistant" && message.displayText.contains("Show me a plant or animal")) {
                            message.copy(fullText = greetingMessage, displayText = greetingMessage)
                        } else {
                            message
                        }
                    })
                }
                saveSessions()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to load sessions", e)
        }
    }

    private fun saveSessions() {
        try {
            val json = Json.encodeToString(_sessions.value)
            prefs.edit().putString(KEY_SESSIONS, json).apply()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save sessions", e)
        }
    }

    fun createNewChat() {
        if (engine == null) return

        updateCurrentSession()

        // Remove empty current session
        _currentSessionId.value?.let { currentId ->
            val idx = _sessions.value.indexOfFirst { it.id == currentId }
            if (idx >= 0) {
                val session = _sessions.value[idx]
                val hasUserMessages = session.history.any { it.role == "user" }
                if (!hasUserMessages && session.title == "New Chat") {
                    _sessions.value = _sessions.value.toMutableList().also { it.removeAt(idx) }
                }
            }
        }

        val newSession = ChatSession(title = "New Chat")
        _sessions.value = listOf(newSession) + _sessions.value
        _currentSessionId.value = newSession.id
        _messages.value = emptyList()
        _pendingImages.value = emptyList()
        _pendingFile.value = null
        conversationHistory.clear()

        viewModelScope.launch {
            try {
                resetEngines()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to reset engine", e)
            }
        }

        saveSessions()
    }

    fun loadSession(session: ChatSession) {
        if (engine == null) return

        updateCurrentSession()

        _currentSessionId.value = session.id

        // Rebuild messages from history
        _messages.value = session.history.map { msg ->
            val role = if (msg.role == "user") ChatRole.USER else ChatRole.ASSISTANT
            val imageUris = msg.imageFilenames.mapNotNull { loadImageUri(it) }
            ChatMessageUI(
                role = role,
                text = msg.displayText,
                images = imageUris,
                files = msg.fileNames
            )
        }

        conversationHistory = session.history.toMutableList()

        viewModelScope.launch {
            try {
                resetEngines()
            } catch (e: Exception) {
                Log.e(TAG, "Failed to reset engine", e)
            }
        }

        // Move to top
        val idx = _sessions.value.indexOfFirst { it.id == session.id }
        if (idx > 0) {
            _sessions.value = _sessions.value.toMutableList().also {
                val s = it.removeAt(idx)
                it.add(0, s)
            }
        }
    }

    private fun updateCurrentSession() {
        val currentId = _currentSessionId.value ?: return
        val idx = _sessions.value.indexOfFirst { it.id == currentId }
        if (idx < 0) return

        _sessions.value = _sessions.value.toMutableList().also { list ->
            val session = list[idx]

            // Update title
            var newTitle = session.title
            if (newTitle == "New Chat" || newTitle == "[Image attached]" || newTitle == "[File attached]") {
                val firstText = _messages.value.firstOrNull {
                    it.role == ChatRole.USER &&
                    !it.text.startsWith("[") && it.text.isNotBlank()
                }
                if (firstText != null) {
                    newTitle = firstText.text.take(30) + if (firstText.text.length > 30) "..." else ""
                } else {
                    val firstImage = _messages.value.firstOrNull { it.role == ChatRole.USER && it.images.isNotEmpty() }
                    if (firstImage != null) {
                        newTitle = "Image Recognition"
                    } else {
                        val firstFile = _messages.value.firstOrNull { it.role == ChatRole.USER && it.files.isNotEmpty() }
                        if (firstFile != null) {
                            newTitle = "File Analysis"
                        }
                    }
                }
            }

            list[idx] = session.copy(
                title = newTitle,
                history = conversationHistory.toList(),
                updatedAt = System.currentTimeMillis()
            )
        }

        saveSessions()
    }

    fun deleteSession(session: ChatSession) {
        // Delete image files
        session.history.forEach { msg ->
            msg.imageFilenames.forEach { deleteImage(it) }
        }

        _sessions.value = _sessions.value.filter { it.id != session.id }

        if (_currentSessionId.value == session.id) {
            val newSession = ChatSession(title = "New Chat")
            _sessions.value = listOf(newSession) + _sessions.value
            _currentSessionId.value = newSession.id
            _messages.value = emptyList()
            _pendingImages.value = emptyList()
            _pendingFile.value = null
            conversationHistory.clear()

            viewModelScope.launch {
                try {
                    resetEngines()
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to reset engine", e)
                }
            }
        }

        saveSessions()
    }

    fun renameSession(session: ChatSession, newTitle: String) {
        if (newTitle.isBlank()) return

        val idx = _sessions.value.indexOfFirst { it.id == session.id }
        if (idx >= 0) {
            _sessions.value = _sessions.value.toMutableList().also {
                it[idx] = it[idx].copy(title = newTitle.trim())
            }
            saveSessions()
        }
    }

    fun setSearchText(text: String) {
        _searchText.value = text
    }

    // MARK: - Model Loading

    fun loadModel() {
        if (engine != null) return

        viewModelScope.launch {
            _isLoading.value = true
            _loadingStage.value = LoadingStage.PREPARING
            _loadingStatus.value = "Preparing model..."
            _loadingProgress.value = 0.0

            try {
                val modelFile = File(modelsDir, MODEL_FILENAME)
                val clipFile = File(modelsDir, CLIP_FILENAME)

                removeIncompleteFile(modelFile, MODEL_SIZE)
                removeIncompleteFile(clipFile, CLIP_SIZE)

                // Download model if needed
                if (!modelFile.exists()) {
                    _loadingStage.value = LoadingStage.DOWNLOADING
                    _loadingStatus.value = "Downloading model..."
                    downloadFile(MODEL_URL, modelFile, MODEL_SIZE) { progress, speed ->
                        _loadingProgress.value = progress * 0.5 // 0-50%
                        val speedMB = speed / (1024 * 1024)
                        _loadingStatus.value = "Downloading: ${(progress * 100).toInt()}% • ${String.format("%.1f", speedMB)} MB/s"
                    }
                }

                // Download CLIP if needed
                if (!clipFile.exists()) {
                    _loadingStage.value = LoadingStage.DOWNLOADING
                    _loadingStatus.value = "Downloading vision model..."
                    downloadFile(CLIP_URL, clipFile, CLIP_SIZE) { progress, speed ->
                        _loadingProgress.value = 0.5 + progress * 0.4 // 50-90%
                        val speedMB = speed / (1024 * 1024)
                        _loadingStatus.value = "Downloading vision: ${(progress * 100).toInt()}% • ${String.format("%.1f", speedMB)} MB/s"
                    }
                }

                // Load model
                _loadingStage.value = LoadingStage.LOADING
                _loadingStatus.value = "Loading model into memory..."
                _loadingProgress.value = 0.9

                val visionConfig = ForgeConfiguration.vision(context)
                visionConfig.temperature = 0.1f
                visionConfig.maxTokens = 256
                val newVisionEngine = ForgeEngine.createVision(
                    modelFile.absolutePath,
                    clipFile.absolutePath,
                    visionConfig
                )

                val textConfig = ForgeConfiguration.auto(context)
                textConfig.contextSize = maxOf(textConfig.contextSize, 2048)
                textConfig.temperature = 0.1f
                textConfig.maxTokens = 256
                val newTextEngine = try {
                    ForgeEngine.create(modelFile.absolutePath, textConfig)
                } catch (e: Exception) {
                    newVisionEngine.close()
                    throw e
                }

                engine = newTextEngine
                visionEngine = newVisionEngine

                Log.i(TAG, "Model loaded successfully")

                _loadingProgress.value = 1.0
                _loadingStatus.value = "Almost ready..."

                // Reclaim storage from the previous mobile model only after the new one is ready.
                File(modelsDir, "LFM2.5-VL-1.6B-Q4_0.gguf").delete()
                File(modelsDir, "LFM2.5-VL-1.6B-Q8_0.gguf").delete()
                File(modelsDir, "mmproj-LFM2.5-VL-1.6b-Q8_0.gguf").delete()

                delay(300) // Small delay for visual completion

                // Initialize first session
                if (_sessions.value.isEmpty()) {
                    val initialSession = ChatSession(title = "New Chat")
                    _sessions.value = listOf(initialSession)
                    _currentSessionId.value = initialSession.id
                    _messages.value = listOf(ChatMessageUI(role = ChatRole.ASSISTANT, text = greetingMessage))
                } else {
                    _sessions.value.firstOrNull()?.let { loadSession(it) }
                }

                _isLoading.value = false

            } catch (e: Exception) {
                Log.e(TAG, "Failed to load model", e)
                _loadingStatus.value = "Failed: ${e.message}"
                _loadingStage.value = LoadingStage.ERROR
            }
        }
    }

    private fun removeIncompleteFile(file: File, expectedSize: Long) {
        File(file.parentFile, "${file.name}.part").delete()
        if (file.exists() && file.length() != expectedSize) {
            Log.w(TAG, "Removing incomplete ${file.name}: ${file.length()} of $expectedSize bytes")
            file.delete()
        }
    }

    private suspend fun downloadFile(
        urlString: String,
        destination: File,
        expectedSize: Long,
        onProgress: (Double, Double) -> Unit
    ) = withContext(Dispatchers.IO) {
        val partial = File(destination.parentFile, "${destination.name}.part")
        partial.delete()
        val url = URL(urlString)
        val connection = url.openConnection() as HttpURLConnection
        connection.requestMethod = "GET"
        connection.connectTimeout = 30000
        connection.readTimeout = 30000
        connection.instanceFollowRedirects = true

        try {
            val responseCode = connection.responseCode
            if (responseCode != HttpURLConnection.HTTP_OK) {
                throw IOException("Download failed: HTTP $responseCode")
            }

            val contentLength = connection.contentLengthLong
            var downloaded = 0L
            var lastUpdate = System.currentTimeMillis()
            var lastBytes = 0L

            connection.inputStream.buffered().use { input ->
                FileOutputStream(partial).buffered().use { output ->
                    val buffer = ByteArray(65536)
                    var bytesRead: Int

                    while (input.read(buffer).also { bytesRead = it } != -1) {
                        output.write(buffer, 0, bytesRead)
                        downloaded += bytesRead

                        val now = System.currentTimeMillis()
                        if (now - lastUpdate >= 100) {
                            val progress = if (contentLength > 0) downloaded.toDouble() / contentLength else 0.0
                            val elapsed = (now - lastUpdate) / 1000.0
                            val speed = if (elapsed > 0) (downloaded - lastBytes) / elapsed else 0.0

                            withContext(Dispatchers.Main) {
                                onProgress(progress, speed)
                            }

                            lastUpdate = now
                            lastBytes = downloaded
                        }
                    }
                }
            }
            if (downloaded != expectedSize) {
                throw IOException("Incomplete download: received $downloaded of $expectedSize bytes")
            }
            if (destination.exists() && !destination.delete()) {
                throw IOException("Could not replace ${destination.name}")
            }
            if (!partial.renameTo(destination)) {
                throw IOException("Could not finish ${destination.name}")
            }
        } catch (e: Exception) {
            partial.delete()
            throw e
        } finally {
            connection.disconnect()
        }
    }

    // MARK: - Image Recognition

    fun addPendingImage(uri: Uri) {
        _pendingImages.value = _pendingImages.value + uri
    }

    fun removePendingImage(index: Int) {
        val current = _pendingImages.value.toMutableList()
        if (index in current.indices) {
            current.removeAt(index)
            _pendingImages.value = current
        }
    }

    fun setPendingImages(uris: List<Uri>) {
        _pendingImages.value = uris
    }

    fun setPendingFile(file: PendingFile?) {
        _pendingFile.value = file
    }

    fun processFile(uri: Uri) {
        viewModelScope.launch {
            try {
                val content = withContext(Dispatchers.IO) {
                    DocumentProcessor.extractText(context, uri)
                }

                val fileName = getFileName(uri)
                setPendingFile(PendingFile(fileName, content))

            } catch (e: Exception) {
                Log.e(TAG, "Failed to process file", e)
                _messages.value = _messages.value + ChatMessageUI(
                    role = ChatRole.ASSISTANT,
                    text = "❌ Error: ${e.message}"
                )
            }
        }
    }

    private fun getFileName(uri: Uri): String {
        var name = "document"
        context.contentResolver.query(uri, null, null, null, null)?.use { cursor ->
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (cursor.moveToFirst()) {
                name = cursor.getString(nameIndex)
            }
        }
        return name
    }

    fun startListening() {
        speechRecognizerHelper.startListening()
    }

    fun stopListening() {
        speechRecognizerHelper.stopListening()
    }

    fun recognizeImage(imageUris: List<Uri>, text: String = "") {
        val currentEngine = visionEngine ?: return

        generationJob?.cancel()

        val displayText = if (text.isBlank()) "[Image attached]" else text
        val promptText = if (text.isBlank()) "Describe this image and help me understand what matters." else text

        // Add user message immediately
        _messages.value = _messages.value + ChatMessageUI(
            role = ChatRole.USER,
            text = displayText,
            images = imageUris.map { it.toString() }
        )

        _currentResponse.value = ""
        _isGenerating.value = true
        _pendingImages.value = emptyList()

        generationJob = viewModelScope.launch {
            try {
                // Compress and save images
                val imageFilenames = mutableListOf<String>()
                val jpegDatas = mutableListOf<ByteArray>()

                imageUris.forEach { uri ->
                    val data = withContext(Dispatchers.IO) { compressImage(uri) }
                    jpegDatas.add(data)
                    saveImage(data)?.let { imageFilenames.add(it) }
                }

                // Add to history
                conversationHistory.add(ConversationMessage(
                    role = "user",
                    fullText = promptText,
                    displayText = displayText,
                    imageFilenames = imageFilenames
                ))

                // Vision requests are rebuilt from text history so an old bitmap can never leak
                // into a new request. The latest image is the only active media marker.
                currentEngine.reset()
                val fullPrompt = formatPrompt(
                    systemPrompt,
                    promptText,
                    isFirstTurn = true,
                    includeActiveImage = true
                )

                Log.d(TAG, "Vision prompt: $fullPrompt")

                // Generate - only use the first image for now as model supports 1
                val mainImage = jpegDatas.firstOrNull() ?: throw Exception("No image data")

                currentEngine.generateVisionStream(mainImage, fullPrompt)
                    .catch { e ->
                        Log.e(TAG, "Vision generation error", e)
                    }
                    .collect { token ->
                        if (!isActive) return@collect
                        _currentResponse.value += token
                    }

                if (isActive && _currentResponse.value.isNotBlank()) {
                    _messages.value = _messages.value + ChatMessageUI(
                        role = ChatRole.ASSISTANT,
                        text = _currentResponse.value
                    )
                    conversationHistory.add(ConversationMessage(
                        role = "assistant",
                        fullText = _currentResponse.value,
                        displayText = _currentResponse.value
                    ))
                    _currentResponse.value = ""

                    // The text engine did not see this vision turn. Force a one-time replay of
                    // textual history on the next normal assistant request.
                    engine?.reset()
                }

                updateCurrentSession()

            } catch (e: Exception) {
                Log.e(TAG, "Vision processing failed", e)
                if (isActive) {
                    _messages.value = _messages.value + ChatMessageUI(
                        role = ChatRole.ASSISTANT,
                        text = "Sorry, I couldn't process the image. Error: ${e.message}"
                    )
                }
            } finally {
                _isGenerating.value = false
            }
        }
    }

    private fun compressImage(uri: Uri): ByteArray {
        val inputStream = context.contentResolver.openInputStream(uri)
            ?: throw IOException("Cannot open image")

        val original = BitmapFactory.decodeStream(inputStream)
        inputStream.close()

        // Resize if needed (max 768px on longest side)
        val maxDim = 768
        val scale = if (maxOf(original.width, original.height) > maxDim) {
            maxDim.toFloat() / maxOf(original.width, original.height)
        } else 1f

        val bitmap = if (scale < 1f) {
            Bitmap.createScaledBitmap(
                original,
                (original.width * scale).toInt(),
                (original.height * scale).toInt(),
                true
            ).also { if (it != original) original.recycle() }
        } else {
            original
        }

        val outputStream = ByteArrayOutputStream()
        bitmap.compress(Bitmap.CompressFormat.JPEG, 70, outputStream)
        bitmap.recycle()

        return outputStream.toByteArray()
    }

    // MARK: - Text Follow-up

    fun sendFollowUp(text: String): Boolean {
        val currentEngine = engine ?: return false

        // If there's a pending image, use vision
        if (_pendingImages.value.isNotEmpty()) {
            recognizeImage(_pendingImages.value, text)
            return true
        }

        // Handle pending file
        var messageText = text
        var displayText = text
        val fileNames = mutableListOf<String>()

        _pendingFile.value?.let { file ->
            messageText = if (text.isBlank()) {
                "Please analyze this document:\n\n${file.content}"
            } else {
                "$text\n\n--- Document Content ---\n${file.content}"
            }
            displayText = if (text.isBlank()) "[File attached]" else text
            fileNames.add(file.fileName)
            _pendingFile.value = null
        }

        if (messageText.isBlank()) return false

        // Add user message immediately
        _messages.value = _messages.value + ChatMessageUI(
            role = ChatRole.USER,
            text = displayText,
            files = fileNames
        )

        conversationHistory.add(ConversationMessage(
            role = "user",
            fullText = text,
            displayText = displayText,
            fileNames = fileNames
        ))

        generationJob?.cancel()
        _currentResponse.value = ""
        _isGenerating.value = true

                val fullPrompt = formatPrompt(
                    systemPrompt,
                    messageText,
                    isFirstTurn = currentEngine.isFirstTurn,
                    includeActiveImage = false
                )
        val isFirst = currentEngine.isFirstTurn

        generationJob = viewModelScope.launch {
            try {
                Log.d(TAG, "Follow-up prompt (isFirst=$isFirst): $fullPrompt")

                currentEngine.generateTurnStream(fullPrompt, addBOS = isFirst)
                    .catch { e ->
                        Log.e(TAG, "Generation error", e)
                    }
                    .collect { token ->
                        if (!isActive) return@collect
                        _currentResponse.value += token
                    }

                if (isActive && _currentResponse.value.isNotBlank()) {
                    _messages.value = _messages.value + ChatMessageUI(
                        role = ChatRole.ASSISTANT,
                        text = _currentResponse.value
                    )
                    conversationHistory.add(ConversationMessage(
                        role = "assistant",
                        fullText = _currentResponse.value,
                        displayText = _currentResponse.value
                    ))
                    _currentResponse.value = ""
                }

                updateCurrentSession()

            } catch (e: Exception) {
                Log.e(TAG, "Follow-up failed", e)
                if (isActive) {
                    _messages.value = _messages.value + ChatMessageUI(
                        role = ChatRole.ASSISTANT,
                        text = "Sorry, I couldn't process that. Please try again."
                    )
                }
            } finally {
                _isGenerating.value = false
            }
        }
        return true
    }

    fun stopGeneration() {
        generationJob?.cancel()
        generationJob = null

        if (_currentResponse.value.isNotBlank()) {
            _messages.value = _messages.value + ChatMessageUI(
                role = ChatRole.ASSISTANT,
                text = _currentResponse.value
            )
            conversationHistory.add(ConversationMessage(
                role = "assistant",
                fullText = _currentResponse.value,
                displayText = _currentResponse.value
            ))
            _currentResponse.value = ""
        }

        _isGenerating.value = false
        updateCurrentSession()
    }

    // MARK: - Prompt Formatting

    private fun formatPrompt(
        systemPrompt: String,
        userMessage: String,
        isFirstTurn: Boolean,
        includeActiveImage: Boolean
    ): String {
        val sb = StringBuilder()
        if (isFirstTurn) {
            sb.append("<|im_start|>system\n$systemPrompt<|im_end|>\n")

            val latestImageIndex = conversationHistory.indexOfLast {
                it.role == "user" && it.imageFilenames.isNotEmpty()
            }
            conversationHistory.forEachIndexed { index, message ->
                val role = if (message.role == "user") "user" else "assistant"
                val content = if (includeActiveImage && index == latestImageIndex) {
                    "${ForgeEngine.getMediaMarker()}\n${message.fullText}"
                } else {
                    message.fullText
                }
                sb.append("<|im_start|>$role\n$content<|im_end|>\n")
            }
        } else {
            sb.append("<|im_start|>user\n$userMessage<|im_end|>\n")
        }
        sb.append("<|im_start|>assistant\n")
        return sb.toString()
    }

    private suspend fun resetEngines() {
        engine?.reset()
        if (visionEngine !== engine) {
            visionEngine?.reset()
        }
    }

    // MARK: - Image Storage

    private fun saveImage(data: ByteArray): String? {
        return try {
            val filename = "${UUID.randomUUID()}.jpg"
            val file = File(imagesDir, filename)
            FileOutputStream(file).use { it.write(data) }
            filename
        } catch (e: Exception) {
            Log.e(TAG, "Failed to save image", e)
            null
        }
    }

    private fun loadImageUri(filename: String): String? {
        val file = File(imagesDir, filename)
        return if (file.exists()) Uri.fromFile(file).toString() else null
    }

    private fun deleteImage(filename: String) {
        try {
            File(imagesDir, filename).delete()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to delete image", e)
        }
    }

    // MARK: - Settings

    fun getUserName(): String {
        return prefs.getString(KEY_USER_NAME, "") ?: ""
    }

    fun setUserName(name: String) {
        prefs.edit().putString(KEY_USER_NAME, name.trim()).apply()
    }

    fun getUserBirthday(): String {
        return prefs.getString(KEY_USER_BIRTHDAY, "") ?: ""
    }

    fun setUserBirthday(birthday: String) {
        prefs.edit().putString(KEY_USER_BIRTHDAY, birthday.trim()).apply()
    }

    // MARK: - Lifecycle

    override fun onCleared() {
        super.onCleared()
        generationJob?.cancel()
        engine?.close()
        if (visionEngine !== engine) {
            visionEngine?.close()
        }
        speechRecognizerHelper.release()
    }
}
