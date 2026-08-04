# Lesson 08: Forge SDK Deep Dive

A detailed exploration of the Forge SDK architecture.

---

## SDK Structure

```
ForgeSwift/
├── Package.swift                    # Swift Package Manager config
├── ForgeRustCore.xcframework/       # Pre-built Rust core
│   ├── ios-arm64/
│   ├── ios-arm64_x86_64-simulator/
│   └── macos-arm64/
└── Sources/
    └── ForgeSwift/
        └── ForgeSwift.swift         # Swift API wrapper

forge-rust/                          # Rust source code
├── forge-core/
│   ├── src/
│   │   ├── lib.rs                   # Crate root
│   │   ├── core/                    # Core components
│   │   │   ├── model.rs             # LlamaModel
│   │   │   ├── context.rs           # LlamaContext
│   │   │   ├── batch.rs             # BatchArena
│   │   │   ├── sampler.rs           # Sampler chain
│   │   │   └── memory.rs            # MemoryTracker
│   │   ├── engines/
│   │   │   └── engine.rs            # ForgeEngine
│   │   ├── encoders/
│   │   │   └── mtmd_encoder.rs      # Multimodal (vision/audio)
│   │   ├── decoders/
│   │   │   └── audio/               # ISTFT vocoder
│   │   ├── ffi/
│   │   │   └── ffi.rs               # C FFI for Swift
│   │   └── traits/
│   │       ├── encoder.rs           # MediaEncoder trait
│   │       └── decoder.rs           # MediaDecoder trait
│   └── Cargo.toml
└── llama-cpp-sys/                   # Rust bindings to llama.cpp
```

---

## ForgeSwift.swift

The main entry point for Swift applications.

### ForgeConfiguration

```swift
public struct ForgeConfiguration {
    public var contextSize: UInt32 = 2048
    public var batchSize: UInt32 = 128
    public var threads: Int32 = 2
    public var maxTokens: UInt32 = 128
    public var temperature: Float = 0.3
    public var topK: Int32 = 40
    public var topP: Float = 0.95
    public var memoryBudgetMB: UInt32 = 0
    public var flashAttention: Bool = true
    public var gpuLayers: Int32 = -1

    public static func auto() -> ForgeConfiguration
    public static func vision() -> ForgeConfiguration
    public static func mobile() -> ForgeConfiguration
    public static func desktop() -> ForgeConfiguration
}
```

### ForgeEngine

```swift
public class ForgeEngine {
    // Initialization
    public init(modelPath: String, config: ForgeConfiguration) throws
    public init(modelPath: String, clipPath: String, config: ForgeConfiguration) throws

    // Text generation
    public func generate(prompt: String, callback: (String) -> Void) throws -> String
    public func generateTurn(prompt: String, addBOS: Bool, callback: (String) -> Void) throws -> String
    public func generateTurnStream(prompt: String, addBOS: Bool) -> AsyncStream<String>

    // Vision
    public func generateVisionStream(imageData: Data, prompt: String) -> AsyncStream<String>

    // Audio
    public var supportsAudio: Bool
    public var audioSampleRate: UInt32?
    public func generateAudioStream(samples: [Float], prompt: String) -> AsyncStream<String>

    // State
    public var isFirstTurn: Bool
    public var tokensProcessed: Int32
    public var tokensToKeep: Int32
    public var currentMemoryMB: Double
    public var peakMemoryMB: Double

    // Control
    public func reset() throws
}
```

### ForgeAudioDecoder

```swift
public class ForgeAudioDecoder {
    public init() throws
    public init(config: ForgeAudioConfiguration) throws

    public var sampleRate: UInt32
    public var embeddingSize: Int

    public func process(embeddings: [Float]) throws -> [Float]
    public func flush() -> [Float]
    public func reset()
}
```

---

## Rust Core Architecture

### ForgeEngine (Rust)

```rust
pub struct ForgeEngine {
    model: LlamaModel,
    context: LlamaContext,
    sampler: Sampler,
    memory: MemoryTracker,
    config: ForgeConfig,
    multimodal: Option<MultimodalContext>,
    n_past: i32,
    n_keep: i32,
}

impl ForgeEngine {
    pub fn new(model_path: &str, params: &ForgeParams) -> Result<Self>;
    pub fn new_with_vision(model_path: &str, clip_path: &str, params: &ForgeParams) -> Result<Self>;

    pub fn generate<F>(&mut self, prompt: &str, callback: F) -> Result<String>;
    pub fn generate_turn<F>(&mut self, prompt: &str, add_bos: bool, callback: F) -> Result<String>;
    pub fn generate_vision<F>(&mut self, image_data: &[u8], prompt: &str, callback: F) -> Result<String>;
    pub fn generate_audio<F>(&mut self, samples: &[f32], prompt: &str, callback: F) -> Result<String>;

    pub fn reset(&mut self) -> Result<()>;
    pub fn supports_audio(&self) -> bool;
    pub fn audio_sample_rate(&self) -> u32;
}
```

### FFI Layer

The Rust core exposes C-compatible functions for Swift:

```c
// Engine lifecycle
ForgeHandle forge_engine_create(const char* model_path, ForgeParams* params);
ForgeHandle forge_engine_create_vision(const char* model_path, const char* clip_path, ForgeParams* params);
void forge_engine_destroy(ForgeHandle engine);

// Text generation
ForgeResult forge_generate(ForgeHandle engine, const char* prompt, TokenCallback cb, void* user_data);
ForgeResult forge_generate_turn(ForgeHandle engine, const char* prompt, bool add_bos, TokenCallback cb, void* user_data);

// Vision
ForgeResult forge_generate_vision(ForgeHandle engine, const uint8_t* data, usize len, const char* prompt, TokenCallback cb, void* user_data);

// Audio
bool forge_engine_supports_audio(ForgeHandle engine);
uint32_t forge_engine_audio_sample_rate(ForgeHandle engine);
ForgeResult forge_generate_audio(ForgeHandle engine, const float* samples, usize len, const char* prompt, TokenCallback cb, void* user_data);

// Audio decoder (vocoder)
ForgeAudioDecoderHandle forge_audio_decoder_create(void);
void forge_audio_decoder_destroy(ForgeAudioDecoderHandle decoder);
ForgeResult forge_audio_decoder_process(ForgeAudioDecoderHandle decoder, const float* embeddings, usize len, float* output, usize capacity, usize* written);
ForgeResult forge_audio_decoder_flush(ForgeAudioDecoderHandle decoder, float* output, usize capacity, usize* written);
```

---

## Multimodal Architecture

The SDK uses a trait-based architecture for extensibility:

### MediaEncoder Trait

```rust
pub trait MediaEncoder: Send {
    fn media_marker(&self) -> MediaMarker;
    fn capabilities(&self) -> EncoderCapabilities;
}

pub struct EncoderCapabilities {
    pub vision: bool,
    pub audio: bool,
    pub audio_output: bool,
}
```

### MediaDecoder Trait

```rust
pub trait MediaDecoder: Send + Sync {
    type Input;
    type Output;

    fn decode(&mut self, input: &Self::Input) -> Result<Self::Output>;
    fn reset(&mut self);
}
```

### MultimodalContext

Wraps llama.cpp's `mtmd` layer for vision and audio:

```rust
pub struct MultimodalContext {
    ptr: NonNull<mtmd_context>,
}

impl MultimodalContext {
    pub fn supports_vision(&self) -> bool;
    pub fn supports_audio(&self) -> bool;

    pub fn process_image_rgba(&self, ...) -> Result<InputChunks>;
    pub fn process_audio(&self, samples: &[f32], prompt: &str) -> Result<InputChunks>;

    pub fn eval_chunks(&mut self, ...) -> Result<i32>;
}

impl MediaEncoder for MultimodalContext { ... }
```

---

## Usage Examples

### Basic Chat

```swift
import ForgeSwift

initializeForge()

let engine = try ForgeEngine(
    modelPath: "/path/to/model.gguf",
    config: .auto()
)

// First turn (with BOS)
let prompt1 = """
<|im_start|>system
You are helpful.
<|im_end|><|im_start|>user
Hello!
<|im_end|><|im_start|>assistant
"""

for await token in engine.generateTurnStream(prompt: prompt1, addBOS: true) {
    print(token, terminator: "")
}

// Second turn (no BOS)
let prompt2 = """
<|im_start|>user
Tell me more.
<|im_end|><|im_start|>assistant
"""

for await token in engine.generateTurnStream(prompt: prompt2, addBOS: false) {
    print(token, terminator: "")
}
```

### Vision

```swift
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-VL.gguf",
    clipPath: "/path/to/mmproj.gguf",
    config: .vision()
)

let imageData = UIImage(named: "photo")!.jpegData(compressionQuality: 0.8)!

let prompt = """
<|im_start|>user
<|image_start|><|image_end|>What is this?
<|im_end|><|im_start|>assistant
"""

for await token in engine.generateVisionStream(imageData: imageData, prompt: prompt) {
    print(token, terminator: "")
}
```

### Audio to Text

```swift
let engine = try ForgeEngine(
    modelPath: "/path/to/audio-model.gguf",
    clipPath: "/path/to/audio-encoder.gguf",
    config: .auto()
)

guard engine.supportsAudio else { return }

let samples: [Float] = captureAudio() // PCM f32

let prompt = "<|audio|>Transcribe.<|im_end|><|im_start|>assistant\n"

for await token in engine.generateAudioStream(samples: samples, prompt: prompt) {
    print(token, terminator: "")
}
```

---

## Key Takeaways

1. **ForgeSwift** is a thin Swift wrapper over the Rust core
2. **ForgeRustCore** is pre-built as an XCFramework
3. **FFI** uses C-compatible functions for Swift interop
4. **RAII** in Rust ensures automatic memory cleanup
5. **Traits** enable extensible multimodal support
6. **AsyncStream** provides non-blocking UI updates

---

## Next Lesson

👉 **[Lesson 09: Multimodal & Vision →](./09-multimodal-vision.md)**
