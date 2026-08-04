# Building from Source

This guide covers advanced build configurations for contributors and developers who need custom builds.

---

## Prerequisites

- macOS 13.3+
- Xcode 15.0+
- CMake 3.28+
- Swift 5.9+

Install CMake if needed:

```bash
brew install cmake
```

---

## Quick Start (CPU-Only)

The repository includes a pre-built `llama_cpu.xcframework`. To use it:

1. Open one of the example projects in `Examples/`
2. Build and run

This works on all devices but is slower than GPU-accelerated inference.

---

## Building with GPU (Metal) Support

For full Metal acceleration on Apple Silicon:

```bash
cd Forge/llama.cpp
./build-xcframework.sh
```

This creates `build-apple/llama.xcframework` with:

| Platform | Architecture | Metal |
|----------|--------------|-------|
| iOS Device | arm64 | ✅ |
| iOS Simulator | arm64, x86_64 | ❌ |
| macOS | arm64, x86_64 | ✅ (arm64 only) |
| visionOS | arm64 | ✅ |
| tvOS | arm64 | ✅ |

Then update `Forge/Package.swift`:

```swift
.binaryTarget(
    name: "llama",
    path: "./llama.cpp/build-apple/llama.xcframework"
)
```

---

## Building with Multimodal (Vision/Audio) Support

The new llama.cpp uses the `mtmd` (multimodal) library for vision and audio.

### 1. Enable mtmd in the Build

Edit `Forge/llama.cpp/build-xcframework.sh`:

```bash
# Add to COMMON_CMAKE_ARGS:
-DLLAMA_BUILD_MTMD=ON
```

Then rebuild:

```bash
./build-xcframework.sh
```

### 2. Update the Module Map

After building, update the module map in each platform's framework. For example, for iOS:

```bash
# Edit: build-apple/llama.xcframework/ios-arm64/llama.framework/Modules/module.modulemap
```

Add the mtmd headers:

```modulemap
framework module llama {
    header "llama.h"
    header "ggml.h"
    header "ggml-alloc.h"
    header "ggml-backend.h"
    header "ggml-metal.h"
    header "ggml-cpu.h"
    header "ggml-blas.h"
    header "gguf.h"
    header "mtmd.h"       // Add this
    header "clip.h"       // Add this

    link "c++"
    link framework "Accelerate"
    link framework "Metal"
    link framework "Foundation"

    export *
}
```

### 3. Copy the Headers

```bash
# From the llama.cpp directory
cp tools/mtmd/mtmd.h build-apple/llama.xcframework/ios-arm64/llama.framework/Headers/
cp tools/mtmd/clip.h build-apple/llama.xcframework/ios-arm64/llama.framework/Headers/

# Repeat for other platforms:
# - ios-arm64_x86_64-simulator
# - macos-arm64_x86_64
# - xros-arm64 (visionOS)
# - tvos-arm64
```

---

## Testing the Build

### Run the Demo Project

```bash
cd Forge/DemoProject
swift build
swift run DemoProject
```

### Run an Example App

1. Open `Examples/ForgeChat/` or `Examples/AmmaRecognize-ForgeTest/`
2. Run `xcodegen generate` if needed (or use the Package.swift)
3. Build and run (⌘R)
4. Load a model file when prompted

---

## Troubleshooting

### "No such module 'llama'"

The XCFramework path is incorrect.

**Fix**:
1. Verify the path in `Package.swift` matches your framework location
2. Run `swift package clean` and rebuild
3. Check the framework was built successfully

### Metal not working

**Symptoms**: GPU shows as unavailable, inference is slow

**Fix**:
1. Ensure you're on Apple Silicon (M1+) or a device with A12+ chip
2. Verify the framework was built with Metal enabled:
   ```bash
   # Check for Metal symbols
   nm llama.xcframework/ios-arm64/llama.framework/llama | grep metal
   ```
3. On Intel Macs, Metal is automatically disabled

### "symbol not found: _mtmd_init_from_file"

The mtmd library wasn't included in the build.

**Fix**: Rebuild with the mtmd flag:

```bash
# In build-xcframework.sh, ensure this is set:
-DLLAMA_BUILD_MTMD=ON
```

### Slow inference

**Checklist**:
1. Use a quantized model (Q4_K_M or Q8_0)
2. Verify GPU is enabled: `ForgeSDK.isGPUAvailable`
3. Reduce context size if it's too large
4. Check you're not running on a simulator (no Metal)

---

## Model Recommendations by Device

| Device RAM | Recommended Models |
|------------|-------------------|
| 4GB | TinyLlama 1.1B Q4_K_M |
| 6GB | Qwen2.5-1.5B Q4_K_M, Phi-3-mini Q4_K_M |
| 8GB+ | Qwen2.5-3B Q4_K_M, LLaMA-3.2-3B Q4_K_M |
| 16GB+ | LLaMA-3.1-8B Q4_K_M, Qwen2.5-7B Q4_K_M |

---

## Architecture

```
Forge SDK
├── Forge.swift          # High-level Swift API (ForgeSDK class)
├── AI.swift             # Inference orchestration (queues, model lifecycle)
├── LLaMa.swift          # LLaMA GGUF implementation
├── LLaMa_MModal.swift   # Vision/multimodal extension (CLIP integration)
├── LLMBase.swift        # Abstract base class for all models
└── ForgeCore (C++)      # C++ bridge to llama.cpp
    ├── exception_helper_objc.mm  # Exception bridging
    └── llama.xcframework         # Pre-built llama.cpp
```

---

## Build Scripts

| Script | Purpose |
|--------|---------|
| `llama.cpp/build-xcframework.sh` | Build universal XCFramework |
| `build-xcframework_cpu.sh` | Build CPU-only framework |

---

## Roadmap

- [x] Basic text generation
- [x] GPU acceleration (Metal)
- [x] Context state persistence
- [x] LoRA adapter support
- [ ] Native Whisper audio integration
- [ ] Speculative decoding
- [ ] Quantization-aware fine-tuning
