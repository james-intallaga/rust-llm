# Performance and Quality Issues

This document records investigation and fixes for performance and response quality issues in Forge SDK.

## Problem Summary

**Symptoms observed:**
- Responses were verbose and unfocused
- Same model, same prompts, different quality depending on configuration

**Example comparison:**

| Configuration | Response to "How are you doing?" |
|---------------|----------------------------------|
| Optimized | "I'm doing great, thank you for asking! How about you?" |
| Default | "I hope you're having a great day! It's lovely to chat with you today. Let's talk about your favorite things. What do you enjoy doing in your free time? Do you have any hobbies or activities that bring you joy?..." |

---

## Root Cause Analysis

### Key Findings

| Parameter | Default | Optimized | Impact |
|-----------|---------|-----------|--------|
| **Temperature** | 0.7 | 0.3 | High - causes unfocused output |
| **Quantization** | Q4_0 | Q8_0 | Medium - affects model quality |
| **top_p** | 0.9 | 0.9 | None - same |
| **top_k** | 40 | 40 | None - same |

### Why Temperature Matters

Temperature controls randomness in token selection:

- **Low (0.1-0.3):** Deterministic, focused, picks highest probability tokens
- **Medium (0.5-0.7):** Balanced, some creativity
- **High (0.8-1.0+):** Creative, unpredictable, can go off-topic

For **vision/identification tasks**, low temperature (0.3) is optimal because:
- The model should identify what it sees, not be creative
- Responses should be factual and concise
- Less chance of hallucination

### Why Quantization Matters

| Quantization | Bits | Size | Quality |
|--------------|------|------|---------|
| Q4_0 | 4-bit | ~1.5GB | Good, some precision loss |
| Q8_0 | 8-bit | ~2.8GB | Better, closer to original |
| F16 | 16-bit | ~6GB | Best, original precision |

Q8_0 preserves more model weights, leading to:
- More accurate responses
- Better instruction following
- Fewer "garbage" tokens

---

## Fixes Applied

### Fix 1: Lower Temperature for Vision

```swift
// Vision config
config.temperature = 0.3  // Low for focused vision responses
config.topP = 0.9
config.topK = 40
```

### Fix 2: Use Q8_0 Quantization

```swift
// Use Q8_0 instead of Q4_0 for better quality
url: "https://huggingface.co/LiquidAI/LFM2-VL-3B-GGUF/resolve/main/LFM2-VL-3B-Q8_0.gguf"
```

---

## Previous Fixes (Sampler Chain)

### Fix 3: Missing Repeat Penalty Sampler

The sampler chain was missing `llama_sampler_init_penalties`, causing repetitive output.

```swift
llama_sampler_chain_add(self.sampling,
    llama_sampler_init_penalties(
        sampleParams.repeat_last_n,
        sampleParams.repeat_penalty,
        sampleParams.frequence_penalty,
        sampleParams.presence_penalty
    ))
```

### Fix 4: Missing Min-P Sampler

Min-P filtering removes low-probability "garbage" tokens.

```swift
if sampleParams.min_p > 0 {
    llama_sampler_chain_add(self.sampling,
        llama_sampler_init_min_p(sampleParams.min_p, 1))
}
```

### Fix 5: Wrong Sampler Order

The distribution sampler (`llama_sampler_init_dist`) must be **last** in the chain.

```swift
// Correct order:
// 1. temp → 2. top_k → 3. top_p → 4. min_p → 5. typical_p
// → 6. mirostat → 7. penalties → 8. dist (LAST)
```

### Fix 6: Sampler State Not Reset

For multimodal inference, the sampler state must be reset between images.

```swift
llama_sampler_reset(sampler)
```

### Fix 7: Flash Attention API Change

The llama.cpp API changed from boolean to enum.

```swift
// Before (broken)
context_params.flash_attn = contextParams.flash_attn

// After (working)
context_params.flash_attn_type = contextParams.flash_attn
    ? LLAMA_FLASH_ATTN_TYPE_ENABLED
    : LLAMA_FLASH_ATTN_TYPE_DISABLED
```

### Fix 8: Random Seed

Hardcoded seed was causing reproducible but suboptimal sampling.

```swift
// Before
llama_sampler_init_dist(1234567)

// After
llama_sampler_init_dist(UInt32.random(in: 0..<UInt32.max))
```

---

## Complete Sampler Chain (Final)

```swift
func init_sampling_param() {
    let randomSeed = UInt32.random(in: 0..<UInt32.max)

    // Greedy for temp=0
    if sampleParams.temp == 0 {
        llama_sampler_chain_add(self.sampling, llama_sampler_init_greedy())
        return
    }

    // 1. Temperature
    llama_sampler_chain_add(self.sampling, llama_sampler_init_temp(sampleParams.temp))

    // 2. Top-K
    if sampleParams.top_k > 0 {
        llama_sampler_chain_add(self.sampling, llama_sampler_init_top_k(sampleParams.top_k))
    }

    // 3. Top-P
    if sampleParams.top_p > 0 && sampleParams.top_p < 1.0 {
        llama_sampler_chain_add(self.sampling, llama_sampler_init_top_p(sampleParams.top_p, 1))
    }

    // 4. Min-P
    if sampleParams.min_p > 0 {
        llama_sampler_chain_add(self.sampling, llama_sampler_init_min_p(sampleParams.min_p, 1))
    }

    // 5. Typical-P
    if sampleParams.typical_p > 0 && sampleParams.typical_p < 1.0 {
        llama_sampler_chain_add(self.sampling, llama_sampler_init_typical(sampleParams.typical_p, 1))
    }

    // 6. Mirostat (if enabled)
    if sampleParams.mirostat == 2 {
        llama_sampler_chain_add(self.sampling,
            llama_sampler_init_mirostat_v2(randomSeed, sampleParams.mirostat_tau, sampleParams.mirostat_eta))
    }

    // 7. Penalties
    if sampleParams.repeat_penalty != 1.0 || sampleParams.frequence_penalty != 0.0 || sampleParams.presence_penalty != 0.0 {
        llama_sampler_chain_add(self.sampling,
            llama_sampler_init_penalties(
                sampleParams.repeat_last_n,
                sampleParams.repeat_penalty,
                sampleParams.frequence_penalty,
                sampleParams.presence_penalty
            ))
    }

    // 8. Distribution (MUST BE LAST)
    llama_sampler_chain_add(self.sampling, llama_sampler_init_dist(randomSeed))
}
```

---

## Recommended Settings by Model

See [Model Settings](../developer/model-settings.md) for official recommended parameters for each supported model.

---

## Verification

After applying all fixes:

| Metric | Before | After |
|--------|--------|-------|
| Response focus | Verbose, off-topic | Concise, relevant |
| Token quality | Some garbage | Clean output |
| Vision accuracy | Inconsistent | Reliable |
| Second image | Nonsense | Correct |

---

## References

- [llama.cpp Sampling Documentation](https://github.com/ggerganov/llama.cpp/blob/master/examples/main/README.md)
- [LiquidAI LFM2 Models](https://huggingface.co/LiquidAI)
