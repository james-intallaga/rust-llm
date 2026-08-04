# Forge Rust Layer - Stubs and Potential Issues

This document records all stubs, placeholders, and potential issues identified during the code review.
Reference this before the Swift wrapper implementation to ensure nothing is forgotten.

---

## 1. FFI Stubs in `llama-cpp-sys/src/lib.rs`

**Status:** These are **placeholder stubs** that allow the code to compile without the actual llama.cpp headers.

**Location:** Lines 26-341 (inside `#[cfg(not(feature = "generate"))] mod stubs { ... }`)

**What they are:**
- Stub type definitions for `llama_model`, `llama_context`, `llama_sampler`, `llama_batch`, etc.
- Stub function declarations for all `llama_*` and `mtmd_*` functions

**When replaced:**
- These stubs are **automatically replaced** by bindgen-generated bindings when:
  1. The `generate` feature is enabled
  2. The `llama.xcframework` headers are present at the expected path

**Action for Swift wrapper:**
- The XCFramework build script must link against the real `llama.xcframework`
- No code changes needed - stubs will be replaced automatically

---

## 2. Incomplete `ForgeParams` → `ForgeConfig` Conversion

**Location:** `forge-core/src/ffi.rs:89-97`

**Issue:** The FFI `ForgeParams` struct doesn't include all sampler parameters:
- ❌ `min_p` - not passed through
- ❌ `repeat_penalty` - not passed through
- ❌ `repeat_last_n` - not passed through
- ❌ DRY parameters - not passed through
- ❌ `typical_p` - not passed through

**Current code:**
```rust
impl From<ForgeParams> for ForgeConfig {
    fn from(params: ForgeParams) -> Self {
        ForgeConfig::default()
            .with_context_size(params.n_ctx)
            .with_batch_size(params.n_batch)
            .with_max_tokens(params.max_tokens)
            .with_temperature(params.temperature)
            .with_memory_budget(params.memory_budget_mb as usize)
    }
}
```

**Recommendation:**
Either:
1. Extend `ForgeParams` with all sampler fields, OR
2. Add `forge_set_sampler_*` FFI functions to set individual params

**For Swift wrapper:**
- Use the default values for now (they are LFM2-VL optimized)
- Or extend the FFI if different values are needed

---

## 3. Unused Field: `image_buffer`

**Location:** `forge-core/src/multimodal.rs:62`

**Issue:** `image_buffer: Vec<u8>` is declared in `MultimodalContext` but never used.

```rust
pub struct MultimodalContext {
    ptr: NonNull<mtmd_context>,
    /// Reusable buffer for image data to avoid repeated allocations
    image_buffer: Vec<u8>,  // UNUSED
}
```

**Recommendation:** Either implement buffer reuse or remove the field.

**Impact:** Minor (just wasted 4MB pre-allocation).

---

## 4. Skipped Samplers

**Location:** `forge-core/src/sampler.rs`

**The following samplers are not implemented:**

| Sampler | Line | Reason |
|---------|------|--------|
| Logit Bias | 150 | "skipped for now (requires explicit bias list)" |
| Top-N-Sigma | 193 | "not commonly used, skipped" |
| XTC | 228 | "not commonly used, skipped" |
| Mirostat | - | Not implemented (use Top-P/Min-P instead) |

**Impact:**
- Logit Bias: May be needed for suppressing specific tokens
- Top-N-Sigma, XTC, Mirostat: Not commonly used, safe to skip

**For Swift wrapper:**
- These can be added later if needed
- Current sampler chain is complete for LFM2-VL

---

## 5. Memory Tracker Limitation

**Location:** `forge-core/src/memory.rs`

**Issue:** `MemoryTracker` only tracks Rust-side allocations. It does NOT track:
- llama.cpp internal allocations
- Metal GPU buffers
- KV cache growth

**Current behavior:** Memory budget is checked but may not reflect actual process memory.

**Recommendation:** For accurate memory tracking on iOS, use `task_vm_info` in Swift wrapper (as done in Swift SDK).

---

## 6. Raw Model Pointer in Context

**Location:** `forge-core/src/context.rs:66`

```rust
pub struct LlamaContext {
    ptr: NonNull<llama_context>,
    model: *const LlamaModel,  // Raw pointer!
    // ...
}
```

**Issue:** This raw pointer could become dangling if `LlamaModel` is dropped before `LlamaContext`.

**Current mitigation:** `ForgeEngine` owns both and drops them in correct order.

**Recommendation:** Consider using `Rc<LlamaModel>` or keep current design with clear documentation.

---

## 7. CBIndgen Warning

**Location:** Build output

**Warning:**
```
the option `Z` is only accepted on the nightly compiler
```

**Impact:** None. `cbindgen` still generates the header file. The warning is about macro expansion for documentation only.

---

## Summary: What Swift Wrapper Must Handle

### Required (from model-tuning-guide.md):

1. ✅ **BOS Token Control** - Use `forge_is_first_turn()` and `forge_generate_turn()`
2. ✅ **Chat Template Formatting** - Apply model-specific template in Swift
3. ✅ **EOG Token Handling** - Handled automatically by Rust layer
4. ✅ **Sampler Reset** - Handled automatically by Rust layer

### Optional Enhancements:

1. ⚠️ Extend `ForgeParams` to include all sampler parameters
2. ⚠️ Add real memory tracking using iOS APIs
3. ⚠️ Add Logit Bias support if needed for token suppression

---

## Files Changed in This Review

- `llama-cpp-sys/src/lib.rs` - Fixed `mtmd_tokenize` signature, added `mtmd_input_text` struct
- `forge-core/src/multimodal.rs` - Fixed `process_image()` to use correct `mtmd_tokenize` API
- `forge-core/src/engine.rs` - Added `last_eog_token`, EOG feedback, `generate_turn()`, `is_first_turn()`, `n_past()`
- `forge-core/src/ffi.rs` - Added `forge_generate_turn()`, `forge_is_first_turn()`, `forge_n_past()`
- `include/forge_ffi.h` - Updated with new FFI functions

---

## Added Multimodal Helper Functions (Second Review)

Based on Swift SDK multimodal implementation, the following **generic** functions were added to the Rust layer:

### Added to `llama-cpp-sys/src/lib.rs`:

| Function | Purpose |
|----------|---------|
| `mtmd_support_vision()` | Check if model supports vision |
| `mtmd_default_marker()` | Get media marker string (e.g., "<image>") |
| `mtmd_input_chunks_size()` | Get number of chunks after tokenization |
| `mtmd_helper_get_n_tokens()` | Get total token count |
| `mtmd_helper_bitmap_init_from_file()` | Load image from file path |
| `mtmd_bitmap_get_nx()` / `mtmd_bitmap_get_ny()` | Get image dimensions |
| `mtmd_context_params.warmup` | Warmup flag (compile Metal pipelines) |

### Added to `forge-core/src/multimodal.rs`:

| Function | Purpose |
|----------|---------|
| `get_default_media_marker()` | Get media marker string (safe Rust wrapper) |
| `MultimodalContext::supports_vision()` | Check if vision is supported |
| `MultimodalContext::process_image_from_file()` | Load and tokenize image from file path |
| `InputChunks::n_chunks()` | Get chunk count |
| `InputChunks::n_tokens()` | Get token count |
| `MultimodalParams::warmup` | Warmup flag |

### Added to FFI (`ffi.rs` and `forge_ffi.h`):

| Function | Purpose |
|----------|---------|
| `forge_get_media_marker()` | Get media marker for Swift to use in prompts |

---

## What Stays in Swift Layer (Model-Specific)

These are **NOT** generic and should remain in Swift:

| Feature | Reason |
|---------|--------|
| Chat template formatting | Model-specific (LFM2 vs Llama vs Mistral) |
| EOS/EOT logit bias for first token | Workaround for specific models |
| Memory logging with `task_vm_info` | iOS-specific API |
| Context/n_predict clamping | Runtime tuning based on device |
| KVShift | Model-specific context management |

---

*Last updated: 2026-01-06*
