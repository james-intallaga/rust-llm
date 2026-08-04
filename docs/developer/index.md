# Forge SDK

**On-device AI inference for iOS, Android, and macOS.**

Forge SDK lets you run large language models (LLMs) directly on mobile devices and computers—no server required. Built on a shared Rust core wrapping [llama.cpp](https://github.com/ggml-org/llama.cpp) with GPU acceleration.

---

## Quick Example

### Swift (iOS/macOS)

```swift
import ForgeSwift

let engine = try ForgeEngine(modelPath: "/path/to/model.gguf", config: .auto())

let response = try engine.generate(prompt: "Explain quantum computing simply.") { token in
    print(token, terminator: "")
}
```

### Kotlin (Android)

```kotlin
import com.forge.sdk.*

val engine = ForgeEngine.create(modelPath, ForgeConfiguration.auto(context))

engine.generateStream("Explain quantum computing simply.").collect { token ->
    print(token)
}

engine.close()
```

---

## Why Forge SDK?

| Feature | Benefit |
|---------|---------|
| **100% On-Device** | Data never leaves the user's device. Complete privacy. |
| **Cross-Platform** | Same Rust core for iOS, Android, and macOS. |
| **GPU Acceleration** | Metal on Apple (M1-M4, A12+), CPU-optimized on Android. |
| **Swift-Native API** | Async/await, AsyncStream, clean configuration. |
| **Kotlin-Native API** | Coroutines, Flow, idiomatic Kotlin. |
| **Multimodal Ready** | Vision models for images, audio models for voice. |
| **Streaming Output** | Real-time token streaming for responsive UI. |

---

## Supported Platforms

| Platform | Minimum Version | GPU Support | SDK |
|----------|-----------------|-------------|-----|
| iOS | 15.0+ | ✅ Metal (A12+) | ForgeSwift |
| macOS | 13.0+ | ✅ Metal (M1+) | ForgeSwift |
| visionOS | 1.0+ | ✅ Metal | ForgeSwift |
| Android | 7.0+ (API 24) | CPU-optimized | ForgeAndroid |

**Development Requirements**:
- **iOS/macOS**: Xcode 15+, Swift 5.9+
- **Android**: Android Studio, Kotlin 1.9+, NDK 27.0+

**Recommended Hardware**: 8GB+ RAM for 7B models, 4GB+ for 1-3B models.

---

## Supported Models

Forge SDK supports any model in **GGUF format** with llama.cpp compatibility:

### Text Models
- **LLaMA** (1, 2, 3, 3.1, 3.2, 3.3)
- **Qwen** (1.5, 2, 2.5, 3)
- **Phi** (2, 3, 3.5, 4)
- **Mistral / Mixtral**
- **Gemma** (1, 2, 3)
- **LFM** (Liquid AI)

### Vision Models
- **LFM2.5-VL** (Liquid AI) ✅ Recommended
- **Qwen2.5-VL** (Alibaba)
- **Qwen3-VL** (Alibaba)
- **LLaVA** (1.5, 1.6)
- **InternVL**
- **Gemma3** (Vision)

### Audio Models
- **LFM2.5-Audio** (Liquid AI) ✅ Recommended
- **Qwen2-Audio** (Alibaba)
- **Ultravox** (Fixie.ai)
- **Voxtral** (Mistral)
- **GLM-Audio** (THUDM)

> **Note**: Audio models require llama.cpp with audio support. See [Multimodal](multimodal.md) for details.

Download models from [Hugging Face](https://huggingface.co/models?search=gguf).

---

## Architecture

Both SDKs share the same Rust core:

```
┌─────────────────────────────────────────────────────────────┐
│                     forge-rust (Rust Core)                  │
│  - llama.cpp bindings via llama-cpp-sys                     │
│  - Vision encoder (mtmd)                                    │
│  - Audio encoder/decoder (ISTFT vocoder)                    │
│  - Memory management (RAII)                                 │
├─────────────────────────────────────────────────────────────┤
│               llama.cpp (C/C++)                             │
│        (GPU acceleration, model loading, KV cache)          │
└─────────────────────────────────────────────────────────────┘
                    │                           │
                    ▼                           ▼
    ┌───────────────────────┐       ┌───────────────────────┐
    │   ForgeSwift (iOS)    │       │ ForgeAndroid (Kotlin) │
    │   C FFI → Swift       │       │   C FFI → JNI → Kotlin│
    │   AsyncStream API     │       │   Flow API            │
    └───────────────────────┘       └───────────────────────┘
```

---

## Get Started

1. **[Installation](installation.md)** - Add Forge SDK to your project (iOS or Android)
2. **[Quick Start](quick-start.md)** - Run your first inference in 5 minutes
3. **[Configuration](configuration.md)** - Tune for your device and use case

---

## What Can You Build?

- 💬 **Chat apps** with local AI assistants
- 📝 **Writing tools** for drafting and editing
- 📸 **Vision apps** that describe images
- 🎤 **Voice apps** with speech-to-speech
- 🔒 **Privacy-first apps** where data stays on-device

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
