// ForgeSwift - Swift wrapper for the Rust-based llama.cpp inference engine
//
// This module provides a safe, Swift-native API for running LLM inference
// using the Rust wrapper around llama.cpp.
//
// The Rust layer provides:
// - RAII memory management (automatic cleanup)
// - Arena allocators for efficient batch memory
// - Proper sampler chain implementation
// - Model-weight memory budget preflight

import Foundation
// Import the C FFI module from the ForgeRustCore.xcframework
// The module name "ForgeRustCore" comes from the module.modulemap inside the framework
import ForgeRustCore

/// Initialize the Forge backend. Call once before using any other functions.
public func initializeForge() {
    forge_init()
}

/// Cleanup the Forge backend. Call when completely done with inference.
public func cleanupForge() {
    forge_cleanup()
}

/// Get the default media marker for image prompts (e.g., "<image>")
public func getMediaMarker() -> String {
    guard let cStr = forge_get_media_marker() else {
        return "<image>"
    }
    return String(cString: cStr)
}

/// Configuration for the Forge inference engine
public struct ForgeConfiguration {
    /// Context size (number of tokens that can be processed)
    public var contextSize: UInt32 = 2048

    /// Batch size for prompt processing
    public var batchSize: UInt32 = 128

    /// Number of threads for inference
    public var threads: Int32 = 2

    /// Maximum tokens to generate per turn
    public var maxTokens: UInt32 = 128

    /// Temperature (0.0 = greedy, higher = more random)
    public var temperature: Float = 0.3

    /// Top-K sampling (0 = disabled)
    public var topK: Int32 = 40

    /// Top-P (nucleus) sampling (1.0 = disabled)
    public var topP: Float = 0.95

    /// Memory budget in MB (0 = unlimited)
    public var memoryBudgetMB: UInt32 = 0

    /// Enable flash attention
    public var flashAttention: Bool = true

    /// Number of GPU layers (-1 = all)
    public var gpuLayers: Int32 = -1

    /// Create default configuration
    public init() {}

    // MARK: - Device Detection

    /// Automatically detect device capabilities and return optimal configuration
    ///
    /// This checks the device's physical memory and processor count to determine
    /// the best settings. Falls back to conservative defaults if detection fails.
    ///
    /// - Returns: Configuration optimized for the current device
    public static func auto() -> ForgeConfiguration {
        #if os(macOS)
        return detectMacDevice()
        #else
        return detectiOSDevice()
        #endif
    }

    /// Configuration for vision models with automatic device detection
    ///
    /// Vision models need larger context because each image becomes ~576 tokens.
    /// This starts with device-appropriate settings and adjusts for vision.
    ///
    /// - Returns: Configuration optimized for vision models on the current device
    public static func vision() -> ForgeConfiguration {
        var config = auto()

        // Vision models need extra context for image embeddings
        // Minimum 4096 for vision, but respect device limits
        let baseContext = config.contextSize
        config.contextSize = max(baseContext, 4096)

        // OPTIMIZATION #2: Increase batch size for GPU prefill performance
        // 512 is the sweet spot for modern iPhone GPUs to process image tokens
        config.batchSize = 512

        // Log the vision adjustment
        print("[ForgeSwift] Vision config: context \(baseContext) → \(config.contextSize), batch \(config.batchSize)")

        return config
    }

    /// Configuration optimized for mobile devices (conservative defaults)
    ///
    /// Use this when you want predictable, safe settings regardless of device.
    public static func mobile() -> ForgeConfiguration {
        var config = ForgeConfiguration()
        config.contextSize = 2048
        config.batchSize = 128
        config.threads = 2
        config.maxTokens = 128
        config.memoryBudgetMB = 800
        print("[ForgeSwift] Using mobile() preset: ctx=2048, batch=128, threads=2")
        return config
    }

    /// Configuration for desktop/laptop (macOS)
    public static func desktop() -> ForgeConfiguration {
        var config = ForgeConfiguration()
        config.contextSize = 8192
        config.batchSize = 512
        config.threads = 8
        config.maxTokens = 1024
        config.memoryBudgetMB = 0  // Unlimited on desktop
        print("[ForgeSwift] Using desktop() preset: ctx=8192, batch=512, threads=8")
        return config
    }

    // MARK: - Private Detection Methods

    #if os(macOS)
    private static func detectMacDevice() -> ForgeConfiguration {
        let totalMemoryGB = Double(ProcessInfo.processInfo.physicalMemory) / (1024 * 1024 * 1024)
        let processorCount = ProcessInfo.processInfo.processorCount

        print("[ForgeSwift] macOS detected: \(String(format: "%.1f", totalMemoryGB)) GB RAM, \(processorCount) cores")

        var config = ForgeConfiguration()

        if totalMemoryGB >= 32 {
            // High-end Mac (32GB+)
            config.contextSize = 16384
            config.batchSize = 512
            config.maxTokens = 2048
            config.threads = Int32(min(processorCount, 12))
            config.memoryBudgetMB = 0
        } else if totalMemoryGB >= 16 {
            // Standard Mac (16GB)
            config.contextSize = 8192
            config.batchSize = 512
            config.maxTokens = 1024
            config.threads = Int32(min(processorCount, 8))
            config.memoryBudgetMB = 0
        } else {
            // Lower-end Mac (8GB)
            config.contextSize = 4096
            config.batchSize = 256
            config.maxTokens = 512
            config.threads = Int32(min(processorCount, 4))
            config.memoryBudgetMB = 2000
        }

        print("[ForgeSwift] Auto config: ctx=\(config.contextSize), batch=\(config.batchSize), threads=\(config.threads), maxTokens=\(config.maxTokens)")
        return config
    }
    #endif

    private static func detectiOSDevice() -> ForgeConfiguration {
        // Safely get device info with fallbacks
        let totalMemoryBytes = ProcessInfo.processInfo.physicalMemory
        let processorCount = ProcessInfo.processInfo.processorCount

        // Convert to GB, default to 3GB if something goes wrong
        let totalMemoryGB: Double
        if totalMemoryBytes > 0 {
            totalMemoryGB = Double(totalMemoryBytes) / (1024 * 1024 * 1024)
        } else {
            print("[ForgeSwift] ⚠️ Could not detect memory, using conservative defaults")
            return mobile()  // Fail-safe to conservative config
        }

        print("[ForgeSwift] iOS detected: \(String(format: "%.1f", totalMemoryGB)) GB RAM, \(processorCount) cores")

        var config = ForgeConfiguration()

        // Use smaller context sizes on mobile to reduce KV cache memory
        if totalMemoryGB >= 8 {
            // iPhone 15 Pro Max, iPad Pro (8GB+)
            config.contextSize = 1024  // Reduced from 4096 to save ~750MB KV cache
            config.batchSize = 256
            config.maxTokens = 512
            config.threads = 2 // Conservative for stability
            config.gpuLayers = 0 // LLM on CPU to free GPU for camera/other tasks
            config.memoryBudgetMB = 2000
        } else if totalMemoryGB >= 6 {
            // iPhone 14 Pro, iPhone 15 Pro (6GB)
            config.contextSize = 1024  // Reduced from 4096 to save ~750MB KV cache
            config.batchSize = 256
            config.maxTokens = 256
            config.threads = 2 // Conservative for stability
            config.gpuLayers = 0 // LLM on CPU to free GPU for camera/other tasks
            config.memoryBudgetMB = 1200
        } else if totalMemoryGB >= 4 {
            // iPhone 14, iPhone 13 (4GB)
            config.contextSize = 1024  // Reduced from 2048 to save ~250MB KV cache
            config.batchSize = 128
            config.maxTokens = 256
            config.threads = 2 // Conservative for stability
            config.gpuLayers = 0 // LLM on CPU to free GPU for camera/other tasks
            config.memoryBudgetMB = 800
        } else {
            // Older devices (3GB or less) - conservative
            config.contextSize = 512   // Even smaller for low-memory devices
            config.batchSize = 64
            config.maxTokens = 128
            config.threads = 2 // Conservative for stability
            config.gpuLayers = 0 // LLM on CPU to free GPU for camera/other tasks
            config.memoryBudgetMB = 500
        }

        print("[ForgeSwift] Auto config: ctx=\(config.contextSize), batch=\(config.batchSize), threads=\(config.threads), maxTokens=\(config.maxTokens), memBudget=\(config.memoryBudgetMB)MB")
        return config
    }

    /// Convert to C struct
    internal var cParams: ForgeParams {
        var params = forge_params_default()
        params.n_ctx = contextSize
        params.n_batch = batchSize
        params.n_threads = threads
        params.max_tokens = maxTokens
        params.temperature = temperature
        params.top_k = topK
        params.top_p = topP
        params.memory_budget_mb = memoryBudgetMB
        params.flash_attn = flashAttention
        params.n_gpu_layers = gpuLayers
        return params
    }
}

/// Error types for Forge operations
public enum ForgeError: Error {
    case modelLoadFailed
    case contextCreationFailed
    case multimodalLoadFailed
    case tokenizationFailed
    case decodeFailed
    case invalidParameter(String)
    case memoryExceeded
    case cancelled
    case unknown(Int32)

    internal init(result: ForgeResult) {
        switch result {
        case ForgeResult_ModelLoadFailed:
            self = .modelLoadFailed
        case ForgeResult_ContextCreationFailed:
            self = .contextCreationFailed
        case ForgeResult_MultimodalLoadFailed:
            self = .multimodalLoadFailed
        case ForgeResult_TokenizationFailed:
            self = .tokenizationFailed
        case ForgeResult_DecodeFailed:
            self = .decodeFailed
        case ForgeResult_InvalidParameter:
            self = .invalidParameter("Invalid parameter")
        case ForgeResult_MemoryExceeded:
            self = .memoryExceeded
        case ForgeResult_Cancelled:
            self = .cancelled
        default:
            self = .unknown(Int32(result.rawValue))
        }
    }
}

/// The main inference engine
///
/// This class wraps the Rust ForgeEngine and provides a safe, Swift-native API.
/// Memory is automatically managed - the Rust layer uses RAII for all resources.
///
/// # Example
///
/// ```swift
/// let engine = try ForgeEngine(modelPath: "model.gguf")
/// let response = try engine.generate(prompt: "Hello, world!") { token in
///     print(token, terminator: "")
/// }
/// ```
public class ForgeEngine {
    private var handle: ForgeHandle

    /// Create an engine for text-only inference
    ///
    /// - Parameters:
    ///   - modelPath: Path to the GGUF model file
    ///   - config: Engine configuration
    public init(modelPath: String, config: ForgeConfiguration = ForgeConfiguration()) throws {
        var params = config.cParams
        guard let h = forge_engine_create(modelPath, &params) else {
            throw ForgeError.modelLoadFailed
        }
        self.handle = h
    }

    /// Create an engine with vision capabilities
    ///
    /// - Parameters:
    ///   - modelPath: Path to the GGUF model file
    ///   - clipPath: Path to the CLIP model file (mmproj-*.gguf)
    ///   - config: Engine configuration
    public init(modelPath: String, clipPath: String, config: ForgeConfiguration = ForgeConfiguration()) throws {
        var params = config.cParams
        guard let h = forge_engine_create_vision(modelPath, clipPath, &params) else {
            throw ForgeError.multimodalLoadFailed
        }
        self.handle = h
    }

    deinit {
        forge_engine_destroy(handle)
    }

    /// Generate text from a prompt (single-turn, clears context)
    ///
    /// - Parameters:
    ///   - prompt: The input prompt
    ///   - callback: Called for each generated token
    /// - Returns: The complete generated text
    @discardableResult
    public func generate(prompt: String, callback: @escaping (String) -> Void) throws -> String {
        var output = ""

        // Create a context for the callback
        class CallbackContext {
            var callback: (String) -> Void
            var output: String = ""
            init(_ callback: @escaping (String) -> Void) { self.callback = callback }
        }

        let context = CallbackContext(callback)
        let contextPtr = Unmanaged.passRetained(context).toOpaque()

        defer {
            Unmanaged<CallbackContext>.fromOpaque(contextPtr).release()
        }

        let tokenCallback: TokenCallback = { token, userData in
            guard let token = token, let userData = userData else { return }
            let ctx = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            let str = String(cString: token)
            ctx.output += str
            ctx.callback(str)
        }

        let result = forge_generate(handle, prompt, tokenCallback, contextPtr)

        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }

        output = context.output
        return output
    }

    /// Generate a response for a conversation turn (multi-turn aware)
    ///
    /// This properly handles BOS tokens and context accumulation for multi-turn conversations.
    ///
    /// - Parameters:
    ///   - prompt: Formatted prompt for this turn
    ///   - addBOS: Whether to add BOS token (true for first turn only)
    ///   - callback: Called for each generated token
    /// - Returns: The generated response
    @discardableResult
    public func generateTurn(prompt: String, addBOS: Bool, callback: @escaping (String) -> Void) throws -> String {
        var output = ""

        class CallbackContext {
            var callback: (String) -> Void
            var output: String = ""
            init(_ callback: @escaping (String) -> Void) { self.callback = callback }
        }

        let context = CallbackContext(callback)
        let contextPtr = Unmanaged.passRetained(context).toOpaque()

        defer {
            Unmanaged<CallbackContext>.fromOpaque(contextPtr).release()
        }

        let tokenCallback: TokenCallback = { token, userData in
            guard let token = token, let userData = userData else { return }
            let ctx = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            let str = String(cString: token)
            ctx.output += str
            ctx.callback(str)
        }

        let result = forge_generate_turn(handle, prompt, addBOS, tokenCallback, contextPtr)

        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }

        output = context.output
        return output
    }

    /// Generate text from an image and prompt (vision models only)
    ///
    /// - Parameters:
    ///   - imageData: Raw RGBA pixel data
    ///   - width: Image width in pixels
    ///   - height: Image height in pixels
    ///   - prompt: Text prompt (should contain the image marker)
    ///   - callback: Called for each generated token
    /// - Returns: The generated description
    @discardableResult
    public func generateVision(
        imageData: Data,
        width: UInt32,
        height: UInt32,
        prompt: String,
        callback: @escaping (String) -> Void
    ) throws -> String {
        var output = ""

        class CallbackContext {
            var callback: (String) -> Void
            var output: String = ""
            init(_ callback: @escaping (String) -> Void) { self.callback = callback }
        }

        let context = CallbackContext(callback)
        let contextPtr = Unmanaged.passRetained(context).toOpaque()

        defer {
            Unmanaged<CallbackContext>.fromOpaque(contextPtr).release()
        }

        let tokenCallback: TokenCallback = { token, userData in
            guard let token = token, let userData = userData else { return }
            let ctx = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            let str = String(cString: token)
            ctx.output += str
            ctx.callback(str)
        }

        let result = imageData.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> ForgeResult in
            guard let baseAddress = buffer.baseAddress else {
                return ForgeResult_NullPointer
            }
            return forge_generate_vision_rgba(
                handle,
                width,
                height,
                baseAddress.assumingMemoryBound(to: UInt8.self),
                UInt(imageData.count),
                prompt,
                tokenCallback,
                contextPtr
            )
        }

        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }

        output = context.output
        return output
    }

    /// Generate text from an image and prompt using AsyncStream
    ///
    /// This method is non-blocking and yields tokens as they are generated.
    /// It handles JPEG/PNG decoding in the background.
    ///
    /// - Parameters:
    ///   - imageData: Encoded image data (JPEG, PNG)
    ///   - prompt: Text prompt (should contain the image marker)
    /// - Returns: An AsyncStream of generated tokens
    public func generateVisionStream(
        imageData: Data,
        prompt: String
    ) -> AsyncStream<String> {
        AsyncStream { continuation in
            let task = Task.detached {
                class StreamContext {
                    var continuation: AsyncStream<String>.Continuation
                    init(_ continuation: AsyncStream<String>.Continuation) {
                        self.continuation = continuation
                    }
                }

                let streamContext = StreamContext(continuation)
                let contextPtr = Unmanaged.passRetained(streamContext).toOpaque()

                defer {
                    Unmanaged<StreamContext>.fromOpaque(contextPtr).release()
                }

                let tokenCallback: TokenCallback = { token, userData in
                    guard let token = token, let userData = userData else { return }
                    let ctx = Unmanaged<StreamContext>.fromOpaque(userData).takeUnretainedValue()
                    let str = String(cString: token)
                    ctx.continuation.yield(str)
                }

                let result = imageData.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> ForgeResult in
                    guard let baseAddress = buffer.baseAddress else {
                        return ForgeResult_NullPointer
                    }
                    return forge_generate_vision(
                        self.handle,
                        baseAddress.assumingMemoryBound(to: UInt8.self),
                        UInt(imageData.count),
                        prompt,
                        tokenCallback,
                        contextPtr
                    )
                }

                if result != ForgeResult_Ok {
                    print("[ForgeSwift] Vision stream error: \(result)")
                }

                continuation.finish()
            }

            continuation.onTermination = { _ in
                forge_cancel(self.handle)
                task.cancel()
            }
        }
    }

    /// Generate text from a prompt using AsyncStream
    ///
    /// - Parameters:
    ///   - prompt: Formatted prompt for this turn
    ///   - addBOS: Whether to add BOS token (true for first turn only)
    /// - Returns: An AsyncStream of generated tokens
    public func generateTurnStream(
        prompt: String,
        addBOS: Bool
    ) -> AsyncStream<String> {
        AsyncStream { continuation in
            let task = Task.detached {
                class StreamContext {
                    var continuation: AsyncStream<String>.Continuation
                    init(_ continuation: AsyncStream<String>.Continuation) {
                        self.continuation = continuation
                    }
                }

                let streamContext = StreamContext(continuation)
                let contextPtr = Unmanaged.passRetained(streamContext).toOpaque()

                defer {
                    Unmanaged<StreamContext>.fromOpaque(contextPtr).release()
                }

                let tokenCallback: TokenCallback = { token, userData in
                    guard let token = token, let userData = userData else { return }
                    let ctx = Unmanaged<StreamContext>.fromOpaque(userData).takeUnretainedValue()
                    let str = String(cString: token)
                    ctx.continuation.yield(str)
                }

                let result = forge_generate_turn(self.handle, prompt, addBOS, tokenCallback, contextPtr)

                if result != ForgeResult_Ok {
                    print("[ForgeSwift] Turn stream error: \(result)")
                }

                continuation.finish()
            }

            continuation.onTermination = { _ in
                forge_cancel(self.handle)
                task.cancel()
            }
        }
    }

    /// Check if this is the first turn (context is empty)
    public var isFirstTurn: Bool {
        forge_is_first_turn(handle)
    }

    /// Get the current context position (tokens processed so far)
    public var tokensProcessed: Int32 {
        forge_n_past(handle)
    }

    // MARK: - Context Window Sliding

    /// Number of tokens to preserve when context sliding occurs
    ///
    /// When the context fills up (approaches `contextSize`), the engine automatically
    /// shifts the context to make room for new tokens. This property controls how many
    /// tokens at the beginning are preserved (typically the system prompt).
    ///
    /// Set this after initializing the engine but before the first `generateTurn()` call
    /// if you want to preserve your system prompt during long conversations.
    ///
    /// # Example
    ///
    /// ```swift
    /// let engine = try ForgeEngine(modelPath: path)
    /// let systemPrompt = "You are a helpful assistant."
    /// // Tokenize and count system prompt tokens, then set n_keep
    /// engine.tokensToKeep = 50  // Keep first 50 tokens (system prompt)
    /// ```
    public var tokensToKeep: Int32 {
        get { forge_get_n_keep(handle) }
        set { forge_set_n_keep(handle, newValue) }
    }

    /// Check if context sliding is supported
    ///
    /// Most models support context sliding, but some specialized models may not.
    /// If this returns false, the engine will stop generating when the context fills up.
    public var canShiftContext: Bool {
        forge_can_shift_context(handle)
    }

    /// Reset the conversation context
    public func reset() throws {
        let result = forge_reset(handle)
        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }
    }

    /// Cancel the currently running generation, if any.
    public func cancel() {
        forge_cancel(handle)
    }

    /// Get current memory usage in MB
    public var currentMemoryMB: Double {
        forge_memory_current_mb(handle)
    }

    /// Get peak memory usage in MB
    public var peakMemoryMB: Double {
        forge_memory_peak_mb(handle)
    }

}

// MARK: - Audio Decoder (Vocoder)

/// Configuration for the audio decoder (vocoder)
public struct ForgeAudioConfiguration {
    /// FFT size (default: 1280 for LFM2.5-Audio)
    public var nFft: UInt32 = 1280

    /// Hop length in samples (default: 320)
    public var hopLength: UInt32 = 320

    /// Sample rate in Hz (default: 24000)
    public var sampleRate: UInt32 = 24000

    /// Create default configuration for LFM2.5-Audio
    public init() {}

    /// Create configuration for LFM2.5-Audio (convenience)
    public static func lfm25Audio() -> ForgeAudioConfiguration {
        ForgeAudioConfiguration()
    }

    /// Convert to C struct
    internal var cConfig: ForgeAudioConfig {
        ForgeAudioConfig(
            n_fft: nFft,
            hop_length: hopLength,
            sample_rate: sampleRate
        )
    }
}

/// Audio decoder (vocoder) for converting LLM embeddings to audio
///
/// This class wraps the Rust ISTFT vocoder and provides a Swift-native API
/// for real-time audio synthesis from speech-to-speech models.
///
/// # Example
///
/// ```swift
/// let decoder = try ForgeAudioDecoder()
///
/// // Process each frame from the model
/// while let embeddings = getNextEmbeddings() {
///     let samples = try decoder.process(embeddings: embeddings)
///     audioPlayer.enqueue(samples)
/// }
///
/// // Flush remaining audio at end
/// let finalSamples = decoder.flush()
/// audioPlayer.enqueue(finalSamples)
/// ```
public class ForgeAudioDecoder {
    private var handle: ForgeAudioDecoderHandle

    /// The sample rate of the output audio
    public let sampleRate: UInt32

    /// The expected number of embedding values per frame
    public var embeddingSize: Int {
        Int(forge_audio_decoder_embedding_size(handle))
    }

    /// Create an audio decoder with default LFM2.5-Audio parameters
    public init() throws {
        guard let h = forge_audio_decoder_create() else {
            throw ForgeError.unknown(0)
        }
        self.handle = h
        self.sampleRate = forge_audio_decoder_sample_rate(h)
    }

    /// Create an audio decoder with custom configuration
    ///
    /// - Parameter config: Audio decoder configuration
    public init(config: ForgeAudioConfiguration) throws {
        var cConfig = config.cConfig
        guard let h = forge_audio_decoder_create_with_config(&cConfig) else {
            throw ForgeError.unknown(0)
        }
        self.handle = h
        self.sampleRate = config.sampleRate
    }

    deinit {
        forge_audio_decoder_destroy(handle)
    }

    /// Process embeddings and generate audio samples
    ///
    /// This is the main vocoder function. It takes magnitude/phase embeddings
    /// from the LLM and converts them to PCM audio samples.
    ///
    /// - Parameter embeddings: Model output embeddings (magnitude + phase)
    /// - Returns: PCM audio samples (f32, -1.0 to 1.0)
    public func process(embeddings: [Float]) throws -> [Float] {
        // Allocate output buffer (hop_length samples per frame)
        var output = [Float](repeating: 0, count: 1024)  // More than enough for one frame
        var samplesWritten: Int = 0

        let result = embeddings.withUnsafeBufferPointer { embeddingsPtr -> ForgeResult in
            output.withUnsafeMutableBufferPointer { outputPtr -> ForgeResult in
                forge_audio_decoder_process(
                    handle,
                    embeddingsPtr.baseAddress,
                    UInt(embeddingsPtr.count),
                    outputPtr.baseAddress,
                    UInt(outputPtr.count),
                    &samplesWritten
                )
            }
        }

        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }

        // Return only the written samples
        return Array(output.prefix(samplesWritten))
    }

    /// Flush remaining audio samples at end of stream
    ///
    /// Call this after processing all frames to get the final samples
    /// that are still in the overlap buffer.
    ///
    /// - Returns: Remaining PCM audio samples
    public func flush() -> [Float] {
        var output = [Float](repeating: 0, count: 2048)  // Larger buffer for flush
        var samplesWritten: Int = 0

        let result = output.withUnsafeMutableBufferPointer { outputPtr -> ForgeResult in
            forge_audio_decoder_flush(
                handle,
                outputPtr.baseAddress,
                UInt(outputPtr.count),
                &samplesWritten
            )
        }

        if result != ForgeResult_Ok {
            print("[ForgeSwift] Audio decoder flush failed: \(result)")
            return []
        }

        return Array(output.prefix(samplesWritten))
    }

    /// Reset the decoder state for a new audio stream
    public func reset() {
        _ = forge_audio_decoder_reset(handle)
    }
}

// MARK: - Audio Decoder (Vocoder)

// MARK: - Audio Input Extensions for ForgeEngine

extension ForgeEngine {
    /// Check if this engine supports audio input
    public var supportsAudio: Bool {
        forge_engine_supports_audio(handle)
    }

    /// Get the expected audio sample rate in Hz
    ///
    /// Returns nil if audio is not supported.
    public var audioSampleRate: UInt32? {
        let rate = forge_engine_audio_sample_rate(handle)
        return rate > 0 ? rate : nil
    }

    /// Generate text from audio input
    ///
    /// - Parameters:
    ///   - samples: PCM audio samples (f32, normalized -1.0 to 1.0)
    ///   - prompt: Text prompt (should contain the media marker)
    ///   - callback: Called for each generated token
    /// - Returns: The generated response text
    @discardableResult
    public func generateAudio(
        samples: [Float],
        prompt: String,
        callback: @escaping (String) -> Void
    ) throws -> String {
        var output = ""

        class CallbackContext {
            var callback: (String) -> Void
            var output: String = ""
            init(_ callback: @escaping (String) -> Void) { self.callback = callback }
        }

        let context = CallbackContext(callback)
        let contextPtr = Unmanaged.passRetained(context).toOpaque()

        defer {
            Unmanaged<CallbackContext>.fromOpaque(contextPtr).release()
        }

        let tokenCallback: TokenCallback = { token, userData in
            guard let token = token, let userData = userData else { return }
            let ctx = Unmanaged<CallbackContext>.fromOpaque(userData).takeUnretainedValue()
            let str = String(cString: token)
            ctx.output += str
            ctx.callback(str)
        }

        let result = samples.withUnsafeBufferPointer { samplesPtr -> ForgeResult in
            forge_generate_audio(
                handle,
                samplesPtr.baseAddress,
                UInt(samplesPtr.count),
                prompt,
                tokenCallback,
                contextPtr
            )
        }

        if result != ForgeResult_Ok {
            throw ForgeError(result: result)
        }

        output = context.output
        return output
    }

    /// Generate text from audio input using AsyncStream
    ///
    /// This method is non-blocking and yields tokens as they are generated.
    ///
    /// - Parameters:
    ///   - samples: PCM audio samples (f32, normalized -1.0 to 1.0)
    ///   - prompt: Text prompt (should contain the media marker)
    /// - Returns: An AsyncStream of generated tokens
    public func generateAudioStream(
        samples: [Float],
        prompt: String
    ) -> AsyncStream<String> {
        AsyncStream { continuation in
            let task = Task.detached {
                class StreamContext {
                    var continuation: AsyncStream<String>.Continuation
                    init(_ continuation: AsyncStream<String>.Continuation) {
                        self.continuation = continuation
                    }
                }

                let streamContext = StreamContext(continuation)
                let contextPtr = Unmanaged.passRetained(streamContext).toOpaque()

                defer {
                    Unmanaged<StreamContext>.fromOpaque(contextPtr).release()
                }

                let tokenCallback: TokenCallback = { token, userData in
                    guard let token = token, let userData = userData else { return }
                    let ctx = Unmanaged<StreamContext>.fromOpaque(userData).takeUnretainedValue()
                    let str = String(cString: token)
                    ctx.continuation.yield(str)
                }

                let result = samples.withUnsafeBufferPointer { samplesPtr -> ForgeResult in
                    forge_generate_audio(
                        self.handle,
                        samplesPtr.baseAddress,
                        UInt(samplesPtr.count),
                        prompt,
                        tokenCallback,
                        contextPtr
                    )
                }

                if result != ForgeResult_Ok {
                    print("[ForgeSwift] Audio stream error: \(result)")
                }

                continuation.finish()
            }

            continuation.onTermination = { _ in
                forge_cancel(self.handle)
                task.cancel()
            }
        }
    }
}
