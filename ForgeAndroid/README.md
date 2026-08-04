# ForgeAndroid

Android SDK wrapper for Forge, using the same Rust core as ForgeSwift.

## Architecture

The Android SDK shares the same Rust core as the iOS SDK:
- forge-rust: Rust core with llama.cpp bindings, vision/audio encoders
- ForgeAndroid: Kotlin wrapper with JNI bridge

## Prerequisites

1. Android NDK (version 27.0.12077973 or later)
2. Rust Android targets: `rustup target add aarch64-linux-android x86_64-linux-android`
3. cargo-ndk: `cargo install cargo-ndk`

## Building Native Libraries

Before building the Android project, build the native libraries:

```bash
cd forge-rust
./scripts/build-android.sh
```

This builds llama.cpp and libforge_core.so for arm64-v8a and x86_64.

## SDK Usage

### Basic Chat

```kotlin
val config = ForgeConfiguration.auto(context)
val engine = ForgeEngine.create(modelPath, config)

engine.generateStream("Hello!").collect { token ->
    print(token)
}

engine.close()
```

### Vision Model

```kotlin
val config = ForgeConfiguration.vision(context)
val engine = ForgeEngine.createVision(modelPath, clipPath, config)

val imageBytes = readImageAsBytes()
engine.generateVisionStream(imageBytes, "<image>\nDescribe this.").collect { ... }
```

## Model Recommendations

- LFM2-1.2B-Q4_0.gguf (~700 MB) - Fast text chat
- LFM2.5-VL-1.6B-Q4_0.gguf (~1.1 GB) - Vision chat
