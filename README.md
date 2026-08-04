# Forge SDK

**Build fast, private AI assistants for iPhone, iPad, Android, and Mac from one native Rust core.**

Forge SDK is a local-first framework for bringing language, vision, and audio models directly into native apps. It turns llama.cpp-compatible GGUF models into clean Swift and Kotlin APIs, streams responses token by token, and shares the hard inference work across platforms through a memory-safe Rust core.

With Forge, an app can deliver useful AI without an inference server, a permanent internet connection, or separate inference architectures for Apple and Android.

> [!NOTE]
> Forge is actively evolving. Minor bugs and API refinements may still occur, and reproducible issue reports are welcome.

## Why Forge exists

Forge is designed for developers who want excellent on-device AI without rebuilding the same infrastructure for every platform:

- **Private by design:** prompts, images, documents, and generated responses can remain on the device.
- **One cross-platform core:** model loading, sampling, conversation state, cancellation, and memory ownership are implemented once in Rust.
- **Native app experience:** Swift receives `AsyncStream`; Kotlin receives `Flow`; both integrate naturally with modern UI code.
- **Offline and responsive:** once the model is installed, generation does not depend on network availability or server response time.
- **Efficient model distribution:** quantized GGUF models make capable assistants practical on phones, tablets, and personal computers.
- **Built for assistants:** streaming text, multi-turn context, vision input, audio APIs, automatic device configuration, and example apps are included.

The result is a compact foundation for personal assistants, private chat, visual understanding, document help, accessibility tools, and other local AI experiences.

## Built on proven foundations

Forge combines three strong layers:

1. [llama.cpp](https://github.com/ggml-org/llama.cpp) provides high-performance GGUF inference, quantization support, tokenization, sampling, and KV-cache management.
2. `forge-core` adds shared Rust ownership, lifecycle management, multimodal plumbing, cancellation, and a stable FFI boundary.
3. ForgeSwift and ForgeAndroid expose small, idiomatic APIs for native applications.

### Acknowledgement

Forge would not exist without [llama.cpp](https://github.com/ggml-org/llama.cpp), [ggml](https://github.com/ggml-org/ggml), and their contributors. Their work makes efficient local LLM and VLM inference possible across a wide range of hardware. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for dependency and licensing information.

## Designed for efficient local performance

Forge keeps the performance-critical path native and close to llama.cpp while removing much of the platform integration work:

- **Native execution:** model inference remains in optimized C/C++ and Rust code rather than crossing into a scripting runtime for each token.
- **Metal acceleration:** Apple builds use llama.cpp's Metal backend to take advantage of Apple silicon and unified memory.
- **Optimized Android builds:** Android targets `arm64-v8a` and `x86_64`, uses optimized CPU kernels and OpenMP, and supports modern 16 KB memory pages.
- **Quantized GGUF models:** four-bit and other quantizations can reduce downloads and memory pressure while keeping useful model quality.
- **Direct token streaming:** generated output reaches the UI incrementally, making assistants feel responsive before a full answer is complete.
- **No network round trip:** after loading, local inference is independent of internet latency, service availability, rate limits, and per-token API fees.
- **Device-aware defaults:** Forge selects practical context, batch, thread, and GPU settings as a strong starting point for each device.

### The Forge advantage

| If you would otherwise use… | What Forge gives you |
|---|---|
| Direct [llama.cpp](https://github.com/ggml-org/llama.cpp) integration | The same proven inference foundation plus shared Rust lifecycle management, Swift `AsyncStream`, Kotlin `Flow`, automatic configuration, and ready-to-run app examples. |
| A cloud or Python inference service | Fully local operation, private data flow, offline availability, immediate token streaming, and no inference-server operations or per-token API bill. |
| Separate Apple and Android implementations | One model format and one shared inference core, with thin native APIs for each platform. |
| Apple [Core ML](https://developer.apple.com/documentation/coreml) alone | A GGUF-centered workflow that also reaches Android and can follow llama.cpp's rapidly expanding model support. |
| [LiteRT](https://developers.google.com/edge/litert), [ONNX Runtime Mobile](https://onnxruntime.ai/docs/get-started/with-mobile.html), or [ExecuTorch](https://docs.pytorch.org/executorch/stable/intro-overview.html) | A focused path for generative GGUF models without requiring a separate ONNX, TFLite, or ExecuTorch export pipeline. |

Forge deliberately focuses on the shortest route from a compatible GGUF model to a polished native assistant. Broader ML runtimes remain useful for other workloads; Forge's strength is making local generative AI straightforward across Apple and Android.

### Measuring performance

For meaningful results, benchmark the intended model on the intended physical device and record:

1. model load time;
2. time to first token;
3. prompt-processing tokens per second;
4. generation tokens per second;
5. peak memory;
6. energy use and sustained speed after several minutes.

Model architecture, quantization, context, prompt length, temperature, device temperature, and build type all affect results. Include those details when sharing benchmarks so Forge performance can be reproduced and improved.

## Platform support

| Platform | Minimum | Current acceleration | Native API |
|---|---:|---|---|
| iOS / iPadOS | 16.4 | Metal on supported devices | Swift |
| macOS | 13 | Metal on Apple silicon; CPU fallback where available | Swift |
| Android | 7.0 / API 24 | Optimized CPU | Kotlin |

The included Android example has a minimum of API 26 even though the SDK library supports API 24.

## Start here: run the example apps

The examples are the quickest way to verify the complete model-download, loading, and streaming path.

### 1. Clone the repository

Clone with submodules because llama.cpp is pinned as a Git submodule:

```bash
git clone --recurse-submodules https://github.com/james-intallaga/rust-llm.git
cd rust-llm
```

If you already cloned without submodules:

```bash
git submodule update --init --recursive
```

### 2A. Run on iPhone, iPad, or Mac

You need:

- macOS with Xcode 15 or newer;
- the Xcode command-line tools;
- [Rustup](https://rustup.rs/); the repository selects Rust 1.97.0 automatically;
- enough free disk space for build products and model files.

Build both Apple XCFrameworks from the repository root:

```bash
./scripts/build-apple.sh
```

This creates the ignored local artifacts `llama.xcframework` and `ForgeSwift/ForgeRustCore.xcframework`.

Open the example project:

```bash
open example-swift/AmmaRecognize-Rust.xcodeproj
```

In Xcode:

1. select the `AmmaRecognize-Rust` scheme;
2. select a simulator, iPhone, iPad, or **My Mac (Designed for iPad)** destination;
3. choose your development team if running on a physical iPhone or iPad;
4. press **Run**.

On first launch, the example downloads its selected model and validates the file before loading it. Keep the app in the foreground and allow the download to finish. The default policy uses the small `LFM2.5-VL-450M` model on phones and tablets, and selects `LFM2.5-8B-A1B` for text chat on a Mac with sufficient memory.

### 2B. Run on Android

You need:

- Android Studio with Android SDK 34 or newer;
- JDK 17;
- Android NDK `27.0.12077973`;
- CMake 3.22.1;
- [Rustup](https://rustup.rs/) and `cargo-ndk` 4.1.2.

Install the Rust targets and Android build helper once:

```bash
rustup target add aarch64-linux-android x86_64-linux-android
cargo install cargo-ndk --version 4.1.2 --locked
```

Point the build at your NDK. This is the default path on macOS; adjust it for your machine:

```bash
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/27.0.12077973"
```

Build the native libraries from the repository root:

```bash
./forge-rust/scripts/build-android.sh
```

Then open `example-android` in Android Studio, allow Gradle to sync, select an `arm64-v8a` device or emulator, and press **Run**. To verify the build from a terminal instead:

```bash
cd example-android
./gradlew :app:assembleDebug
```

The example downloads `LFM2.5-VL-450M` and its matching vision projector on first launch. Downloads are size-checked before activation.

## Add Forge to your own app

### Swift

First run `./scripts/build-apple.sh`. In Xcode choose **File → Add Package Dependencies… → Add Local…**, select the `ForgeSwift` directory, and add the `ForgeSwift` product to your target.

Initialize once, then create and retain an engine:

```swift
import ForgeSwift

initializeForge()

let engine = try ForgeEngine(
    modelPath: modelURL.path,
    config: .auto()
)

for await token in engine.generateTurnStream(
    prompt: "Explain why the sky is blue in two sentences.",
    addBOS: engine.isFirstTurn
) {
    print(token, terminator: "")
}
```

Do model loading and generation away from latency-sensitive UI work. Keep the `ForgeEngine` alive for the conversation, and call `reset()` when starting a new conversation.

### Kotlin

Include the library module in your Gradle settings:

```kotlin
include(":forge-android")
project(":forge-android").projectDir = file("../rust-llm/ForgeAndroid/forge-android")
```

Add it to the app module:

```kotlin
dependencies {
    implementation(project(":forge-android"))
}
```

Engine creation is a suspending operation. Create, use, and close it from a coroutine, such as a `ViewModel` scope:

```kotlin
import android.content.Context
import androidx.lifecycle.viewModelScope
import com.forge.sdk.ForgeConfiguration
import com.forge.sdk.ForgeEngine
import kotlinx.coroutines.launch

fun loadAndGenerate(context: Context) {
    viewModelScope.launch {
        val modelPath = "${context.filesDir}/models/model.gguf"
        val engine = ForgeEngine.create(
            modelPath = modelPath,
            config = ForgeConfiguration.auto(context)
        )

        try {
            engine.generateTurnStream(
                prompt = "Explain why the sky is blue in two sentences.",
                addBOS = engine.isFirstTurn
            ).collect { token ->
                print(token)
            }
        } finally {
            engine.close()
        }
    }
}
```

For a long-lived chat screen, store the engine in your `ViewModel`, reuse it between turns, and close it from `onCleared()`.

## Choose and install a model

Model weights are not included in this repository. Download them from the original publisher and review the model license before redistribution.

1. Choose a llama.cpp-compatible **GGUF** model.
2. Start with a four-bit quantization such as `Q4_K_M` when memory is limited.
3. Copy or download the `.gguf` file into app-controlled storage.
4. For a vision model, download the matching `mmproj` file from the same repository and revision.
5. Pass absolute filesystem paths to `ForgeEngine`.
6. Test the exact files on every target device class before shipping.

A practical starting policy is:

| Device | Starting point | Reason |
|---|---|---|
| Phone or tablet | 0.5B–1.5B model, Q4 quantization, modest context | Lower memory pressure, faster startup, better sustained thermals |
| Mac with 8 GB memory | Small model first; increase only after measuring peak memory | The OS, app, model, and KV cache share memory |
| Mac with 12 GB+ memory | A quantized 3B–8B model if the workload needs higher quality | More capability, with longer load time and higher memory use |

The included assistant examples currently use:

- [LFM2.5-VL-450M-GGUF](https://huggingface.co/LiquidAI/LFM2.5-VL-450M-GGUF) for phone, tablet, and image requests;
- [LFM2.5-8B-A1B-GGUF](https://huggingface.co/LiquidAI/LFM2.5-8B-A1B-GGUF) for text chat on sufficiently capable Macs.

See [recommended models](docs/troubleshooting-and-todos/recommended-models.md), [model settings](docs/developer/model-settings.md), and the [multimodal guide](docs/developer/multimodal.md). Model compatibility changes with llama.cpp, so a model family listed in the documentation is not a guarantee that every conversion or quantization will load.

## Architecture

```text
Swift / AsyncStream                         Kotlin / Flow
        │                                        │
        └──────────── platform bindings ─────────┘
                             │
                    forge-core (Rust)
          ownership · lifecycle · sampling · FFI
                             │
                         llama.cpp
            GGUF · tokenization · KV cache · inference
                  │                         │
            Apple Metal             Android native CPU
```

The llama.cpp submodule is intentionally pinned so builds do not silently change underneath the Swift, Kotlin, Rust, and JNI layers.

## Capabilities

| Capability | Apple | Android | Notes |
|---|---:|---:|---|
| Text generation | Yes | Yes | Streaming and multi-turn context |
| Vision input | Yes | Yes | Requires a compatible main model and matching `mmproj` |
| Audio input/output APIs | Preview | Preview | Shared APIs for compatible audio models |
| Automatic device configuration | Yes | Yes | Practical defaults based on device resources |
| GPU acceleration | Metal | No | Android currently uses native CPU inference |
| Cancellation and reset | Yes | Yes | Keep one owner for each live engine |

### Project maturity

Forge is already usable for building and experimenting with native assistants, and the example apps exercise the complete download, model-loading, multimodal, and streaming flow. The project is still moving quickly: small bugs may remain, APIs may become cleaner, and support will continue to grow alongside llama.cpp. Contributions and real-device results are especially valuable at this stage.

## Troubleshooting

| Symptom | Check first |
|---|---|
| Model does not load | Confirm the path exists, the file finished downloading, and the GGUF is compatible with the pinned llama.cpp revision. |
| Vision request fails | Confirm the main model and `mmproj` came from the same model repository and revision. |
| App is terminated while loading | Use a smaller quantization or model and reduce the context size. |
| Android `UnsatisfiedLinkError` | Rebuild native libraries and confirm the device ABI is `arm64-v8a` or `x86_64`. |
| Swift package cannot find a binary target | Run `./scripts/build-apple.sh`, then clean and rebuild the Xcode project. |
| Output is slow after sustained use | Check device temperature, lower model size or context, and benchmark a release build on physical hardware. |

More help is available in the [installation guide](docs/developer/installation.md), [configuration guide](docs/developer/configuration.md), [API reference](docs/developer/api-reference.md), and [troubleshooting notes](docs/troubleshooting-and-todos/11-troubleshooting-debugging.md).

## Repository layout

```text
forge-rust/       Shared Rust core and native build scripts
ForgeSwift/       Swift package and Apple API
ForgeAndroid/     Kotlin library and JNI bridge
example-swift/    iOS/iPadOS assistant example
example-android/  Android assistant example
llama.cpp/        Pinned upstream submodule
docs/             Developer guides and technical notes
```

## Contributing and security

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. For vulnerabilities or sensitive reports, follow [SECURITY.md](SECURITY.md) instead of filing a public issue.

## License

Forge SDK is available under the [MIT License](LICENSE). Model weights have their own licenses and are not covered by the Forge license.

Copyright © 2026 James M. Z and AMMA AI Intallaga Tech.
