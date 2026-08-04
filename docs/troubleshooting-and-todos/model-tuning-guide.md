# Model Tuning Guide for Forge SDK

This document records the fine-tuning details and optimizations applied to ensure high-quality model inference. It distinguishes between **universal settings** (applicable to all SLMs) and **model-specific settings** (for particular models like LFM2-VL 3B).

---

## ⚠️ CRITICAL FIXES (Must Apply)

These fixes resolved major issues (repetition, broken responses, role confusion):

| # | Fix | Location | Problem Solved |
|---|-----|----------|----------------|
| **1** | **BOS token only on first turn** | `LLMBase.swift` → `LLMTokenize()` | Model repeating text endlessly |
| **2** | **Use model's native chat template** | `LLMBase.swift` → `Predict()` | Role confusion, "stupid answers" |
| **3** | **KV cache position = `nPast + i`** | `LLMBase.swift` → `llm_eval()` | Broken/incoherent responses |
| **4** | **Feed EOG token back to model** | `LLMBase.swift` → `Predict()` | Multi-turn context loss |
| **5** | **Enable DRY sampler** | `LLaMa.swift` → `initSampler()` | Phrase/sentence repetition |
| **6** | **`llama_sampler_accept` in eval** | `LLaMa.swift` → `llm_eval()` | Repeat penalty not working |

### Quick Code References

```swift
// Fix #1: BOS only on first turn (LLMBase.swift)
let shouldAddBos = (self.nPast == 0)

// Fix #3: Correct KV position (LLMBase.swift)
batch.pos[i] = self.nPast + Int32(i)

// Fix #5: Enable DRY (LLaMa.swift)
llama_sampler_chain_add(sampling, llama_sampler_init_dry(..., 0.8, ...))

// Fix #6: Accept in eval (LLaMa.swift)
llama_sampler_accept(smpl, outputToken)
```

---

## Table of Contents

1. [Universal Settings (All SLMs)](#universal-settings-all-slms)
2. [LFM2-VL 3B Specific Settings](#lfm2-vl-3b-specific-settings)
3. [Chat Template Implementation](#chat-template-implementation)
4. [Sampler Chain Configuration](#sampler-chain-configuration)
5. [Common Issues and Fixes](#common-issues-and-fixes)
6. [Model Profile System](#model-profile-system)

---

## Universal Settings (All SLMs)

These settings apply to **all small language models** regardless of architecture.

### 1. BOS Token Handling

**Critical**: Only add the BOS (Beginning of Sequence) token on the **first turn** of a conversation.

```swift
// In LLMTokenize - only add BOS at nPast == 0
let shouldAddBos = (self.nPast == 0)
```

**Why**: Adding BOS to every user message corrupts recurrent memory in models with state-space architectures (like LFM2, Mamba, etc.) and can cause repetition/degradation in transformer models.

### 2. KV Cache Token Positioning

When evaluating tokens, use the correct position index:

```swift
// Correct: Use nPast + i for each token's position
batch.pos[i] = self.nPast + Int32(i)
```

**Why**: Incorrect positioning overwrites previous context, causing broken or incoherent responses.

### 3. EOG Token Handling

Feed End-of-Generation (EOG) tokens back into the model context:

```swift
// After generation completes, feed EOG token back
if llama_token_is_eog(model, outputToken) {
    // Evaluate the EOG token to update model state
    _ = try? llm_eval(inputBatch: [outputToken])
}
```

**Why**: The model needs to "see" its own EOG token to properly transition between turns in multi-turn conversations.

### 4. Sampler Accept on Evaluation

Call `llama_sampler_accept` during token evaluation, not just during sampling:

```swift
// In llm_eval, after successful decode:
if let smpl = self.sampling {
    llama_sampler_accept(smpl, outputToken)
}
```

**Why**: This updates the sampler's internal state (for repeat penalty, etc.) to include all tokens in context.

### 5. Default Sampling Parameters

These defaults work well across most SLMs:

| Parameter | Default Value | Purpose |
|-----------|---------------|---------|
| `n_batch` | 512 | Batch size for prompt processing |
| `repeat_penalty` | 1.15 | Penalize repeated tokens |
| `repeat_last_n` | 64 | Window for repeat penalty |
| `top_k` | 40 | Top-K sampling |
| `top_p` | 0.95 | Nucleus sampling threshold |
| `min_p` | 0.15 | Minimum probability filter |
| `frequency_penalty` | 0.0 | Penalize frequent tokens |
| `presence_penalty` | 0.0 | Penalize present tokens |

### 6. Sampler Chain Order

The order of samplers matters. Use this chain:

1. **Logit Bias** (if any)
2. **Penalties** (repeat, frequency, presence)
3. **DRY** (Don't Repeat Yourself)
4. **Top-N-Sigma** (if enabled)
5. **Top-K**
6. **Typical-P** (if enabled)
7. **Top-P** (nucleus)
8. **Min-P**
9. **XTC** (if enabled)
10. **Temperature**
11. **Distribution Sampler** (final selection)

---

## LFM2-VL 3B Specific Settings

These settings are optimized for **LiquidAI LFM2-VL 3B** (and similar LFM2 models).

### 1. Temperature

```json
"temperature": 0.3
```

**Why**: Lower temperature (0.3) produces more focused, deterministic responses. Recommended for vision tasks and factual Q&A.

### 2. Chat Template Format

LFM2 models use a specific chat template:

```
<|startoftext|><|im_start|>system
{system_prompt}
<|im_end|><|im_start|>user
{user_message}
<|im_end|><|im_start|>assistant
```

**Key Points**:
- `<|startoftext|>` only at the very beginning (first turn)
- `<|im_start|>` and `<|im_end|>` as role markers
- Leave assistant turn **open** (no closing tag) for generation

### 3. Vision/Multimodal Format

For images, embed the `<image>` marker within the user turn:

```
<|startoftext|><|im_start|>system
{system_prompt}
<|im_end|><|im_start|>user
<image>
{user_message}
<|im_end|><|im_start|>assistant
```

### 4. Special Tokens

| Token | ID | Purpose |
|-------|-----|---------|
| `<\|startoftext\|>` | 1 | BOS token |
| `<\|endoftext\|>` | 2 | EOT token (EOG) |
| `<\|im_start\|>` | 6 | Role start marker |
| `<\|im_end\|>` | 7 | Role end marker (EOG) |
| `<image>` | 396 | Image placeholder |

### 5. Advanced Samplers (Optional)

These can improve output quality for LFM2:

```json
{
  "dry_multiplier": 0.8,
  "dry_base": 1.75,
  "dry_allowed_length": 2,
  "dry_penalty_last_n": 256,
  "top_n_sigma": 0.0,
  "xtc_probability": 0.0,
  "xtc_threshold": 0.1
}
```

### 6. Model Profile (JSON)

Complete profile for LFM2-VL 3B:

```json
{
  "id": "lfm2-vl-3b",
  "name": "LFM2-VL 3B",
  "description": "LiquidAI LFM2 Vision-Language 3B model",
  "architecture": "lfm2",
  "modelPatterns": ["lfm2-vl", "lfm2_vl"],
  "context": {
    "default_size": 4096,
    "max_size": 128000
  },
  "sampling": {
    "temperature": 0.3,
    "top_p": 0.95,
    "top_k": 40,
    "min_p": 0.15,
    "repeat_penalty": 1.15,
    "repeat_last_n": 64,
    "frequency_penalty": 0.0,
    "presence_penalty": 0.0,
    "dry_multiplier": 0.8,
    "dry_base": 1.75,
    "dry_allowed_length": 2,
    "dry_penalty_last_n": 256
  },
  "vision": {
    "supported": true,
    "image_marker": "<image>",
    "max_image_size": 1024
  }
}
```

---

## Chat Template Implementation

### First Turn (with system prompt)

```swift
func lfm2ChatPrompt(system: String, user: String) -> String {
    return """
<|startoftext|><|im_start|>system
\(system)
<|im_end|><|im_start|>user
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

### Subsequent Turns

```swift
func lfm2UserTurn(user: String) -> String {
    return """
<|im_start|>user
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

### Vision First Turn

```swift
func lfm2VisionPrompt(system: String, user: String, imageMarker: String) -> String {
    return """
<|startoftext|><|im_start|>system
\(system)
<|im_end|><|im_start|>user
\(imageMarker)
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

---

## Sampler Chain Configuration

### Implementation (LLaMa.swift)

```swift
func initSampler() {
    let sparams = llama_sampler_chain_default_params()
    sampling = llama_sampler_chain_init(sparams)

    guard let sampling = sampling, let model = model else { return }
    let vocab = llama_model_get_vocab(model)

    // 1. Logit bias (if configured)
    if sampleParams.logit_bias_count > 0 {
        llama_sampler_chain_add(sampling,
            llama_sampler_init_logit_bias(
                llama_vocab_n_tokens(vocab),
                sampleParams.logit_bias_count,
                sampleParams.logit_bias))
    }

    // 2. Penalties
    llama_sampler_chain_add(sampling,
        llama_sampler_init_penalties(
            Int32(sampleParams.repeat_last_n),
            sampleParams.repeat_penalty,
            sampleParams.frequencyPenalty,
            sampleParams.presencePenalty))

    // 3. DRY sampler
    if sampleParams.dry_multiplier > 0 {
        llama_sampler_chain_add(sampling,
            llama_sampler_init_dry(vocab,
                Int32(llama_vocab_n_tokens(vocab)),
                sampleParams.dry_multiplier,
                sampleParams.dry_base,
                Int32(sampleParams.dry_allowed_length),
                Int32(sampleParams.dry_penalty_last_n),
                nil, 0))
    }

    // 4. Top-K
    llama_sampler_chain_add(sampling,
        llama_sampler_init_top_k(sampleParams.top_k))

    // 5. Top-P
    llama_sampler_chain_add(sampling,
        llama_sampler_init_top_p(sampleParams.top_p, 1))

    // 6. Min-P
    llama_sampler_chain_add(sampling,
        llama_sampler_init_min_p(sampleParams.min_p, 1))

    // 7. Temperature
    llama_sampler_chain_add(sampling,
        llama_sampler_init_temp(sampleParams.temp))

    // 8. Distribution sampler (final)
    llama_sampler_chain_add(sampling,
        llama_sampler_init_dist(UInt32.random(in: 0..<UInt32.max)))
}
```

---

## Common Issues and Fixes

### Issue 1: Model Repeats Text

**Symptoms**: Output like "I am well I am well I am well..."

**Causes & Fixes**:
1. **BOS token on every turn** → Only add BOS at `nPast == 0`
2. **Sampler not accepting tokens** → Call `llama_sampler_accept` in `llm_eval`
3. **Weak repeat penalty** → Increase `repeat_penalty` to 1.15+
4. **DRY sampler disabled** → Enable with `dry_multiplier: 0.8`

### Issue 2: Broken/Incoherent Responses

**Symptoms**: Nonsensical output, wrong context

**Causes & Fixes**:
1. **Wrong KV cache positions** → Use `nPast + i` for token positions
2. **EOG tokens not fed back** → Evaluate EOG token after generation

### Issue 3: Role Confusion / Wrong Persona

**Symptoms**: Model acts as user, wrong identity

**Causes & Fixes**:
1. **Wrong chat template** → Use model's native template format
2. **System prompt not embedded** → Include system in first turn template
3. **Plain text prompts** → Always use role markers

### Issue 4: Empty Responses

**Symptoms**: Model generates 0 tokens

**Causes & Fixes**:
1. **EOS/EOT banned incorrectly** → Don't ban EOG tokens
2. **Temperature too low** → Use at least 0.1
3. **Context overflow** → Check `nPast < contextLength`

---

## Model Profile System

### Profile Auto-Detection

Profiles are auto-detected based on filename patterns:

```swift
static func autoDetect(for filename: String) -> ModelProfile? {
    let patterns = [
        "lfm2-vl": "lfm2-vl-3b",
        "lfm2": "lfm2-2.6b",
        // Add more patterns as needed
    ]

    for (pattern, profileId) in patterns {
        if filename.lowercased().contains(pattern) {
            return load(id: profileId)
        }
    }
    return nil
}
```

### Profile File Location

Place JSON profiles in:
```
Resources/ModelProfiles/{profile-id}.json
```

### Adding New Model Support

1. Create a new JSON profile in `ModelProfiles/`
2. Add filename patterns to `modelPatterns` array
3. Configure appropriate sampling parameters
4. Test with multi-turn conversation

---

## Testing Checklist

Before deploying a new model configuration:

- [ ] First turn generates coherent greeting
- [ ] Multi-turn conversation maintains context
- [ ] No repetition in long responses
- [ ] Model uses correct identity/persona
- [ ] EOG tokens properly terminate generation
- [ ] Token count accumulates across turns
- [ ] Vision inputs work (if multimodal)

---

## Version History

| Date | Changes |
|------|---------|
| 2026-01-04 | Initial documentation. Fixed BOS token handling, chat templates, sampler chain for LFM2-VL 3B |

---

## References

- [llama.cpp Sampling Documentation](https://github.com/ggerganov/llama.cpp/blob/master/docs/sampling.md)
- [LiquidAI LFM2 Model Card](https://huggingface.co/LiquidAI/LFM2-VL-3B)
- [Forge SDK API Reference](../developer/api-reference.md)
