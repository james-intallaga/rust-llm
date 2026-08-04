# API Reference

Complete reference for all public APIs in the Forge SDK for both Swift and Kotlin.

---

## ForgeEngine

The main inference engine. Wraps the Rust ForgeEngine and provides safe, platform-native APIs.

### Swift Initialization

```swift
// Text-only engine
public init(modelPath: String, config: ForgeConfiguration = ForgeConfiguration()) throws

// Vision-capable engine
public init(modelPath: String, clipPath: String, config: ForgeConfiguration = ForgeConfiguration()) throws
```

### Kotlin Initialization

```kotlin
// Text-only engine
suspend fun create(modelPath: String, config: ForgeConfiguration = ForgeConfiguration()): ForgeEngine

// Vision-capable engine
suspend fun createVision(modelPath: String, clipPath: String, config: ForgeConfiguration): ForgeEngine
```

| Parameter | Description |
|-----------|-------------|
| `modelPath` | Absolute path to the `.gguf` model file |
| `clipPath` | Path to the CLIP/mmproj model for vision |
| `config` | Engine configuration |

### Properties

| Property | Swift Type | Kotlin Type | Description |
|----------|------------|-------------|-------------|
| `isFirstTurn` | `Bool` | `Boolean` | Whether context is empty |
| `tokensProcessed` | `Int32` | `Int` | Current context position |
| `currentMemoryMB` | `Double` | `Double` | Current memory usage |
| `peakMemoryMB` | `Double` | `Double` | Peak memory usage |
| `supportsAudio` | `Bool` | `Boolean` | Whether audio is supported |

---

## Text Generation

### Swift

```swift
// Single-turn (clears context)
@discardableResult
func generate(prompt: String, callback: @escaping (String) -> Void) throws -> String

// Multi-turn (preserves context)
@discardableResult
func generateTurn(prompt: String, addBOS: Bool, callback: @escaping (String) -> Void) throws -> String

// Multi-turn with AsyncStream
func generateTurnStream(prompt: String, addBOS: Bool) -> AsyncStream<String>
```

### Kotlin

```kotlin
// Single-turn with Flow
fun generateStream(prompt: String): Flow<String>

// Multi-turn with Flow
fun generateTurnStream(prompt: String, addBOS: Boolean): Flow<String>
```

---

## Vision Generation

### Swift

```swift
// Vision with encoded image (JPEG/PNG)
func generateVisionStream(imageData: Data, prompt: String) -> AsyncStream<String>
```

### Kotlin

```kotlin
// Vision with encoded image (JPEG/PNG)
fun generateVisionStream(imageData: ByteArray, prompt: String): Flow<String>
```

---

## Audio Generation (Swift only)

```swift
// Audio with callback
@discardableResult
func generateAudio(samples: [Float], prompt: String, callback: @escaping (String) -> Void) throws -> String

// Audio with AsyncStream
func generateAudioStream(samples: [Float], prompt: String) -> AsyncStream<String>
```

---

## Control Methods

| Method | Swift | Kotlin | Description |
|--------|-------|--------|-------------|
| Reset | `reset()` | `reset()` | Clear conversation context |
| Close | N/A (RAII) | `close()` | Release resources |

---

## ForgeConfiguration

Configuration for the inference engine.

### Swift Presets

```swift
ForgeConfiguration.auto()      // Auto-detect device capabilities
ForgeConfiguration.vision()    // Optimized for vision models
ForgeConfiguration.mobile()    // Conservative for low-memory
ForgeConfiguration.desktop()   // High-performance for macOS
```

### Kotlin Presets

```kotlin
ForgeConfiguration.auto(context)   // Auto-detect device
ForgeConfiguration.vision(context) // Optimized for vision
ForgeConfiguration.mobile()        // Conservative settings
```

### All Properties

| Property | Swift Type | Kotlin Type | Default | Description |
|----------|------------|-------------|---------|-------------|
| `contextSize` | `UInt32` | `Int` | `2048` | Max context window |
| `batchSize` | `UInt32` | `Int` | `128` | Batch size |
| `threads` | `Int32` | `Int` | `2` | CPU threads |
| `maxTokens` | `UInt32` | `Int` | `128` | Max tokens per response |
| `temperature` | `Float` | `Float` | `0.3` | Sampling randomness |
| `topK` | `Int32` | `Int` | `40` | Top-K sampling |
| `topP` | `Float` | `Float` | `0.95` | Nucleus sampling |
| `flashAttention` | `Bool` | `Boolean` | `true` | Flash Attention |
| `gpuLayers` | `Int32` | `Int` | `-1` | GPU layers (-1 = all) |

---

## ForgeAudioDecoder (Swift)

Audio decoder (vocoder) for converting embeddings to audio.

```swift
// Initialization
public init() throws
public init(config: ForgeAudioConfiguration) throws

// Properties
var sampleRate: UInt32
var embeddingSize: Int

// Methods
func process(embeddings: [Float]) throws -> [Float]
func flush() -> [Float]
func reset()
```

---

## ForgeError

Errors that can occur during Forge operations.

### Swift

```swift
public enum ForgeError: Error {
    case modelLoadFailed
    case contextCreationFailed
    case multimodalLoadFailed
    case tokenizationFailed
    case decodeFailed
    case invalidParameter(String)
    case memoryExceeded
    case unknown(Int32)
}
```

### Kotlin

```kotlin
sealed class ForgeError(message: String) : Exception(message) {
    class ModelLoadFailed : ForgeError("Failed to load model")
    class ContextCreationFailed : ForgeError("Failed to create context")
    class MultimodalLoadFailed : ForgeError("Failed to load multimodal model")
    class TokenizationFailed : ForgeError("Tokenization failed")
    class DecodeFailed : ForgeError("Decode failed")
    class InvalidParameter(details: String) : ForgeError("Invalid parameter: $details")
    class MemoryExceeded : ForgeError("Memory budget exceeded")
    class Unknown(code: Int) : ForgeError("Unknown error (code: $code)")
}
```

---

## Global Functions

### Swift

```swift
func initializeForge()         // Initialize backend (call once)
func cleanupForge()            // Cleanup backend
func getMediaMarker() -> String // Get media marker
```

### Kotlin

```kotlin
ForgeEngine.initialize()       // Initialize backend
ForgeEngine.cleanup()          // Cleanup backend
ForgeEngine.getMediaMarker()   // Get media marker
```

---

## Example Usage

### Swift - Basic Chat

```swift
import ForgeSwift

initializeForge()

let engine = try ForgeEngine(modelPath: "/path/to/model.gguf", config: .auto())

for await token in engine.generateTurnStream(
    prompt: "<|im_start|>user\nHello!<|im_end|><|im_start|>assistant\n",
    addBOS: true
) {
    print(token, terminator: "")
}
```

### Kotlin - Basic Chat

```kotlin
import com.forge.sdk.*

val engine = ForgeEngine.create(modelPath, ForgeConfiguration.auto(context))

engine.generateTurnStream(
    prompt = "<|im_start|>user\nHello!<|im_end|><|im_start|>assistant\n",
    addBOS = true
).collect { token ->
    print(token)
}

engine.close()
```

### Swift - Vision

```swift
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-VL.gguf",
    clipPath: "/path/to/mmproj.gguf",
    config: .vision()
)

let imageData = UIImage(named: "photo")!.jpegData(compressionQuality: 0.8)!
let prompt = "<|im_start|>user\n<|image_start|><|image_end|>What is this?<|im_end|><|im_start|>assistant\n"

for await token in engine.generateVisionStream(imageData: imageData, prompt: prompt) {
    print(token, terminator: "")
}
```

### Kotlin - Vision

```kotlin
val engine = ForgeEngine.createVision(
    modelPath = "$filesDir/LFM2.5-VL.gguf",
    clipPath = "$filesDir/mmproj.gguf",
    config = ForgeConfiguration.vision(context)
)

val imageBytes = assets.open("photo.jpg").readBytes()
val prompt = "<|im_start|>user\n<|image_start|><|image_end|>What is this?<|im_end|><|im_start|>assistant\n"

engine.generateVisionStream(imageBytes, prompt).collect { token ->
    print(token)
}

engine.close()
```

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
