# Troubleshooting Multimodal (Vision) Issues

This guide documents common issues encountered when using Forge SDK's multimodal (vision) capabilities and their solutions.

## Issue 1: Image Fails to Generate Sensible Response

### Symptoms
- First image works correctly and produces accurate descriptions
- Subsequent images produce "nonsense" output (gibberish, repetitive tokens, or completely wrong descriptions)
- The model seems to "remember" the previous image instead of analyzing the new one

### Root Cause
The `llama_sampler` maintains internal state that accumulates during inference:
- **Repeat penalty tracker** - remembers which tokens were generated to avoid repetition
- **Mirostat μ (mu)** - adaptive learning rate that adjusts based on output
- **Random number generator (RNG)** - seeded state that affects sampling randomness
- **DRY sampler state** - "Don't Repeat Yourself" token history

When you clear the KV cache for a new image but **don't reset the sampler**, the sampler still "remembers" tokens from the previous image's generation. This causes it to incorrectly penalize or avoid tokens for the new image, resulting in corrupted output.

### Solution
Reset both the memory (KV cache) AND the sampler state before each new image inference:

```swift
// In LLaMa_MModal.swift - Predict() function

// 1. Clear the KV cache and recurrent memory
if let ctx = self.context {
    let memory = llama_get_memory(ctx)
    if memory != nil {
        llama_memory_clear(memory, true)
        self.nPast = 0
        self.outputRepeatTokens = []
    }
}

// 2. CRITICAL: Reset the sampler to clear accumulated state
resetSampler()
```

The `resetSampler()` method calls `llama_sampler_reset()` which clears:
- Repeat penalty history
- Mirostat μ value
- RNG state
- Any other accumulated sampler state

### Files Changed
- `Forge/Sources/Forge/LLaMa.swift` - Added `resetSampler()` method
- `Forge/Sources/Forge/LLaMa_MModal.swift` - Call `resetSampler()` in `Predict()`

---

## Issue 2: App Hangs After Image Generation (Thread Blocked)

### Symptoms
- Image is processed correctly and generates a valid response
- The app freezes/hangs after the response is generated
- UI becomes unresponsive
- Debugger shows the main thread is blocked

### Root Cause
The `VisionViewModel.updateCurrentSession()` was attempting to:
1. Encode large `UIImage` data to JPEG format
2. Write the encoded data to `UserDefaults`
3. Perform both operations **on the main thread**

This caused the UI to freeze because:
- JPEG encoding is CPU-intensive
- `UserDefaults` writes are synchronous
- Large images (several MB) can take significant time to process

### Solution
1. **Don't persist raw image data** - Store a placeholder text instead
2. **Move session saving to a background thread**

```swift
// In VisionViewModel.swift

private func updateCurrentSession() {
    // Don't persist the actual image - just mark that an image was attached
    // This prevents the main thread from blocking on large image encoding
    let messageToSave = SavedMessage(
        role: message.role,
        content: message.role == .user ? "[Image attached]" : message.content,
        imageData: nil  // Don't save image data
    )

    // Save to UserDefaults asynchronously
    saveSessionsAsync()
}

private func saveSessionsAsync() {
    DispatchQueue.global(qos: .background).async {
        // JSON encoding and UserDefaults write happens off main thread
        if let encoded = try? JSONEncoder().encode(self.sessions) {
            UserDefaults.standard.set(encoded, forKey: "savedSessions")
        }
    }
}
```

### Files Changed
- `Examples/AmmaRecognize-ForgeTest/VisionViewModel.swift`

---

## Issue 3: Second Image Doesn't Work (Corrupted Output)

### Symptoms
- First image inference works perfectly
- Second image inference produces:
  - Completely wrong descriptions
  - Repetitive tokens or loops
  - Gibberish output
  - The same response as the first image

### Root Cause
This is the same issue as **Issue 1** (sampler state not being reset), but manifests specifically when:
1. User processes Image A successfully
2. User immediately processes Image B
3. Image B's output is corrupted by Image A's sampler state

For hybrid models like LFM2 (which use both attention and recurrent memory), clearing memory involves:
- `mem_attn->clear()` - Clears the KV cache (attention layers)
- `mem_recr->clear()` - Clears the recurrent memory (recurrent layers)

Both are cleared by `llama_memory_clear(memory, true)`, but the **sampler** is a separate component.

### Solution
Same as Issue 1 - ensure `resetSampler()` is called before each new image inference:

```swift
// Clear ALL state for fresh image inference
llama_memory_clear(memory, true)  // Clears KV cache + recurrent memory
self.nPast = 0
self.outputRepeatTokens = []
resetSampler()  // Clears sampler state (repeat penalty, mirostat, RNG)
```

### Verification
After the fix, you should see this log output for each image:

```
LLaMa_MModal: Clearing KV cache for fresh image inference
LLaMa_MModal: Resetting sampler state
```

Each image should now be processed completely independently.

---

## Debugging Tips

### Enable Debug Logging
The `LLaMa_MModal` class includes extensive debug logging. Look for these key log entries:

```
LLaMa_MModal: ========================================
LLaMa_MModal: Starting multimodal prediction
LLaMa_MModal: Image path: /path/to/image.jpg
LLaMa_MModal: MTMD initialized: true
LLaMa_MModal: Clearing KV cache for fresh image inference
LLaMa_MModal: Resetting sampler state
LLaMa_MModal: Image file size: 293 KB
...
LLaMa_MModal: Evaluation took 9.47 seconds
LLaMa_MModal: Chunks evaluated, nPast = 807
LLaMa_MModal: Starting token generation...
[EOG]
LLaMa_MModal: ========================================
LLaMa_MModal: Generation complete!
LLaMa_MModal: Total tokens: 879 (72 generated)
LLaMa_MModal: Response length: 408 chars
```

### Key Log Entries to Watch
| Log Entry | What It Means |
|-----------|---------------|
| `Clearing KV cache for fresh image inference` | Memory is being cleared |
| `Resetting sampler state` | Sampler is being reset (critical for multi-image) |
| `Evaluation took X seconds` | Image processing time |
| `[EOG]` | End-of-generation token detected |
| `Generation complete!` | Token generation finished successfully |

### Common Warning Messages
These warnings are **informational** and don't indicate errors:

```
WARNING: the CLIP graph uses unsupported operators by the backend
UPSCALE: type = f32, ne = [64 64 1152 1]
```

This means the UPSCALE operator falls back to CPU for image processing. Performance may be slightly slower, but functionality is not affected.

---

## Summary of Fixes

| Issue | Root Cause | Solution |
|-------|------------|----------|
| First image works, subsequent fail | Sampler state not reset | Call `resetSampler()` before each image |
| App hangs after generation | Main thread blocked by image encoding | Move persistence to background thread, don't save raw images |
| Second image produces garbage | Stale repeat penalty/mirostat state | Same as Issue 1 - reset sampler |

---

## Related Documentation
- [Multimodal Guide](../developer/multimodal.md) - How to use vision models
- [API Reference](../developer/api-reference.md) - Full API documentation
- [Performance Tuning](../developer/performance-tuning.md) - Optimize inference speed
