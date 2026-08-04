# Forge-Rust Testing Issues and Solutions

This document records the problems encountered while getting the Rust integration tests to pass, and the solutions implemented.

## Overview

The `forge-core` crate includes integration tests that verify the Rust wrappers work correctly with `llama.cpp`. These tests load actual models and run inference, which uncovered several subtle issues related to GPU resource management and API compatibility.

---

## 🚨 CRITICAL Issue: Double-Indirection Pointer Bug (The Root Cause)

### Symptom
Random crashes during sampling, tokenization, or any operation that accessed the model through the context. Crashes were inconsistent - sometimes working, sometimes crashing with `EXC_BAD_ACCESS`.

### Root Cause
**This was the hardest bug to find and the most important fix.**

The `LlamaContext` was storing a pointer to the Rust `LlamaModel` wrapper struct:

```rust
// WRONG - Double indirection!
pub struct LlamaContext {
    ptr: NonNull<llama_context>,
    model: *const LlamaModel,  // Points to Rust wrapper, not C struct
    // ...
}
```

When we later tried to get the vocab or call model functions:

```rust
// This caused undefined behavior!
let vocab = llama_model_get_vocab((*self.model).as_ptr());
```

The problem: `self.model` pointed to the Rust `LlamaModel` struct in memory. If that struct moved (e.g., due to `Box`, `Vec` reallocation, or stack unwinding), the pointer became invalid. Even worse, the double-dereference `(*self.model).as_ptr()` was accessing potentially freed memory.

### Why It Was Hard to Debug
- Crashes were **non-deterministic** - depended on memory layout
- Sometimes worked in debug builds, crashed in release builds
- Stack traces pointed to Metal/GPU code, not the actual bug location
- The bug only manifested when the model pointer was actually used

### Solution
Store the **raw C pointer** directly, not a pointer to the Rust wrapper:

```rust
// CORRECT - Direct C pointer
pub struct LlamaContext {
    ptr: NonNull<llama_context>,
    model_ptr: *mut llama_model,  // Direct C pointer from LlamaModel::as_ptr()
    // ...
}

impl LlamaContext {
    pub fn new(model: &LlamaModel, params: ContextParams) -> Result<Self> {
        // ...
        Ok(Self {
            ptr,
            model_ptr: model.as_ptr(),  // Store the raw C pointer immediately
            // ...
        })
    }

    pub fn tokenize(&self, text: &str, add_special: bool) -> Result<Vec<i32>> {
        // Direct use - no double dereference!
        let vocab = unsafe { llama_model_get_vocab(self.model_ptr) };
        // ...
    }
}
```

### Lesson Learned
When wrapping C libraries in Rust:
- **Never store pointers to Rust wrapper structs** if you need to access the underlying C pointer later
- **Always store the raw C pointer directly** when it needs to outlive the wrapper's borrow
- The Rust borrow checker can't help you with raw pointers - manual lifetime management is required

---

## Issue 1: Metal State Corruption from Backend Cleanup

### Symptom
Tests would crash with Metal/GPU errors when running in sequence:
```
error: Execution was interrupted, reason: EXC_BAD_ACCESS (code=1, address=0x...)
```

### Root Cause
The `llama_backend_free()` function (called by `forge_cleanup()`) tears down the Metal compute context. If called while other tests are waiting to run, subsequent tests that try to initialize the backend again will have corrupted Metal state.

### Failed Attempts
1. Adding `forge_cleanup()` at the end of each test → Crashes on test 2+
2. Using `--test-threads=1` to serialize tests → Still crashes because cleanup corrupts shared state

### Solution
Use `LazyLock` to initialize the backend exactly **once** for all tests, and **never** call cleanup:

```rust
use std::sync::LazyLock;

static BACKEND: LazyLock<()> = LazyLock::new(|| {
    forge_init();
});

#[test]
fn test_example() {
    LazyLock::force(&BACKEND);  // Initialize if not already
    // ... test code ...
    // NO cleanup - process exit handles it
}
```

**Why this works:** The OS automatically cleans up all resources when the test process exits. This is safe and common practice for GPU contexts.

---

## Issue 2: KV Cache Clear Causing Crashes

### Symptom
Calling `clear_kv_cache()` between generations caused crashes in Metal execution.

### Root Cause
The `llama.cpp` API changed. The old `llama_kv_cache_clear()` function was replaced with:
1. `llama_get_memory()` - Get the memory handle from context
2. `llama_memory_clear()` - Clear the memory

### Solution
Updated `context.rs` to use the new API:

```rust
pub fn clear_kv_cache(&mut self) {
    unsafe {
        let mem = llama_get_memory(self.ptr.as_ptr());
        if !mem.is_null() {
            llama_memory_clear(mem, true);
        }
    }
}
```

---

## Issue 3: DRY Sampler Requiring Model Reference

### Symptom
The DRY (Don't Repeat Yourself) sampler would crash or produce garbage output.

### Root Cause
`llama_sampler_init_dry()` requires:
1. A pointer to `llama_vocab` (not `llama_model`)
2. The `n_ctx_train` value from the model
3. Proper null handling for sequence breakers

### Solution
Pass the model reference when creating the sampler, and use it to get the vocab:

```rust
impl Sampler {
    pub fn new(params: SamplerParams, model: Option<*const llama_model>) -> Self {
        // ...
        if params.dry_multiplier > 0.0 {
            if let Some(model_ptr) = model {
                let vocab = unsafe { llama_model_get_vocab(model_ptr) };
                let n_ctx_train = unsafe { llama_model_n_ctx_train(model_ptr) };
                // Initialize DRY sampler with correct params
            }
        }
    }
}
```

---

## Issue 4: Context Pointer Type Mismatch

### Symptom
Crash when calling `llama_tokenize`, `llama_token_to_piece`, or `llama_vocab_is_eog`.

### Root Cause
These functions changed from taking `*const llama_model` to taking `*const llama_vocab`. The `LlamaContext` was storing a reference to the Rust `LlamaModel` wrapper instead of the raw C pointer.

### Solution
Store the raw `*mut llama_model` pointer directly in `LlamaContext`:

```rust
pub struct LlamaContext {
    ptr: NonNull<llama_context>,
    model_ptr: *mut llama_model,  // Raw C pointer, not &LlamaModel
    batch: BatchArena,
    n_ctx: u32,
}
```

Then use `llama_model_get_vocab()` to get the vocab when needed:

```rust
pub fn tokenize(&self, text: &str, add_special: bool) -> Result<Vec<i32>> {
    let vocab = unsafe { llama_model_get_vocab(self.model_ptr) };
    // ... use vocab for tokenization
}
```

---

## Issue 5: Bindgen Failing for iOS Cross-Compilation

### Symptom
Build fails with:
```
error: version 'sim' in target triple 'arm64-apple-ios-sim' is invalid
```

### Root Cause
`bindgen` tries to run clang for the target architecture, but fails when cross-compiling for iOS because the host (macOS) clang doesn't understand iOS-specific target triples.

### Solution
Skip `bindgen` for iOS targets and use pre-defined stubs instead:

```rust
// build.rs
let is_ios_target = target.contains("apple-ios");

if !is_ios_target {
    // Generate bindings with bindgen
} else {
    println!("cargo:warning=Cross-compiling for iOS, using stub bindings");
}
```

The stubs in `llama-cpp-sys/src/lib.rs` provide the necessary type definitions and function declarations.

---

## Issue 6: XCFramework Binary Naming

### Symptom
`xcodebuild -create-xcframework` fails with:
```
error: unable to find any architecture information in the binary
```

### Root Cause
The framework bundle's binary must have the **exact same name** as the framework. E.g., `ForgeRustCore.framework/ForgeRustCore`, not `ForgeRustCore.framework/libforge_core.a`.

### Solution
Updated `package-xcframework.sh` to copy and rename the binary:

```bash
# Copy the static library AS the framework binary
cp "${lib_path}" "${framework_dir}/${FRAMEWORK_NAME}"
```

---

## Testing Best Practices

Based on these issues, here are the recommended practices:

1. **Backend Lifecycle**: Initialize once with `LazyLock`, never cleanup during tests
2. **Test Isolation**: Run with `--test-threads=1` to prevent race conditions
3. **Model Path**: Use `FORGE_TEST_MODEL` env var or check common locations
4. **Error Handling**: Tests should gracefully skip if no model is available

### Running Tests

```bash
# With a specific model
FORGE_TEST_MODEL=/path/to/model.gguf cargo test --release -- --nocapture --test-threads=1

# With automatic model discovery
DYLD_FRAMEWORK_PATH=/path/to/llama.xcframework/macos-arm64_x86_64 \
cargo test --release -- --nocapture --test-threads=1
```

---

## Summary of Changes

| Priority | File | Change |
|----------|------|--------|
| 🚨 **CRITICAL** | `forge-core/src/context.rs` | Store `*mut llama_model` (raw C pointer) instead of `*const LlamaModel` (Rust wrapper pointer) - **fixed the root cause crash** |
| High | `forge-core/src/context.rs` | Use `llama_get_memory` + `llama_memory_clear` for KV cache |
| High | `forge-core/src/sampler.rs` | Pass model reference for DRY sampler initialization |
| Medium | `llama-cpp-sys/build.rs` | Skip bindgen for iOS cross-compilation |
| Medium | `scripts/package-xcframework.sh` | Rename binary to match framework name |
| Medium | `forge-core/tests/integration_test.rs` | Use `LazyLock` for one-time backend init |
