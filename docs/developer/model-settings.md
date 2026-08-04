# Model-Specific Settings

Each model has different recommended settings for optimal performance. This document tracks the official recommendations from model providers.

## Quick Reference

| Model | Provider | Temperature | Top-P | Top-K | Context | Notes |
|-------|----------|-------------|-------|-------|---------|-------|
| LFM2.5-VL-1.6B | LiquidAI | 0.3 | 0.9 | 40 | 4096 | Vision, mobile optimized |
| LFM2.5-Audio-1.5B | LiquidAI | 0.3 | 0.9 | 40 | 4096 | Audio-to-audio |
| LFM2-2.6B | LiquidAI | 0.7 | 0.9 | 40 | 4096 | Text-only |
| Qwen2.5-VL | Alibaba | 0.7 | 0.8 | - | 8192 | Vision |
| Qwen2.5-3B | Alibaba | 0.7 | 0.9 | 50 | 32768 | Text-only |

---

## LFM2.5-VL-1.6B (LiquidAI)

**Vision model for image understanding**

### Recommended Settings
```swift
var config = ForgeConfiguration.vision()
config.temperature = 0.3     // Low for focused responses
config.topP = 0.9
config.topK = 40
```

### Quantization Options
| Quantization | Size | Quality | Speed | Recommendation |
|--------------|------|---------|-------|----------------|
| Q8_0 | ~1.6GB | Best | Good | ✅ Recommended |
| Q4_K_M | ~900MB | Good | Best | Mobile/low memory |

### Download URLs
- **Model (Q8_0)**: `https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/LFM2.5-VL-1.6B-Q8_0.gguf`
- **CLIP/mmproj**: `https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF/resolve/main/mmproj-LFM2.5-VL-1.6b-Q8_0.gguf`

---

## LFM2.5-Audio-1.5B (LiquidAI)

**End-to-end audio foundation model**

### Recommended Settings
```swift
var config = ForgeConfiguration.auto()
config.temperature = 0.3
config.topP = 0.9
config.topK = 40
```

### Download URLs
- **Model**: `https://huggingface.co/LiquidAI/LFM2.5-Audio-1.5B-GGUF`
- **Audio Encoder**: Check repository for `encoder` or `mmproj` file
- **Vocoder**: Check repository for `vocoder.gguf`

---

## LFM2-2.6B-Exp (LiquidAI)

**Text-only model for general conversation**

### Recommended Settings
```swift
var config = ForgeConfiguration.auto()
config.temperature = 0.7     // Standard for creative text
config.topP = 0.9
config.topK = 40
```

### Quantization Options
| Quantization | Size | Recommendation |
|--------------|------|----------------|
| Q4_K_M | ~1.6GB | ✅ Good balance |
| Q8_0 | ~2.8GB | Best quality |

### Download URL
- **Model (Q4_K_M)**: `https://huggingface.co/LiquidAI/LFM2-2.6B-Exp-GGUF/resolve/main/LFM2-2.6B-Exp-Q4_K_M.gguf`

---

## Qwen2.5-VL (Alibaba)

**High-quality vision-language model**

### Recommended Settings
```swift
var config = ForgeConfiguration.vision()
config.temperature = 0.7
config.topP = 0.8
config.contextSize = 8192  // Supports larger context
```

### Download URLs
- **7B Model**: `https://huggingface.co/Qwen/Qwen2.5-VL-7B-Instruct-GGUF`
- **2B Model**: `https://huggingface.co/Qwen/Qwen2.5-VL-2B-Instruct-GGUF`

---

## Qwen2.5-3B-Instruct (Alibaba)

**Efficient text model**

### Recommended Settings
```swift
var config = ForgeConfiguration.auto()
config.temperature = 0.7
config.topP = 0.9
config.topK = 50
```

### Download URL
- **Model (Q4_K_M)**: `https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF`

---

## Adding New Models

When adding a new model, research and document:

1. **Official temperature** - Check model card on Hugging Face
2. **Sampling parameters** - Look for provider examples
3. **Quantization trade-offs** - Test Q4 vs Q8 quality
4. **Context length** - Maximum supported context
5. **Prompt format** - Chat template (ChatML, LLaMA, etc.)

---

## References

- [LiquidAI Models](https://huggingface.co/LiquidAI)
- [Qwen Models](https://huggingface.co/Qwen)
- [llama.cpp Sampling](https://github.com/ggml-org/llama.cpp/wiki/Sampling)
