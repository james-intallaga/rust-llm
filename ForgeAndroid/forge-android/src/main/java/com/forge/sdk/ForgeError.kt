package com.forge.sdk

/**
 * Error types for Forge operations.
 *
 * Mirrors ForgeError from ForgeSwift.
 */
sealed class ForgeError(message: String) : Exception(message) {
    class ModelLoadFailed : ForgeError("Failed to load model")
    class ContextCreationFailed : ForgeError("Failed to create context")
    class MultimodalLoadFailed : ForgeError("Failed to load multimodal model")
    class TokenizationFailed : ForgeError("Tokenization failed")
    class DecodeFailed : ForgeError("Decode failed")
    class InvalidParameter(details: String) : ForgeError("Invalid parameter: $details")
    class MemoryExceeded : ForgeError("Memory budget exceeded")
    class Cancelled : ForgeError("Generation cancelled")
    class Unknown(code: Int) : ForgeError("Unknown error (code: $code)")

    companion object {
        /** Convert ForgeResult code to ForgeError */
        fun fromResult(result: Int): ForgeError? = when (result) {
            0 -> null  // Success
            1 -> InvalidParameter("Null pointer")
            2 -> ModelLoadFailed()
            3 -> ContextCreationFailed()
            4 -> MultimodalLoadFailed()
            5 -> TokenizationFailed()
            6 -> DecodeFailed()
            7 -> InvalidParameter("Invalid parameter")
            8 -> MemoryExceeded()
            9 -> Cancelled()
            else -> Unknown(result)
        }
    }
}
