# Configuration

The `ForgeConfiguration` class allows you to tune the performance and behavior of the inference engine on both iOS and Android.

## Creating a Configuration

### Swift (iOS/macOS)

```swift
// Auto-detect device capabilities (recommended)
var config = ForgeConfiguration.auto()

// Vision-optimized (4096 context, 512 batch)
var visionConfig = ForgeConfiguration.vision()

// Conservative for mobile
var mobileConfig = ForgeConfiguration.mobile()

// High-performance for macOS
var desktopConfig = ForgeConfiguration.desktop()
```

### Kotlin (Android)

```kotlin
// Auto-detect device capabilities (recommended)
val config = ForgeConfiguration.auto(context)

// Vision-optimized
val visionConfig = ForgeConfiguration.vision(context)

// Conservative for low-memory devices
val mobileConfig = ForgeConfiguration.mobile()
```

---

## Performance Settings

These control how the model runs on your device.

| Property | Swift Type | Kotlin Type | Default | Description |
|----------|------------|-------------|---------|-------------|
| `contextSize` | `UInt32` | `Int` | `2048` | Maximum conversation length in tokens |
| `batchSize` | `UInt32` | `Int` | `128` | Tokens processed per batch |
| `threads` | `Int32` | `Int` | `2` | CPU threads for inference |
| `flashAttention` | `Bool` | `Boolean` | `true` | Enable Flash Attention |
| `gpuLayers` | `Int32` | `Int` | `-1` | GPU layers (-1 = all, 0 = CPU) |
| `maxTokens` | `UInt32` | `Int` | `128` | Max tokens per response |

### Context Size Guidelines

| Use Case | Recommended Context | Memory Impact |
|----------|---------------------|---------------|
| Simple Q&A | 1024 | ~100MB |
| Chat conversation | 2048 | ~200MB |
| Vision (single image) | 4096 | ~400MB |
| Long documents | 8192+ | ~800MB+ |

---

## Sampling Parameters

Control the "creativity" of generated text.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `temperature` | `Float` | `0.3` | Randomness: `0.0` = deterministic, `1.0` = creative |
| `topK` | `Int32/Int` | `40` | Only sample from top K most likely tokens |
| `topP` | `Float` | `0.95` | Nucleus sampling threshold |

### Sampling Examples

#### Swift

```swift
// Factual Q&A (deterministic)
var factConfig = ForgeConfiguration.auto()
factConfig.temperature = 0.1
factConfig.topP = 0.9

// Creative writing
var creativeConfig = ForgeConfiguration.auto()
creativeConfig.temperature = 0.9
creativeConfig.topK = 50
```

#### Kotlin

```kotlin
// Factual Q&A
val factConfig = ForgeConfiguration.auto(context).apply {
    temperature = 0.1f
    topP = 0.9f
}

// Creative writing
val creativeConfig = ForgeConfiguration.auto(context).apply {
    temperature = 0.9f
    topK = 50
}
```

---

## Configuration Presets

### `auto()` - Automatic Detection

Automatically detects device capabilities:

| Device | RAM | Context | Batch | Threads | GPU |
|--------|-----|---------|-------|---------|-----|
| iPhone 15 Pro | 8GB+ | 1024 | 256 | 2 | CPU |
| iPhone 14 Pro | 6GB | 1024 | 256 | 2 | CPU |
| Mac M1 16GB | 16GB+ | 8192 | 512 | 8 | Metal |
| Mac M1 8GB | 8GB | 4096 | 256 | 4 | Metal |
| High-end Android | 8GB+ | 2048 | 256 | 4 | CPU |
| Mid-range Android | 6GB | 1024 | 128 | 2 | CPU |

### `vision()` - Vision Models

Starts from `auto()` with increased settings:
- Context: minimum 4096 (for ~576 image tokens)
- Batch: 512 (faster prefill)

### `mobile()` - Conservative

Safe settings for predictable behavior:
- Context: 2048 (Swift) / 1024 (Kotlin)
- Batch: 128
- Threads: 2

### `desktop()` - High Performance (Swift only)

For powerful Macs:
- Context: 8192
- Batch: 512
- Threads: 8

---

## Device-Specific Tuning

### Swift - Low-Memory iPhone (4-6GB RAM)

```swift
var config = ForgeConfiguration.auto()
config.contextSize = 1024
config.batchSize = 128
config.maxTokens = 128
config.memoryBudgetMB = 800  // Hard limit
```

### Swift - High-End Mac

```swift
var config = ForgeConfiguration.desktop()
config.contextSize = 16384
config.batchSize = 512
config.maxTokens = 2048
```

### Kotlin - Low-Memory Android

```kotlin
val config = ForgeConfiguration.mobile().apply {
    contextSize = 512
    batchSize = 64
    maxTokens = 128
}
```

### Kotlin - High-End Android

```kotlin
val config = ForgeConfiguration.auto(context).apply {
    contextSize = 2048
    batchSize = 256
    maxTokens = 512
}
```

---

## Runtime Monitoring

### Swift

```swift
let engine = try ForgeEngine(modelPath: path, config: config)

print("Current: \(engine.currentMemoryMB) MB")
print("Peak: \(engine.peakMemoryMB) MB")
print("Tokens: \(engine.tokensProcessed)")
print("First turn: \(engine.isFirstTurn)")
```

### Kotlin

```kotlin
val engine = ForgeEngine.create(path, config)

println("Current: ${engine.currentMemoryMB} MB")
println("Peak: ${engine.peakMemoryMB} MB")
println("Tokens: ${engine.tokensProcessed}")
println("First turn: ${engine.isFirstTurn}")
```

---

## Platform Differences

| Feature | iOS/macOS | Android |
|---------|-----------|---------|
| GPU Acceleration | ✅ Metal | ❌ CPU only |
| Auto-detect | RAM, cores, GPU | RAM, cores |
| Model-weight budget preflight | Supported | Supported |
| Max context | 32768+ | 4096 recommended |

> **Note**: Android uses CPU-optimized inference. For best performance, use smaller models (1-3B) and moderate context sizes.

---

© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
