# Forge SDK Tutorial Series

A comprehensive guide to understanding and mastering on-device LLM inference with the Forge SDK.

---

## Prerequisites

- Basic Swift programming knowledge
- Familiarity with iOS/macOS development
- Understanding of machine learning concepts (helpful but not required)

---

## Tutorial Structure

### Part 1: Foundations

| Lesson | Title | Duration | Topics |
|--------|-------|----------|--------|
| [01](./01-llama-cpp-fundamentals.md) | llama.cpp Fundamentals | 45 min | Architecture, core concepts, C API |
| [02](./02-gguf-models.md) | Understanding GGUF Models | 30 min | Model format, quantization, loading |
| [03](./03-tokenization.md) | Tokenization Deep Dive | 40 min | BPE, special tokens, vocabulary |

### Part 2: Core Mechanics

| Lesson | Title | Duration | Topics |
|--------|-------|----------|--------|
| [04](./04-context-memory.md) | Context & Memory Management | 50 min | KV cache, context windows, sliding |
| [05](./05-sampling-generation.md) | Sampling & Text Generation | 60 min | Sampler chain, temperature, penalties |
| [06](./06-chat-templates.md) | Chat Templates & Prompting | 35 min | Role markers, multi-turn, system prompts |

### Part 3: Swift Integration

| Lesson | Title | Duration | Topics |
|--------|-------|----------|--------|
| [07](./07-swift-wrapper-architecture.md) | Swift Wrapper Architecture | 55 min | Rust FFI, Swift bridging, memory safety |
| [08](./08-forge-sdk-deep-dive.md) | Forge SDK Deep Dive | 45 min | ForgeEngine, configuration, streaming |

### Part 4: Advanced Topics

| Lesson | Title | Duration | Topics |
|--------|-------|----------|--------|
| [09](./09-multimodal-vision.md) | Multimodal & Vision | 40 min | CLIP, image processing, audio |
| [10](./10-performance-optimization.md) | Performance Optimization | 50 min | Metal, batching, memory efficiency |

---

## Learning Path

```
Beginner           Intermediate              Advanced
    │                   │                        │
    ▼                   ▼                        ▼
┌───────┐          ┌───────┐               ┌───────┐
│ 01-03 │ ──────▶  │ 04-06 │ ──────────▶   │ 09-10 │
└───────┘          └───────┘               └───────┘
                        │
                        ▼
                   ┌───────┐
                   │ 07-08 │
                   └───────┘
```

---

## Key Concepts Overview

### The Inference Pipeline

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Input     │     │  Tokenize   │     │   Decode    │     │   Sample    │
│   Text      │ ──▶ │  to IDs     │ ──▶ │   (Eval)    │ ──▶ │   Token     │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                                              │                    │
                                              ▼                    ▼
                                        ┌─────────────┐     ┌─────────────┐
                                        │  KV Cache   │     │  Detokenize │
                                        │  Update     │     │  to Text    │
                                        └─────────────┘     └─────────────┘
```

### Architecture Layers

```
┌────────────────────────────────────────────────────────────┐
│                      Your Application                       │
├────────────────────────────────────────────────────────────┤
│                       ForgeSwift                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ ForgeEngine  │  │ForgeConfig   │  │ AudioDecoder │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├────────────────────────────────────────────────────────────┤
│                    ForgeRustCore (Rust)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  ForgeEngine │  │    Sampler   │  │ Multimodal   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├────────────────────────────────────────────────────────────┤
│                      llama.cpp                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ llama_model  │  │llama_context │  │llama_sampler │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
├────────────────────────────────────────────────────────────┤
│                    Hardware (Metal)                         │
└────────────────────────────────────────────────────────────┘
```

---

## Quick Reference

After completing the tutorials, use these for quick lookup:

- [API Reference](../developer/api-reference.md)
- [Configuration](../developer/configuration.md)
- [Model Settings](../developer/model-settings.md)

---

## Start Learning

👉 **[Begin with Lesson 01: llama.cpp Fundamentals →](./01-llama-cpp-fundamentals.md)**
