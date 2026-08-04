# Recommended Models

These are practical GGUF starting points for the July 2026 Forge/llama.cpp baseline. Model compatibility changes quickly: keep the text model and its matching `mmproj` from the same publisher, and validate both on the target device before shipping.

## Compatibility levels

- **Forge-tested:** exercised through the Rust engine in this repository.
- **Upstream-supported:** the architecture is present in the pinned llama.cpp/mtmd release, but this repository's automated checks do not download its weights.

## Text Models

| Model | Size | Good For | Link |
|-------|------|----------|------|
| **Qwen 2.5 1.5B Instruct** | ~1.0GB Q4_K_M | Forge-tested baseline; broad device compatibility | [Qwen/Qwen2.5-1.5B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF) |
| **LFM2.5 1.2B Instruct** | Under 1GB at 4-bit | Fast CPU-oriented mobile text | [LiquidAI/LFM2.5-1.2B-Instruct-GGUF](https://huggingface.co/LiquidAI/LFM2.5-1.2B-Instruct-GGUF) |
| **Qwen 3.5 0.8B** | About 0.5GB at 4-bit | Small multilingual model; upstream-supported | [Qwen/Qwen3.5-0.8B](https://huggingface.co/Qwen/Qwen3.5-0.8B) |
| **Qwen 3 4B Instruct** | ~2.5GB Q4_K_M | Higher quality on devices with more memory | [Qwen/Qwen3-4B-Instruct-2507-GGUF](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507-GGUF) |

## Vision & Multimodal Models

| Model | Size | Good For | Link |
|-------|------|----------|------|
| **LFM2.5-VL 1.6B** | 696MB Q4_0 plus mmproj | Best current mobile-first starting point; upstream-supported | [LiquidAI/LFM2.5-VL-1.6B-GGUF](https://huggingface.co/LiquidAI/LFM2.5-VL-1.6B-GGUF) |
| **Qwen 2.5-VL 3B** | ~2GB at 4-bit plus mmproj | Stronger vision where memory allows; upstream-supported | [Qwen/Qwen2.5-VL-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-VL-3B-Instruct) |
| **DeepSeek OCR** | Varies by quantization | Document OCR; upstream mtmd architecture support | [NexaAI/DeepSeek-OCR-GGUF](https://huggingface.co/NexaAI/DeepSeek-OCR-GGUF) |

## Audio Models

| Model | Size | Good For | Link |
|-------|------|----------|------|
Audio input remains model-specific and is not part of the model-backed Forge test suite. Confirm that `supportsAudio`/`supports_audio()` is true after loading before presenting audio features.

---
© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
