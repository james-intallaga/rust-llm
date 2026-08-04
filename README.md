# Forge SDK

**Build private, offline AI assistants for iPhone, iPad, Android, and Mac from one native inference core.**

Forge SDK turns GGUF language and multimodal models into Swift and Kotlin APIs. It keeps model execution on the user's device, streams output as it is generated, and shares model lifecycle, memory management, sampling, and native bindings through a Rust core.

> [!IMPORTANT]
> Forge SDK is an early-stage open-source project. Expect occasional minor bugs, incomplete documentation, and API changes. Test model quality, memory use, thermals, and failure handling on every device class you intend to support. Please report reproducible problems through the issue tracker.

## Why Forge exists

On-device generative AI often requires separate Apple and Android integrations, model conversion pipelines, and platform-specific lifecycle code. Forge provides a narrower path:

- use quantized GGUF models on both platforms;
- keep inference and user data local, with no required inference server;
- expose native streaming APIs: Swift `AsyncStream` and Kotlin `Flow`;
- use Metal on supported Apple devices and optimized CPU inference on Android;
- build text, vision, and audio experiences on the same Rust foundation.

Forge is an integration framework, not a new inference kernel. The actual model execution is powered by [llama.cpp](https://github.com/ggml-org/llama.cpp), which provides GGUF loading, quantization support, sampling, KV-cache handling, and hardware-optimized inference. Forge adds safe Rust ownership and platform-friendly APIs around it.

### Acknowledgement

Forge would not exist without [llama.cpp](https://github.com/ggml-org/llama.cpp), [ggml](https://github.com/ggml-org/ggml), and their contributors. Their work makes efficient local LLM and VLM inference possible across a wide range of hardware. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for dependency and licensing information.

## Performance and efficiency

Forge is designed to reduce integration overhead while retaining the performance characteristics of its pinned llama.cpp engine:

- **Quantized weights:** GGUF quantization can substantially reduce model storage and memory use, and can improve inference speed, with a possible quality trade-off.
- **Apple acceleration:** Apple builds use llama.cpp's Metal backend. Unified memory lets the CPU and GPU access the same model allocation, although the model must still fit comfortably alongside the OS and app.
- **Android efficiency:** the current Android build targets `arm64-v8a` and `x86_64`, uses optimized native CPU kernels and OpenMP, and supports 16 KB memory pages. Android GPU/NPU execution is not enabled in this release.
- **No network round trip:** local generation removes server latency and works offline. It does not imply that a phone is faster than a datacenter GPU.
- **Thin language bindings:** generation stays in native code; Swift and Kotlin receive streamed output rather than running the model themselves.

### How it compares

| Approach | Where it is strongest | Forge trade-off |
|---|---|---|
| Direct [llama.cpp](https://github.com/ggml-org/llama.cpp) | Maximum control, broad hardware backends, command-line and server use | Forge uses the same underlying engine. It adds Rust lifecycle management and native mobile APIs, not a faster inference kernel. |
| Cloud or Python server stacks | Large models, datacenter accelerators, centralized updates, high throughput | Forge removes the network and server dependency and improves data locality, but mobile hardware usually generates large-model output more slowly. |
| Apple [Core ML](https://developer.apple.com/documentation/coreml) | Deep Apple-platform integration and supported Apple accelerators | Forge offers GGUF compatibility and a shared Android core. A well-converted Core ML model may be faster or more energy-efficient on Apple hardware. |
| Google [LiteRT](https://developers.google.com/edge/litert) | Cross-platform ML with model conversion and CPU/GPU/NPU acceleration | Forge is more specialized for llama.cpp-supported generative GGUF models. LiteRT may win when its model format and hardware delegates are a good fit. |
| [ONNX Runtime Mobile](https://onnxruntime.ai/docs/get-started/with-mobile.html) | Portable ONNX models and configurable mobile operator sets | Forge avoids ONNX conversion for compatible GGUF models. ONNX Runtime supports a wider range of general ML workloads. |
| PyTorch [ExecuTorch](https://docs.pytorch.org/executorch/stable/intro-overview.html) | PyTorch export workflows and hardware-specific delegates | Forge provides a direct GGUF path. ExecuTorch may be preferable when the source model and deployment workflow are already based on PyTorch export. |

There is no honest universal “tokens per second” comparison between these runtimes. Results change with the device, model architecture, quantization, context length, prompt length, temperature, thermal state, and accelerator. Compare them on the same physical device with the same model and workload, and record:

1. model load time;
2. time to first token;
3. prompt-processing tokens per second;
4. generation tokens per second;
5. peak memory;
6. energy use and sustained speed after several minutes.

Forge does not currently publish a cross-framework benchmark suite. Performance claims in issues or pull requests should include the device, OS version, model file, quantization, context size, build type, and prompt.

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
git clone --recurse-submodules <repository-url> amma-forge-simple
cd amma-forge-simple
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
project(":forge-android").projectDir = file("../amma-forge-simple/ForgeAndroid/forge-android")
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

## Current capabilities and limits

| Capability | Apple | Android | Notes |
|---|---:|---:|---|
| Text generation | Yes | Yes | Streaming and multi-turn context |
| Vision input | Yes | Yes | Requires a compatible main model and matching `mmproj` |
| Audio input/output APIs | Experimental | Experimental | Model support varies; validate the full path before product use |
| Automatic device configuration | Yes | Yes | A starting point, not a substitute for profiling |
| GPU acceleration | Metal | No | Android currently uses native CPU inference |
| Cancellation and reset | Yes | Yes | Keep one owner for each live engine |

Known limitations:

- API stability is not guaranteed before a stable release.
- Model chat templates and multimodal markers differ between model families.
- Large contexts increase KV-cache memory and can make mobile apps unstable.
- Device simulators are useful for integration testing, not performance measurement.
- The SDK is not intended for safety-critical decisions without independent safeguards.

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

Copyright © 2026 AMMA AI Intallaga Tech.
