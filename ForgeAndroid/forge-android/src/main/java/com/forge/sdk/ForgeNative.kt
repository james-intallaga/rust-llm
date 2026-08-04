package com.forge.sdk

/**
 * Low-level JNI bindings to the Rust Forge core.
 *
 * This class provides direct access to the native functions.
 * Most users should use [ForgeEngine] instead, which provides
 * a higher-level, Kotlin-friendly API.
 */
internal object ForgeNative {

    init {
        // Load all required libraries in dependency order
        System.loadLibrary("omp")
        System.loadLibrary("ggml-base")
        System.loadLibrary("ggml-cpu")
        System.loadLibrary("ggml")
        System.loadLibrary("llama")
        System.loadLibrary("mtmd")
        System.loadLibrary("forge_core")
        System.loadLibrary("forge-android")
    }

    // ============================================================
    // LIFECYCLE
    // ============================================================

    /** Initialize the Forge backend. Call once at app startup. */
    external fun init()

    /** Cleanup the Forge backend. Call when completely done. */
    external fun cleanup()

    // ============================================================
    // ENGINE
    // ============================================================

    /** Create a text-only engine. Returns handle (0 on failure). */
    external fun engineCreate(
        modelPath: String,
        nCtx: Int,
        nBatch: Int,
        nThreads: Int,
        maxTokens: Int,
        temperature: Float,
        topK: Int,
        topP: Float,
        flashAttn: Boolean,
        nGpuLayers: Int
    ): Long

    /** Create a vision-capable engine. Returns handle (0 on failure). */
    external fun engineCreateVision(
        modelPath: String,
        clipPath: String,
        nCtx: Int,
        nBatch: Int,
        nThreads: Int,
        maxTokens: Int,
        temperature: Float,
        topK: Int,
        topP: Float,
        flashAttn: Boolean,
        nGpuLayers: Int
    ): Long

    /** Destroy an engine and free resources. */
    external fun engineDestroy(handle: Long)

    /** Cancel an active generation. Returns ForgeResult code. */
    external fun cancel(handle: Long): Int

    // ============================================================
    // GENERATION
    // ============================================================

    /** Generate text from prompt. Returns ForgeResult code. */
    external fun generate(
        handle: Long,
        prompt: String,
        callback: TokenCallback
    ): Int

    /** Generate for a conversation turn. Returns ForgeResult code. */
    external fun generateTurn(
        handle: Long,
        prompt: String,
        addBos: Boolean,
        callback: TokenCallback
    ): Int

    /** Generate from image and prompt. Returns ForgeResult code. */
    external fun generateVision(
        handle: Long,
        imageData: ByteArray,
        prompt: String,
        callback: TokenCallback
    ): Int

    // ============================================================
    // STATE
    // ============================================================

    /** Check if this is the first turn (empty context). */
    external fun isFirstTurn(handle: Long): Boolean

    /** Get current context position (tokens processed). */
    external fun nPast(handle: Long): Int

    /** Reset the conversation context. Returns ForgeResult code. */
    external fun reset(handle: Long): Int

    /** Get current memory usage in MB. */
    external fun memoryCurrentMb(handle: Long): Double

    /** Get peak memory usage in MB. */
    external fun memoryPeakMb(handle: Long): Double

    /** Get the media marker string (e.g., "<image>"). */
    external fun getMediaMarker(): String
}

/**
 * Callback interface for streaming token generation.
 */
interface TokenCallback {
    fun onToken(token: String)
}
