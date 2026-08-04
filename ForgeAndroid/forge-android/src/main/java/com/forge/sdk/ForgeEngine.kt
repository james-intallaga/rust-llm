package com.forge.sdk

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.Closeable
import java.util.concurrent.atomic.AtomicLong

/**
 * The main inference engine.
 *
 * This class wraps the Rust ForgeEngine and provides a Kotlin-native API.
 * Memory is automatically managed - the Rust layer uses RAII for all resources.
 *
 * # Example
 *
 * ```kotlin
 * val engine = ForgeEngine.create(modelPath, ForgeConfiguration.auto(context))
 *
 * engine.generateStream("Hello, world!").collect { token ->
 *     print(token)
 * }
 *
 * engine.close()
 * ```
 */
class ForgeEngine private constructor(
    handle: Long
) : Closeable {
    private val nativeHandle = AtomicLong(handle)

    private fun requireHandle(): Long = nativeHandle.get().takeIf { it != 0L }
        ?: throw IllegalStateException("ForgeEngine is closed")

    companion object {
        private var initialized = false

        /**
         * Initialize the Forge backend. Call once at app startup.
         */
        fun initialize() {
            if (!initialized) {
                ForgeNative.init()
                initialized = true
            }
        }

        /**
         * Create an engine for text-only inference.
         *
         * @param modelPath Path to the GGUF model file
         * @param config Engine configuration
         * @throws ForgeError.ModelLoadFailed if model cannot be loaded
         */
        @Throws(ForgeError::class)
        suspend fun create(
            modelPath: String,
            config: ForgeConfiguration = ForgeConfiguration()
        ): ForgeEngine = withContext(Dispatchers.IO) {
            initialize()

            val handle = ForgeNative.engineCreate(
                modelPath,
                config.contextSize,
                config.batchSize,
                config.threads,
                config.maxTokens,
                config.temperature,
                config.topK,
                config.topP,
                config.flashAttention,
                config.gpuLayers
            )

            if (handle == 0L) {
                throw ForgeError.ModelLoadFailed()
            }

            ForgeEngine(handle)
        }

        /**
         * Create an engine with vision capabilities.
         *
         * @param modelPath Path to the GGUF model file
         * @param clipPath Path to the CLIP model file (mmproj-*.gguf)
         * @param config Engine configuration
         * @throws ForgeError.MultimodalLoadFailed if models cannot be loaded
         */
        @Throws(ForgeError::class)
        suspend fun createVision(
            modelPath: String,
            clipPath: String,
            config: ForgeConfiguration = ForgeConfiguration()
        ): ForgeEngine = withContext(Dispatchers.IO) {
            initialize()

            val handle = ForgeNative.engineCreateVision(
                modelPath,
                clipPath,
                config.contextSize,
                config.batchSize,
                config.threads,
                config.maxTokens,
                config.temperature,
                config.topK,
                config.topP,
                config.flashAttention,
                config.gpuLayers
            )

            if (handle == 0L) {
                throw ForgeError.MultimodalLoadFailed()
            }

            ForgeEngine(handle)
        }

        /**
         * Get the default media marker for image prompts (e.g., "<image>").
         */
        fun getMediaMarker(): String {
            initialize()
            return ForgeNative.getMediaMarker()
        }

        /**
         * Cleanup the Forge backend. Call when completely done with inference.
         */
        fun cleanup() {
            if (initialized) {
                ForgeNative.cleanup()
                initialized = false
            }
        }
    }

    /**
     * Check if this is the first turn (context is empty).
     */
    val isFirstTurn: Boolean
        get() = ForgeNative.isFirstTurn(requireHandle())

    /**
     * Get the current context position (tokens processed so far).
     */
    val tokensProcessed: Int
        get() = ForgeNative.nPast(requireHandle())

    /**
     * Get current memory usage in MB.
     */
    val currentMemoryMB: Double
        get() = ForgeNative.memoryCurrentMb(requireHandle())

    /**
     * Get peak memory usage in MB.
     */
    val peakMemoryMB: Double
        get() = ForgeNative.memoryPeakMb(requireHandle())

    /**
     * Generate text from a prompt (single-turn, clears context).
     *
     * @param prompt The input prompt
     * @return Flow of generated tokens
     */
    fun generateStream(prompt: String): Flow<String> = callbackFlow {
        val handle = requireHandle()
        launch(Dispatchers.IO) {
            val result = ForgeNative.generate(handle, prompt, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }

        awaitClose { ForgeNative.cancel(handle) }
    }

    /**
     * Generate a response for a conversation turn (multi-turn aware).
     *
     * @param prompt Formatted prompt for this turn
     * @param addBOS Whether to add BOS token (true for first turn only)
     * @return Flow of generated tokens
     */
    fun generateTurnStream(prompt: String, addBOS: Boolean): Flow<String> = callbackFlow {
        val handle = requireHandle()
        launch(Dispatchers.IO) {
            val result = ForgeNative.generateTurn(handle, prompt, addBOS, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }

        awaitClose { ForgeNative.cancel(handle) }
    }

    /**
     * Generate text from an image and prompt (vision models only).
     *
     * @param imageData Encoded image data (JPEG, PNG)
     * @param prompt Text prompt (should contain the image marker)
     * @return Flow of generated tokens
     */
    fun generateVisionStream(imageData: ByteArray, prompt: String): Flow<String> = callbackFlow {
        val handle = requireHandle()
        launch(Dispatchers.IO) {
            val result = ForgeNative.generateVision(handle, imageData, prompt, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }

        awaitClose { ForgeNative.cancel(handle) }
    }

    /**
     * Reset the conversation context.
     */
    @Throws(ForgeError::class)
    suspend fun reset() = withContext(Dispatchers.IO) {
        val result = ForgeNative.reset(requireHandle())
        ForgeError.fromResult(result)?.let { throw it }
    }

    /**
     * Close and release all resources.
     */
    override fun close() {
        val handle = nativeHandle.getAndSet(0L)
        if (handle != 0L) {
            ForgeNative.cancel(handle)
            ForgeNative.engineDestroy(handle)
        }
    }
}
