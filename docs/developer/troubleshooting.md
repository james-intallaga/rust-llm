# Troubleshooting

Common issues and their solutions when working with Forge SDK.

---

## Build & Installation Issues

### "No such module 'Forge'"

**Cause**: The Swift package isn't properly linked.

**Solutions**:
1. Clean build folder: **Product** → **Clean Build Folder** (⌘⇧K)
2. Reset package caches: **File** → **Packages** → **Reset Package Caches**
3. Verify the package is added: **Project** → **Package Dependencies**
4. Ensure your target includes the Forge library

### "llama.xcframework not found"

**Cause**: The binary framework path in `Package.swift` is incorrect.

**Solution**: Check `Forge/Package.swift`:

```swift
// For CPU-only (default)
.binaryTarget(name: "llama", path: "../llama_cpu.xcframework")

// For GPU/Metal
.binaryTarget(name: "llama", path: "./llama.cpp/build-apple/llama.xcframework")
```

### "Undefined symbols for architecture arm64"

**Cause**: Mixing CPU and GPU builds, or corrupted framework.

**Solutions**:
1. Delete `DerivedData` folder
2. Rebuild the xcframework from scratch
3. Ensure all targets use the same framework version

---

## Model Loading Issues

### "Failed to load model"

**Possible causes and solutions**:

| Cause | Solution |
|-------|----------|
| File doesn't exist | Verify path with `FileManager.default.fileExists(atPath:)` |
| Wrong format | Ensure file is `.gguf`, not `.bin`, `.pth`, or `.safetensors` |
| Corrupted download | Re-download the model; check file size matches source |
| Insufficient permissions | On iOS, copy to Documents directory first |

**Debug code**:

```swift
let path = "/path/to/model.gguf"

// Check file exists
guard FileManager.default.fileExists(atPath: path) else {
    print("❌ File not found: \(path)")
    return
}

// Check file size
if let attrs = try? FileManager.default.attributesOfItem(atPath: path),
   let size = attrs[.size] as? Int64 {
    print("📦 Model size: \(size / 1_000_000) MB")
}
```

### "Model not supported" or garbled output

**Cause**: The model architecture isn't compatible.

**Supported architectures**:
- LLaMA (1, 2, 3, 3.1, 3.2)
- Qwen (1.5, 2, 2.5)
- Phi (2, 3, 3.5)
- Mistral / Mixtral
- Gemma
- Most GGUF models from llama.cpp ecosystem

**Not supported**:
- PyTorch `.pth` files
- Safetensors (need conversion)
- GGML v1/v2 (old format)
- AWQ/GPTQ quantized models

### Loading is extremely slow

**Solutions**:
1. Enable mmap (default): `config.useMMap = true`
2. Use a smaller quantization (Q4_K_S instead of Q8_0)
3. Don't load from network/cloud storage
4. On iOS, copy to local Documents directory first

---

## Runtime Issues

### Out of memory crash

**Cause**: Model + context doesn't fit in device RAM.

**Solutions**:

```swift
// 1. Reduce context size
config.contextSize = 1024  // Instead of 4096

// 2. Enable Flash Attention (lower memory during eval)
config.useFlashAttention = true

// 3. Use smaller model quantization
// Q4_K_S (~4 bits) uses ~50% less RAM than Q8_0 (~8 bits)
```

**RAM requirements (approximate)**:

| Model Size | Q4_K_M | Q8_0 |
|------------|--------|------|
| 1.5B | ~1.5 GB | ~2.5 GB |
| 3B | ~2.5 GB | ~4 GB |
| 7B | ~5 GB | ~8 GB |
| 13B | ~9 GB | ~15 GB |

### Generation produces garbage/random text

**Possible causes**:

1. **Corrupted model**: Re-download
2. **Wrong quantization**: Some IQ quantizations need specific hardware
3. **Temperature too high**: Try `config.temperature = 0.7`
4. **Context overflow**: Model is confused by too much context

### Generation is slow

**Checklist**:

```swift
// 1. Is GPU enabled?
print("GPU: \(ForgeSDK.isGPUAvailable)")
config.useGPU = true  // Should be true

// 2. Reasonable thread count?
config.threadCount = max(4, Int32(ProcessInfo.processInfo.processorCount - 2))

// 3. Context not too large?
config.contextSize = 2048  // Large contexts slow down generation

// 4. Flash Attention enabled?
config.useFlashAttention = true  // Can help on some devices
```

**Expected speeds (M1 MacBook Air, Q4_K_M)**:

| Model | Prompt Processing | Generation |
|-------|-------------------|------------|
| 1.5B | ~500 tokens/sec | ~50 tokens/sec |
| 7B | ~150 tokens/sec | ~20 tokens/sec |

### Generation stops early

**Cause**: Hit a stop condition or token limit.

**Solutions**:

```swift
// Increase max tokens
config.maxTokens = 512  // Default is 256

// Check for EOS token issues
// Some models output EOS tokens unexpectedly
```

---

## Vision/Multimodal Issues

!!! tip "Detailed Multimodal Troubleshooting"
    For in-depth solutions to multimodal issues (including nonsense output on second images, app hangs, and sampler state issues), see the **[Multimodal Issues Guide](../troubleshooting-and-todos/multimodal-issues.md)**.

### "CLIP model not found"

**Cause**: `clipModelPath` not set in config.

**Solution**:

```swift
var config = ForgeConfig.chat
config.clipModelPath = "/path/to/mmproj-model-f16.gguf"  // Required!
```

### Image description is wrong/blank

**Possible causes**:

1. **Mismatched model and CLIP**: Ensure they're from the same model family
2. **Image too small**: Minimum ~224x224 pixels
3. **Wrong image format**: Use JPEG or PNG
4. **Sampler state not reset**: See [Multimodal Issues Guide](../troubleshooting-and-todos/multimodal-issues.md)

### Second image produces garbage/nonsense

**Cause**: The sampler state (repeat penalty, mirostat μ, RNG) wasn't reset between images.

**Solution**: The SDK now calls `resetSampler()` before each new image. If you're using a custom implementation, ensure you call:

```swift
// Clear KV cache + recurrent memory
llama_memory_clear(memory, true)
nPast = 0
outputRepeatTokens = []

// CRITICAL: Reset sampler state
resetSampler()
```

See [Multimodal Issues Guide](../troubleshooting-and-todos/multimodal-issues.md) for full details.

### Vision is very slow

CLIP processing is computationally expensive.

**Solutions**:
1. Ensure GPU is enabled: `config.useGPU = true`
2. Resize images before processing: max 1024px
3. The SDK loads CLIP on-demand and frees it after use (this is normal)

### App hangs after image generation

**Cause**: Main thread blocked by image data persistence.

**Solution**: Don't save raw image data to UserDefaults. Store a placeholder instead and move persistence to a background thread.

See [Multimodal Issues Guide](../troubleshooting-and-todos/multimodal-issues.md) for implementation details.

---

## Thread Safety & Concurrency

### Crash when using from multiple threads

**Rule**: One `ForgeSDK` instance should only be used from one thread at a time.

**Safe pattern**:

```swift
@MainActor
class ChatManager: ObservableObject {
    private let forge: ForgeSDK

    init() {
        forge = ForgeSDK(modelPath: path)
    }

    func send(_ message: String) {
        // All calls from MainActor
        forge.generate(prompt: message)
    }
}
```

### Callbacks not firing on main thread

All Forge SDK callbacks (`onToken`, `onComplete`, etc.) are dispatched to the **main thread** automatically. If you're experiencing issues, check that you're not blocking the main thread elsewhere.

---

## Debugging Tips

### Enable verbose logging

```swift
// Check what's happening internally
forge.onInfo = { key, value in
    print("ℹ️ \(key): \(value)")
}
```

### Print device capabilities

```swift
print("🔧 Device: \(ForgeSDK.deviceInfo)")
print("🔧 GPU Available: \(ForgeSDK.isGPUAvailable)")
print("🔧 RAM: \(ProcessInfo.processInfo.physicalMemory / 1_073_741_824) GB")
```

### Measure performance

```swift
var tokenCount = 0
var startTime: Date?

forge.onToken = { token in
    if startTime == nil { startTime = Date() }
    tokenCount += 1
}

forge.onComplete = { _ in
    if let start = startTime {
        let elapsed = Date().timeIntervalSince(start)
        print("⚡ \(String(format: "%.1f", Double(tokenCount) / elapsed)) tokens/sec")
    }
}
```

---

## Getting Help

If you're still stuck:

1. **Check the examples**: `Examples/ForgeChat` and `Examples/AmmaRecognize-ForgeTest`
2. **Review the demo**: `Forge/DemoProject`
3. **Inspect logs**: Look for error messages in Xcode console
4. **Simplify**: Try with a small model (Qwen 0.5B) to isolate the issue
