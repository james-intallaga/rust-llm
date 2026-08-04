# Quick Start

Get up and running with Forge SDK in just a few lines of code.

## Prerequisites

Before you begin, make sure you have:

1. **A GGUF Model File**: Download from [Hugging Face](https://huggingface.co/models?search=gguf). Recommended starter models:
   - **Text**: [LFM2-1.2B Q4_0](https://huggingface.co/LiquidAI/LFM2-1.2B-GGUF) (~700MB)
   - **Text**: [Qwen2.5-3B-Instruct Q4_K_M](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF) (~2GB)
   - **Vision**: [LFM2.5-VL-1.6B Q8_0](https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF) (~1.6GB)
   - **Audio**: [LFM2.5-Audio-1.5B](https://huggingface.co/LiquidAI/LFM2.5-Audio-1.5B-GGUF) (~1.5GB)

2. **Model accessible to your app**: Place the `.gguf` file in Documents directory or download at runtime.

---

## Swift (iOS/macOS)

### Basic Text Generation

```swift
import ForgeSwift

// 1. Initialize the Forge backend (call once at app start)
initializeForge()

// 2. Configure with auto-detection (optimizes for device)
let config = ForgeConfiguration.auto()

// 3. Create engine with model path
let modelPath = "/path/to/model.gguf"
let engine = try ForgeEngine(modelPath: modelPath, config: config)

// 4. Generate with streaming callback
let response = try engine.generate(prompt: "Explain gravity in one sentence.") { token in
    print(token, terminator: "")
}
```

### Vision/Multimodal

```swift
// Create vision engine with CLIP model
let engine = try ForgeEngine(
    modelPath: "/path/to/LFM2.5-VL-1.6B-Q8_0.gguf",
    clipPath: "/path/to/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf",
    config: .vision()
)

// Load image data (JPEG/PNG encoded)
let image = UIImage(named: "photo")!
let imageData = image.jpegData(compressionQuality: 0.8)!

// Generate with AsyncStream
let prompt = "<|im_start|>user\n<|image_start|><|image_end|>What is this?<|im_end|><|im_start|>assistant\n"

for await token in engine.generateVisionStream(imageData: imageData, prompt: prompt) {
    print(token, terminator: "")
}
```

### SwiftUI Example

```swift
struct ChatView: View {
    @State private var prompt = ""
    @State private var response = ""
    @State private var engine: ForgeEngine?

    var body: some View {
        VStack {
            Text(response).padding()

            HStack {
                TextField("Ask something...", text: $prompt)
                Button("Send") { sendMessage() }
            }
        }
        .task { await loadModel() }
    }

    private func loadModel() async {
        initializeForge()
        engine = try? ForgeEngine(modelPath: "/path/to/model.gguf", config: .auto())
    }

    private func sendMessage() {
        guard let engine = engine else { return }
        Task {
            for await token in engine.generateTurnStream(prompt: prompt, addBOS: engine.isFirstTurn) {
                await MainActor.run { response += token }
            }
        }
    }
}
```

---

## Kotlin (Android)

### Basic Text Generation

```kotlin
import com.forge.sdk.*

// 1. Create engine with auto-configured settings
val config = ForgeConfiguration.auto(context)
val modelPath = "${context.filesDir}/models/model.gguf"
val engine = ForgeEngine.create(modelPath, config)

// 2. Generate with Kotlin Flow
engine.generateStream("Explain gravity in one sentence.").collect { token ->
    print(token)
}

// 3. Don't forget to close
engine.close()
```

### Vision/Multimodal

```kotlin
// Create vision engine
val config = ForgeConfiguration.vision(context)
val engine = ForgeEngine.createVision(
    modelPath = "${filesDir}/models/LFM2.5-VL-1.6B-Q8_0.gguf",
    clipPath = "${filesDir}/models/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf",
    config = config
)

// Load image as bytes
val imageBytes = assets.open("photo.jpg").readBytes()

// Generate with Flow
val prompt = "<|im_start|>user\n<|image_start|><|image_end|>What is this?<|im_end|><|im_start|>assistant\n"

engine.generateVisionStream(imageBytes, prompt).collect { token ->
    print(token)
}
```

### Jetpack Compose Example

```kotlin
@Composable
fun ChatScreen(viewModel: ChatViewModel = viewModel()) {
    val messages by viewModel.messages.collectAsState()
    var inputText by remember { mutableStateOf("") }

    Column(modifier = Modifier.fillMaxSize()) {
        LazyColumn(modifier = Modifier.weight(1f)) {
            items(messages) { message ->
                Text(message.content)
            }
        }

        Row {
            TextField(
                value = inputText,
                onValueChange = { inputText = it }
            )
            Button(onClick = { viewModel.sendMessage(inputText) }) {
                Text("Send")
            }
        }
    }
}
```

---

## Device Auto-Detection

Both platforms support automatic device optimization:

### Swift

```swift
let config = ForgeConfiguration.auto()  // Detects RAM, cores, GPU
```

### Kotlin

```kotlin
val config = ForgeConfiguration.auto(context)  // Detects RAM, cores
```

| Device | RAM | Context | Batch | Threads |
|--------|-----|---------|-------|---------|
| iPhone 15 Pro | 8GB+ | 1024 | 256 | 2 |
| iPhone 14 Pro | 6GB | 1024 | 256 | 2 |
| Mac M1+ | 16GB+ | 8192 | 512 | 8 |
| High-end Android | 8GB+ | 2048 | 256 | 4 |
| Mid-range Android | 6GB | 1024 | 128 | 2 |

For vision models, use `ForgeConfiguration.vision()` which increases context to 4096 and batch to 512.

---

## Key Methods

### Swift

| Method | Description |
|--------|-------------|
| `generate(prompt:callback:)` | Single-turn generation with streaming |
| `generateTurn(prompt:addBOS:callback:)` | Multi-turn generation |
| `generateTurnStream(prompt:addBOS:)` | Multi-turn with AsyncStream |
| `generateVisionStream(imageData:prompt:)` | Vision generation |
| `generateAudioStream(samples:prompt:)` | Audio generation |
| `reset()` | Clear conversation context |

### Kotlin

| Method | Description |
|--------|-------------|
| `generateStream(prompt)` | Single-turn with Flow |
| `generateTurnStream(prompt, addBOS)` | Multi-turn with Flow |
| `generateVisionStream(imageData, prompt)` | Vision generation |
| `reset()` | Clear conversation context |
| `close()` | Release resources |

---

## Next Steps

- [Configuration](configuration.md) - Tune performance and sampling parameters
- [Multimodal](multimodal.md) - Add vision and audio capabilities
- [API Reference](api-reference.md) - Complete API documentation

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
