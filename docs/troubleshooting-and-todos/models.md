# Tested & Recommended Models

This page lists models that have been tested and are recommended for use with Forge SDK.

## Text Models

| Model | Architecture | Size | Link |
|-------|--------------|------|------|
| **LFM2 2.6B Exp** | LFM | ~1.8GB | [Hugging Face](https://huggingface.co/LiquidAI/LFM2-2.6B-Exp-GGUF) |
| **HY-MT1.5 1.8B** | LLaMA | ~1.2GB | [Hugging Face](https://huggingface.co/tencent/HY-MT1.5-1.8B-GGUF) |
| **Qwen 3 4B Instruct** | Qwen2 | ~2.5GB | [Hugging Face](https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF) |
| **Llama 3.2 3B** | LLaMA | ~2.0GB | [Hugging Face](https://huggingface.co/meta-llama/Llama-3.2-3B-Instruct-GGUF) |

## Vision & Multimodal Models

| Model | Architecture | Size | Link |
|-------|--------------|------|------|
| **Qwen 2.5-VL 7B** | Qwen2-VL | ~5.0GB | [Hugging Face](https://huggingface.co/unsloth/Qwen2.5-VL-7B-Instruct-GGUF) |
| **LFM2-VL 3B** | LFM-VL | ~2.2GB | [Hugging Face](https://huggingface.co/LiquidAI/LFM2-VL-3B-GGUF) |
| **DeepSeek OCR** | DeepSeek-VL | ~1.5GB | [Hugging Face](https://huggingface.co/NexaAI/DeepSeek-OCR-GGUF) |

## Audio Models

| Model | Architecture | Size | Link |
|-------|--------------|------|------|
| **Fun-Audio-Chat 8B**| Fun-Audio | ~6.0GB | [Hugging Face](https://huggingface.co/FunAudioLLM/Fun-Audio-Chat-8B) |

## Quantization Recommendations

| Device | RAM | Recommended Quantization |
|--------|-----|--------------------------|
| iPhone 15/16 Pro | 8GB | Q4_K_M (up to 7B models) |
| iPhone 14/15 | 6GB | Q4_K_S (up to 3B models) |
| iPad Pro M1+ | 8-16GB | Q5_K_M or Q6_K |
| Mac M-Series | 8-32GB+ | Q5_K_M (7B) or Q4_K_M (14B+) |

For mobile devices, we highly recommend **LFM2** and **HY-MT** models for their exceptional speed-to-performance ratio.

---
© 2026 AMMA AI Intallaga Tech. Built on llama.cpp.
