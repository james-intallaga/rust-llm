package com.forge.sdk

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.Closeable

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
    private val lifecycleMonitor = Object()
    private var nativeHandle: Long = handle
    private var activeOperations = 0
    private var closing = false

    private fun acquireHandle(): Long = synchronized(lifecycleMonitor) {
        check(!closing && nativeHandle != 0L) { "ForgeEngine is closed" }
        activeOperations += 1
        nativeHandle
    }

    private fun releaseHandle() = synchronized(lifecycleMonitor) {
        check(activeOperations > 0) { "Unbalanced native operation" }
        activeOperations -= 1
        if (activeOperations == 0) {
            lifecycleMonitor.notifyAll()
        }
    }

    private inline fun <T> withHandle(block: (Long) -> T): T {
        val handle = acquireHandle()
        return try {
            block(handle)
        } finally {
            releaseHandle()
        }
    }

    private fun requestCancellation() = synchronized(lifecycleMonitor) {
        if (nativeHandle != 0L) {
            ForgeNative.cancel(nativeHandle)
        }
    }

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
            config.validate()

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
            config.validate()

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
        get() = withHandle(ForgeNative::isFirstTurn)

    /**
     * Get the current context position (tokens processed so far).
     */
    val tokensProcessed: Int
        get() = withHandle(ForgeNative::nPast)

    /**
     * Get current memory usage in MB.
     */
    val currentMemoryMB: Double
        get() = withHandle(ForgeNative::memoryCurrentMb)

    /**
     * Get peak memory usage in MB.
     */
    val peakMemoryMB: Double
        get() = withHandle(ForgeNative::memoryPeakMb)

    /**
     * Generate text from a prompt (single-turn, clears context).
     *
     * @param prompt The input prompt
     * @return Flow of generated tokens
     */
    fun generateStream(prompt: String): Flow<String> = callbackFlow {
        val handle = acquireHandle()
        val operation = launch(Dispatchers.IO) {
            val result = ForgeNative.generate(handle, prompt, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }
        operation.invokeOnCompletion { releaseHandle() }

        awaitClose {
            requestCancellation()
            operation.cancel()
        }
    }

    /**
     * Generate a response for a conversation turn (multi-turn aware).
     *
     * @param prompt Formatted prompt for this turn
     * @param addBOS Whether to add BOS token (true for first turn only)
     * @return Flow of generated tokens
     */
    fun generateTurnStream(prompt: String, addBOS: Boolean): Flow<String> = callbackFlow {
        val handle = acquireHandle()
        val operation = launch(Dispatchers.IO) {
            val result = ForgeNative.generateTurn(handle, prompt, addBOS, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }
        operation.invokeOnCompletion { releaseHandle() }

        awaitClose {
            requestCancellation()
            operation.cancel()
        }
    }

    /**
     * Generate text from an image and prompt (vision models only).
     *
     * @param imageData Encoded image data (JPEG, PNG)
     * @param prompt Text prompt (should contain the image marker)
     * @return Flow of generated tokens
     */
    fun generateVisionStream(imageData: ByteArray, prompt: String): Flow<String> = callbackFlow {
        require(imageData.isNotEmpty()) { "imageData must not be empty" }
        require(imageData.size <= 64 * 1024 * 1024) { "imageData must not exceed 64 MiB" }
        val handle = acquireHandle()
        val operation = launch(Dispatchers.IO) {
            val result = ForgeNative.generateVision(handle, imageData, prompt, object : TokenCallback {
                override fun onToken(token: String) {
                    trySend(token)
                }
            })

            ForgeError.fromResult(result)?.let { error ->
                close(error)
            } ?: close()
        }
        operation.invokeOnCompletion { releaseHandle() }

        awaitClose {
            requestCancellation()
            operation.cancel()
        }
    }

    /**
     * Reset the conversation context.
     */
    @Throws(ForgeError::class)
    suspend fun reset() = withContext(Dispatchers.IO) {
        val result = withHandle(ForgeNative::reset)
        ForgeError.fromResult(result)?.let { throw it }
    }

    /**
     * Close and release all resources.
     */
    override fun close() {
        var interrupted = false
        val handle = synchronized(lifecycleMonitor) {
            if (closing || nativeHandle == 0L) {
                return
            }
            closing = true
            ForgeNative.cancel(nativeHandle)
            while (activeOperations > 0) {
                try {
                    lifecycleMonitor.wait()
                } catch (_: InterruptedException) {
                    interrupted = true
                }
            }
            nativeHandle.also { nativeHandle = 0L }
        }

        ForgeNative.engineDestroy(handle)
        if (interrupted) {
            Thread.currentThread().interrupt()
        }
    }
}
