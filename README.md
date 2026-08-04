# Forge SDK

**On-device AI inference for iOS, Android, and macOS**

Run LLMs directly on phones, tablets, and computers—no server required. Built on a shared Rust core wrapping [llama.cpp](https://github.com/ggml-org/llama.cpp) with GPU acceleration (Metal on Apple, CPU-optimized on Android).

---

## Quick Start

### Swift (iOS/macOS)

```swift
import ForgeSwift

let engine = try ForgeEngine(modelPath: "/path/to/model.gguf", config: .auto())

for await token in engine.generateTurnStream(prompt: "Explain quantum computing.", addBOS: true) {
    print(token, terminator: "")
}
```

### Kotlin (Android)

```kotlin
import com.forge.sdk.*

val config = ForgeConfiguration.auto(context)
val engine = ForgeEngine.create(modelPath, config)

engine.generateStream("Explain quantum computing.").collect { token ->
    print(token)
}
```

**📚 [Full Documentation](docs/developer/index.md)** — Installation, configuration, multimodal, and more.

---

## Installation

### iOS/macOS (Swift)

In Xcode: **File** → **Add Package Dependencies** → **Add Local...**
Build the native frameworks with `./scripts/build-apple.sh`, then select the
`ForgeSwift` folder.

### Android (Kotlin)

1. Build native libraries: `./forge-rust/scripts/build-android.sh`
2. Open `ForgeAndroid/` in Android Studio
3. Add `:forge-android` as a dependency

See [Installation Guide](docs/developer/installation.md) for detailed steps.

---

## Get a Model

Download a GGUF model from [Hugging Face](https://huggingface.co/models?search=gguf):

| Model | Size | Good For |
|-------|------|----------|
| [LFM2.5-VL 1.6B](https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF) | ~1.6GB | Vision/Multimodal ✅ |
| [LFM2.5-Audio 1.5B](https://huggingface.co/LiquidAI/LFM2.5-Audio-1.5B-GGUF) | ~1.5GB | Voice/Audio |
| [LFM2 1.2B](https://huggingface.co/LiquidAI/LFM2-1.2B-GGUF) | ~700MB | Fast text chat |
| [Qwen2.5 3B](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF) | ~2GB | General purpose |

See [Model Settings](docs/developer/model-settings.md) for more options.

---

## Architecture

Forge SDK uses a **shared Rust core** for both platforms:

```
┌─────────────────────────────────────────────────────────────┐
│                     forge-rust (Rust Core)                  │
│  - llama.cpp bindings                                       │
│  - Vision encoder                                           │
│  - Audio encoder/decoder                                    │
│  - Memory management (RAII)                                 │
├─────────────────────────────────────────────────────────────┤
│               llama.cpp (C/C++)                             │
│        (GPU acceleration, model loading, KV cache)          │
└─────────────────────────────────────────────────────────────┘
                    │                           │
                    ▼                           ▼
    ┌───────────────────────┐       ┌───────────────────────┐
    │   ForgeSwift (iOS)    │       │ ForgeAndroid (Kotlin) │
    │   XCFramework + Swift │       │   JNI bridge + Kotlin │
    │   AsyncStream API     │       │   Flow API            │
    └───────────────────────┘       └───────────────────────┘
```

---

## Features

| Feature | iOS/macOS | Android |
|---------|-----------|---------|
| Text generation (LLaMA, Qwen, Phi, Mistral, Gemma, LFM) | ✅ | ✅ |
| Vision/Multimodal (LFM2.5-VL, LLaVA, Qwen-VL) | ✅ | ✅ |
| Audio input (LFM2.5-Audio, Qwen2-Audio) | ✅ | ✅ |
| Audio output (ISTFT vocoder) | ✅ | ✅ |
| Streaming tokens | AsyncStream | Flow |
| GPU acceleration | Metal (M1+/A12+) | CPU-optimized |
| Device auto-detection | ✅ | ✅ |
| RAII memory management | ✅ | ✅ |

---

## Project Structure

```
amma-forge-sdk/
├── forge-rust/                 # 🦀 Rust core (cross-platform)
│   ├── forge-core/             # Safe Rust wrappers + FFI
│   │   ├── src/core/           # Model, context, sampler
│   │   ├── src/encoders/       # Vision/audio encoders
│   │   ├── src/decoders/       # Audio vocoder (ISTFT)
│   │   └── src/ffi/            # C FFI for Swift/Kotlin
│   ├── llama-cpp-sys/          # llama.cpp bindings
│   └── scripts/
│       ├── build-ios.sh        # iOS XCFramework build
│       └── build-android.sh    # Android .so build
├── ForgeSwift/                 # 📱 Swift wrapper (iOS/macOS)
│   ├── ForgeRustCore.xcframework/ # Generated locally; not committed
│   └── Sources/ForgeSwift/
├── ForgeAndroid/               # 🤖 Kotlin wrapper (Android)
│   ├── forge-android/          # SDK library module
│   │   ├── src/main/cpp/       # JNI bridge
│   │   └── src/main/java/      # Kotlin API
│   └── app/                    # Example Android app
├── example-swift/              # Example iOS/macOS app
├── llama.cpp/                  # Git submodule
├── llama.xcframework/          # Generated locally; not committed
└── docs/                       # 📚 Documentation
```

---

## Example Apps

### iOS/macOS

```bash
git submodule update --init --recursive
./scripts/build-apple.sh
cd example-swift
open AmmaRecognize-Rust.xcodeproj
```

### Android

```bash
# Build native libraries first
./forge-rust/scripts/build-android.sh

# Open in Android Studio
open ForgeAndroid/
```

---

## Supported Models

### Text
LLaMA 3.x, Qwen 2.5/3, Phi 3/4, Mistral, Gemma 2/3, LFM2

### Vision
LFM2.5-VL, Qwen2.5-VL, Qwen3-VL, LLaVA, InternVL, Gemma3

### Audio
LFM2.5-Audio, Qwen2-Audio, Ultravox, Voxtral, GLM-Audio

### Pending (waiting for llama.cpp support)
Fun-Audio-Chat-8B, Qwen3-Omni

---

## Documentation

| Guide | Description |
|-------|-------------|
| [Installation](docs/developer/installation.md) | Setup for iOS, Android, macOS |
| [Quick Start](docs/developer/quick-start.md) | First inference in 5 minutes |
| [Configuration](docs/developer/configuration.md) | Tuning performance |
| [Multimodal](docs/developer/multimodal.md) | Vision & audio support |
| [API Reference](docs/developer/api-reference.md) | Complete API docs |

---

## Requirements

### iOS/macOS
- iOS 15.0+ / macOS 13.0+
- Xcode 15.0+, Swift 5.9+
- 4GB+ RAM for small models, 8GB+ for 7B models

### Android
- Android 7.0+ (API 24)
- arm64-v8a or x86_64 device
- 4GB+ RAM recommended

### Building Native Libraries
- Rust 1.70+ with Android targets
- Android NDK 27.0+
- cargo-ndk

---

## License

MIT License © 2026 AMMA AI Intallaga Tech

Built on [llama.cpp](https://github.com/ggml-org/llama.cpp) by Georgi Gerganov et al.

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for dependency licensing
information. Contributions are welcome; start with
[CONTRIBUTING.md](CONTRIBUTING.md).
