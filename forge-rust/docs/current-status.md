# Forge-Rust Integration: Current Status

**Last Updated:** January 6, 2026

## Project Structure

```
amma-forge-sdk/
├── forge-rust/                     # Rust wrapper (Cargo workspace)
│   ├── forge-core/                 # Safe Rust API with RAII
│   │   ├── src/
│   │   │   ├── lib.rs              # Module exports
│   │   │   ├── engine.rs           # ForgeEngine (main API)
│   │   │   ├── model.rs            # LlamaModel with Drop
│   │   │   ├── context.rs          # LlamaContext with batch arena
│   │   │   ├── batch.rs            # Arena-style batch allocator
│   │   │   ├── sampler.rs          # Full sampler chain
│   │   │   ├── multimodal.rs       # Vision/CLIP support
│   │   │   ├── memory.rs           # Memory tracking
│   │   │   ├── error.rs            # Error types
│   │   │   └── ffi.rs              # C FFI for Swift
│   │   └── tests/
│   │       └── integration_test.rs # 7 passing tests
│   ├── llama-cpp-sys/              # Low-level FFI bindings
│   ├── build/
│   │   └── ForgeRustCore.xcframework  # Built Rust binary (63MB)
│   ├── docs/
│   │   ├── testing-issues.md       # Problems & solutions
│   │   └── current-status.md       # This file
│   └── scripts/
│       ├── build-ios.sh            # Build for iOS
│       └── package-xcframework.sh  # Create XCFramework
│
├── ForgeSwift/                     # Swift wrapper package
│   ├── Package.swift
│   └── Sources/ForgeSwift/
│       └── ForgeSwift.swift        # Swift API with auto-detection
│
├── llama.xcframework/              # Pre-built llama.cpp
│
└── docs/
    └── forge-rust-integration-plan.md  # Full integration plan
```

## Completed Work

### Phase 1-3: Rust Layer ✅
- [x] Cargo workspace with forge-core + llama-cpp-sys
- [x] Safe Rust wrappers with RAII (automatic cleanup)
- [x] Full sampler chain (penalties → DRY → Top-K → Top-P → Min-P → Temperature → Distribution)
- [x] Memory tracking
- [x] C FFI with cbindgen-generated header
- [x] Multimodal context loading (vision placeholder)

### Phase 4: Build & Link ✅
- [x] iOS build script (arm64 device + universal simulator)
- [x] XCFramework packaging (ForgeRustCore.xcframework)
- [x] Linking against llama.xcframework

### Phase 5: Swift Wrapper ✅
- [x] ForgeSwift package created
- [x] ForgeConfiguration with presets (auto, mobile, desktop, vision)
- [x] Device auto-detection (RAM, cores)
- [x] ForgeEngine class with generate/generateTurn/generateVision
- [x] Proper callback handling for streaming tokens

## Key Fixes Applied

1. **CRITICAL: Double-indirection pointer bug** - Stored raw `*mut llama_model` instead of `*const LlamaModel`
2. **KV cache API change** - Use `llama_get_memory()` + `llama_memory_clear()`
3. **Sampler API** - Functions now take `*const llama_vocab` instead of model
4. **Backend lifecycle** - Use LazyLock for one-time init, never cleanup during tests
5. **iOS cross-compilation** - Skip bindgen, use stubs for iOS targets

## Test Results

All 7 Rust tests pass:
```
test test_backend_lifecycle ... ok
test test_config_creation ... ok
test test_memory_tracking ... ok
test test_model_loading ... ok
test test_simple_generation ... ok
test test_text_generation ... ok
test test_tokenization ... ok
```

Swift package builds successfully.

## Device Auto-Detection Tiers

### iOS
| Tier | RAM | Context | Batch | Threads | Max Tokens | Memory Budget |
|------|-----|---------|-------|---------|------------|---------------|
| Low | <4 GB | 1024 | 64 | 2 | 128 | 500 MB |
| Standard | 4-6 GB | 2048 | 128 | 2 | 256 | 800 MB |
| Pro | 6-8 GB | 4096 | 256 | 4 | 256 | 1200 MB |
| Pro Max | ≥8 GB | 4096 | 256 | 6 | 512 | 2000 MB |

### macOS
| Tier | RAM | Context | Batch | Threads | Max Tokens |
|------|-----|---------|-------|---------|------------|
| Entry | 8 GB | 4096 | 256 | 4 | 512 |
| Standard | 16 GB | 8192 | 512 | 8 | 1024 |
| High-end | ≥32 GB | 16384 | 512 | 12 | 2048 |

## Context Window Sliding (NEW)

When conversations get too long, the engine automatically shifts the context:

1. **Trigger**: When `n_past` reaches 90% of `n_ctx`
2. **Algorithm**:
   - Keep first `n_keep` tokens (system prompt)
   - Remove half of remaining old tokens
   - Shift positions of remaining tokens
3. **API**:
   - Rust: `context.shift_context_if_needed(&mut n_past, n_keep)`
   - Swift: `engine.tokensToKeep = 50` (set system prompt size)
   - Swift: `engine.canShiftContext` (check if supported)

### Usage Example

```swift
let engine = try ForgeEngine(modelPath: path)
engine.tokensToKeep = 100  // Preserve first 100 tokens (system prompt)

// Long conversation - context will auto-shift when needed
for turn in conversation {
    try engine.generateTurn(prompt: turn, addBOS: engine.isFirstTurn) { token in
        print(token, terminator: "")
    }
}
```

## Example App Created ✅

The `example-swift/` folder contains a full-featured iOS app built with ForgeSwift:

```
example-swift/
├── Sources/AmmaRecognize/
│   ├── AmmaRecognizeApp.swift     # Entry point, Forge init
│   ├── ViewModels/
│   │   └── VisionViewModel.swift  # ForgeEngine integration
│   ├── Views/
│   │   └── MainView.swift         # Main UI
│   └── Models/
│       ├── DocumentProcessor.swift
│       └── SpeechRecognizer.swift
├── project.yml                    # XcodeGen config
└── README.md
```

### Building the Example App

```bash
cd example-swift
xcodegen generate
open AmmaRecognize-Rust.xcodeproj
```

## What's Still Needed

### Vision Generation (Placeholder)
The `generate_vision_rgba()` function currently processes images but doesn't actually use the embeddings. The mtmd API changed significantly:
- Old: `mtmd_helper_eval_chunks()`
- New: `mtmd_encode_chunk()` + `mtmd_get_output_embd()`

This needs reimplementation before vision models work fully.

## Build Commands

```bash
# Build Rust for iOS
cd forge-rust && ./scripts/build-ios.sh

# Package XCFramework
./scripts/package-xcframework.sh

# Build Swift wrapper
cd ForgeSwift && swift build

# Run Rust tests
cd forge-rust
DYLD_FRAMEWORK_PATH=../llama.xcframework/macos-arm64_x86_64 \
FORGE_TEST_MODEL=/path/to/model.gguf \
cargo test --release --package forge-core --test integration_test -- --nocapture --test-threads=1
```
