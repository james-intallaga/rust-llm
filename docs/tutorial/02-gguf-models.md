# Lesson 02: Understanding GGUF Models

Learn about the GGUF model format and quantization.

---

## What is GGUF?

**GGUF** (GPT-Generated Unified Format) is the file format used by llama.cpp to store model weights and metadata. It replaced the older GGML format.

### GGUF Structure

```
┌──────────────────────────────────────────────────────────┐
│                       GGUF FILE                          │
├──────────────────────────────────────────────────────────┤
│  HEADER                                                  │
│  ├── Magic number: "GGUF"                                │
│  ├── Version: 3                                          │
│  ├── Tensor count                                        │
│  └── Metadata count                                      │
├──────────────────────────────────────────────────────────┤
│  METADATA (Key-Value Pairs)                              │
│  ├── general.architecture: "llama"                       │
│  ├── general.name: "LFM2-VL-3B"                          │
│  ├── llama.context_length: 128000                        │
│  ├── llama.embedding_length: 2048                        │
│  ├── tokenizer.ggml.model: "gpt2"                        │
│  ├── tokenizer.ggml.tokens: [...]                        │
│  ├── tokenizer.ggml.bos_token_id: 1                      │
│  ├── tokenizer.chat_template: "..."                      │
│  └── ... more metadata ...                               │
├──────────────────────────────────────────────────────────┤
│  TENSOR DATA                                             │
│  ├── token_embd.weight                                   │
│  ├── blk.0.attn_q.weight                                 │
│  ├── blk.0.attn_k.weight                                 │
│  ├── blk.0.attn_v.weight                                 │
│  ├── blk.0.ffn_gate.weight                               │
│  └── ... all layer weights ...                           │
└──────────────────────────────────────────────────────────┘
```

---

## Quantization Types

Quantization reduces model size by using fewer bits per weight.

### Common Quantization Formats

| Type | Bits | Size Ratio | Quality | Speed |
|------|------|------------|---------|-------|
| F32 | 32 | 100% | Best | Slow |
| F16 | 16 | 50% | Excellent | Good |
| Q8_0 | 8 | 25% | Very Good | Fast |
| Q6_K | 6 | ~19% | Good | Fast |
| Q5_K_M | 5 | ~17% | Good | Fast |
| Q4_K_M | 4 | ~14% | Acceptable | Fastest |
| Q4_0 | 4 | ~12% | Lower | Fastest |
| Q2_K | 2 | ~10% | Poor | Fastest |

### Size Examples (3B Model)

| Quantization | Size |
|--------------|------|
| F16 | ~6 GB |
| Q8_0 | ~3 GB |
| Q4_K_M | ~1.8 GB |
| Q4_0 | ~1.5 GB |

### Choosing Quantization

```
Quality Priority          ◄─────────────────────►  Size Priority

   F16    Q8_0    Q6_K    Q5_K_M    Q4_K_M    Q4_0    Q2_K
    │       │       │        │         │        │       │
    ▼       ▼       ▼        ▼         ▼        ▼       ▼
  Best ─────────────────────────────────────────────► Smallest
```

**Recommendations:**
- **Q8_0**: Best quality-size balance for most uses
- **Q4_K_M**: Good for memory-constrained devices
- **F16**: Only for CLIP/vision projectors

---

## Reading Model Metadata

When loading a model, llama.cpp reads metadata:

```
llama_model_loader: loaded meta data with 34 key-value pairs
llama_model_loader: - kv   0: general.architecture str = lfm2
llama_model_loader: - kv   1: general.name str = LFM2 VL 3B
llama_model_loader: - kv  11: lfm2.context_length u32 = 128000
llama_model_loader: - kv  12: lfm2.embedding_length u32 = 2048
llama_model_loader: - kv  14: lfm2.attention.head_count u32 = 32
llama_model_loader: - kv  25: tokenizer.ggml.bos_token_id u32 = 1
llama_model_loader: - kv  26: tokenizer.ggml.eos_token_id u32 = 7
llama_model_loader: - kv  31: tokenizer.chat_template str = ...
```

### Key Metadata Fields

| Field | Description |
|-------|-------------|
| `general.architecture` | Model architecture (llama, lfm2, phi, etc.) |
| `general.name` | Human-readable name |
| `*.context_length` | Maximum context window |
| `*.embedding_length` | Hidden dimension size |
| `*.block_count` | Number of layers |
| `*.attention.head_count` | Number of attention heads |
| `tokenizer.ggml.bos_token_id` | BOS token ID |
| `tokenizer.ggml.eos_token_id` | EOS token ID |
| `tokenizer.chat_template` | Jinja2 chat template |

---

## Model Architectures

Different models have different architectures:

### Transformer-based
- **LLaMA**: Meta's original architecture
- **Phi**: Microsoft's compact models
- **Qwen**: Alibaba's models
- **Gemma**: Google's models

### Hybrid (Transformer + SSM)
- **LFM2**: LiquidAI's Liquid Foundation Models
- **Mamba**: State-space models
- **RWKV**: RNN-style models

### Architecture Differences

```
TRANSFORMER (LLaMA, Phi)           HYBRID (LFM2)
┌─────────────────────┐            ┌─────────────────────┐
│   Self-Attention    │            │ ShortConv (SSM)     │
│   + FFN             │  × N       │ or Self-Attention   │  × N
│   + LayerNorm       │            │ + FFN + LayerNorm   │
└─────────────────────┘            └─────────────────────┘
         │                                  │
    KV Cache                         KV Cache +
    (per layer)                      Recurrent State
```

**LFM2 Specifics:**
- Alternates between ShortConv and Attention layers
- Has recurrent state (like RNN) in addition to KV cache
- More sensitive to BOS token placement

---

## Loading Models in Code

### llama.cpp (C)

```c
llama_model_params params = llama_model_default_params();
params.n_gpu_layers = 99;  // All layers on GPU

llama_model * model = llama_model_load_from_file(
    "/path/to/model.gguf",
    params
);

if (!model) {
    // Handle error
}
```

### Forge SDK (Swift)

```swift
let modelPath = "/path/to/model.gguf"
let config = ForgeConfig(
    modelPath: modelPath,
    useMetal: true,
    useMMap: true
)

let ai = AI(_modelPath: modelPath, _chatName: "chat")
try await ai.loadModel_sync(
    ModelInference.LLama_gguf,
    contextParams: config.asContextParams(),
    sampleParams: ModelSampleParams.default
)
```

---

## Vision Models (Multimodal)

Multimodal models have two GGUF files:

### 1. Main Model (LLM)
- Contains language model weights
- Example: `LFM2-VL-3B-Q8_0.gguf`

### 2. Projector Model (CLIP)
- Contains vision encoder weights
- Maps images to text embedding space
- Example: `mmproj-LFM2-VL-3B-F16.gguf`

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Image     │     │    CLIP     │     │    LLM      │
│   Input     │ ──▶ │  Projector  │ ──▶ │   Model     │ ──▶ Text
└─────────────┘     │  (mmproj)   │     │   (main)    │
                    └─────────────┘     └─────────────┘
```

---

## Inspecting GGUF Files

### Using Python (gguf-py)

```bash
pip install gguf
python -m gguf.dump model.gguf
```

### Key Information to Extract

1. **Architecture**: What type of model is it?
2. **Context Length**: Maximum supported context
3. **Quantization**: What precision are the weights?
4. **Special Tokens**: BOS, EOS, EOT IDs
5. **Chat Template**: How to format prompts

---

## Model Sources

### Hugging Face
- Most models available at huggingface.co
- Look for `-GGUF` suffix in model names
- Choose quantization based on your device

### Converting Models

```bash
# Convert from HuggingFace format
python convert_hf_to_gguf.py \
    /path/to/hf/model \
    --outfile model-f16.gguf

# Quantize
./llama-quantize model-f16.gguf model-q8_0.gguf q8_0
```

---

## Exercises

### Exercise 1: Read Metadata
Run the example app and examine the log output. Find:
1. The model architecture
2. The context length
3. The BOS and EOS token IDs

### Exercise 2: Compare Sizes
Download the same model in Q8_0 and Q4_K_M. Compare:
1. File sizes
2. Memory usage during inference
3. Output quality

### Exercise 3: Explore Tokens
Using the model logs, find:
1. The chat template format
2. Special control tokens
3. The vocabulary size

---

## Key Takeaways

1. **GGUF** contains weights + metadata in one file
2. **Quantization** trades quality for size/speed
3. **Q8_0** is the sweet spot for most use cases
4. **Metadata** tells you how to use the model
5. **Vision models** need two GGUF files

---

## Next Lesson

👉 **[Lesson 03: Tokenization Deep Dive →](./03-tokenization.md)**
