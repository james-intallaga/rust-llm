# Lesson 01: llama.cpp Fundamentals

Understanding the core inference engine that powers the Forge SDK.

---

## What is llama.cpp?

**llama.cpp** is a C/C++ library for running Large Language Models (LLMs) efficiently on consumer hardware. It was created by Georgi Gerganov and has become the de-facto standard for on-device LLM inference.

### Key Features

- **Pure C/C++**: No Python dependencies, runs anywhere
- **Quantization**: Run large models in reduced precision (4-bit, 8-bit)
- **Hardware Acceleration**: Metal (Apple), CUDA (NVIDIA), Vulkan, OpenCL
- **Cross-Platform**: iOS, macOS, Android, Windows, Linux
- **Memory Efficient**: Designed for consumer devices

---

## Core Concepts

### 1. The Model (`llama_model`)

The model contains the neural network weights and architecture information.

```c
// Loading a model
llama_model_params model_params = llama_model_default_params();
model_params.n_gpu_layers = 99;  // Offload all layers to GPU

llama_model * model = llama_model_load_from_file("model.gguf", model_params);
```

**Key Parameters:**
| Parameter | Description |
|-----------|-------------|
| `n_gpu_layers` | Number of layers to run on GPU (99 = all) |
| `use_mmap` | Memory-map the model file |
| `use_mlock` | Lock model in RAM (prevent swapping) |

### 2. The Context (`llama_context`)

The context manages the inference state, including the KV cache.

```c
// Creating a context
llama_context_params ctx_params = llama_context_default_params();
ctx_params.n_ctx = 4096;      // Context window size
ctx_params.n_batch = 512;     // Batch size for prompt processing
ctx_params.flash_attn = true; // Use flash attention

llama_context * ctx = llama_init_from_model(model, ctx_params);
```

**Key Parameters:**
| Parameter | Description |
|-----------|-------------|
| `n_ctx` | Maximum context length (tokens) |
| `n_batch` | Tokens processed per batch |
| `n_ubatch` | Micro-batch size |
| `flash_attn` | Enable flash attention (faster, less memory) |

### 3. The Batch (`llama_batch`)

Batches are used to submit tokens for processing.

```c
// Create a batch
llama_batch batch = llama_batch_init(512, 0, 1);

// Add tokens to the batch
for (int i = 0; i < n_tokens; i++) {
    batch.token[i]    = tokens[i];     // Token ID
    batch.pos[i]      = past_tokens + i; // Position in context
    batch.seq_id[i]   = 0;             // Sequence ID
    batch.logits[i]   = (i == n_tokens - 1); // Only compute logits for last token
}
batch.n_tokens = n_tokens;

// Process the batch (decode)
llama_decode(ctx, batch);
```

### 4. The Vocabulary (`llama_vocab`)

The vocabulary maps between text and token IDs.

```c
llama_vocab * vocab = llama_model_get_vocab(model);

// Tokenize text to IDs
llama_token tokens[256];
int n_tokens = llama_tokenize(vocab, text, strlen(text), tokens, 256, true, false);

// Detokenize ID to text
char piece[64];
int n_chars = llama_token_to_piece(vocab, token_id, piece, 64, 0, true);
```

### 5. The Sampler (`llama_sampler`)

Samplers select the next token from the probability distribution.

```c
// Create a sampler chain
llama_sampler_chain_params sparams = llama_sampler_chain_default_params();
llama_sampler * smpl = llama_sampler_chain_init(sparams);

// Add samplers to the chain
llama_sampler_chain_add(smpl, llama_sampler_init_temp(0.7));
llama_sampler_chain_add(smpl, llama_sampler_init_top_p(0.95, 1));
llama_sampler_chain_add(smpl, llama_sampler_init_dist(seed));

// Sample a token
llama_token new_token = llama_sampler_sample(smpl, ctx, -1);
```

---

## The Inference Loop

The core inference loop follows this pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                    INFERENCE LOOP                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Tokenize input text                                      │
│         │                                                    │
│         ▼                                                    │
│  2. Create batch with tokens                                 │
│         │                                                    │
│         ▼                                                    │
│  3. Decode (forward pass)  ◄───────────────────────┐        │
│         │                                          │        │
│         ▼                                          │        │
│  4. Get logits from last token                     │        │
│         │                                          │        │
│         ▼                                          │        │
│  5. Sample next token                              │        │
│         │                                          │        │
│         ▼                                          │        │
│  6. Check if EOG (end of generation)               │        │
│         │                                          │        │
│         ├─── No ───────────────────────────────────┘        │
│         │                                                    │
│         ▼ Yes                                                │
│  7. Detokenize and return                                    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Pseudocode Implementation

```c
// 1. Tokenize
int n_tokens = llama_tokenize(vocab, prompt, tokens, max_tokens, add_bos, false);

// 2. Evaluate prompt
llama_batch batch = llama_batch_init(n_tokens, 0, 1);
for (int i = 0; i < n_tokens; i++) {
    batch.token[i] = tokens[i];
    batch.pos[i] = i;
    batch.logits[i] = (i == n_tokens - 1);
}
batch.n_tokens = n_tokens;
llama_decode(ctx, batch);

int n_past = n_tokens;

// 3. Generation loop
while (n_past < max_context) {
    // Sample next token
    llama_token new_token = llama_sampler_sample(smpl, ctx, -1);

    // Accept token (for penalty tracking)
    llama_sampler_accept(smpl, new_token);

    // Check for end of generation
    if (llama_token_is_eog(model, new_token)) {
        break;
    }

    // Output token
    char piece[64];
    llama_token_to_piece(vocab, new_token, piece, 64, 0, true);
    printf("%s", piece);

    // Prepare next batch (single token)
    batch.token[0] = new_token;
    batch.pos[0] = n_past;
    batch.logits[0] = true;
    batch.n_tokens = 1;

    llama_decode(ctx, batch);
    n_past++;
}
```

---

## Memory Layout

Understanding how llama.cpp uses memory:

```
┌────────────────────────────────────────────────────────────────┐
│                         RAM / VRAM                              │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   MODEL WEIGHTS                          │   │
│  │  • Token embeddings                                      │   │
│  │  • Attention weights (Q, K, V, O per layer)              │   │
│  │  • FFN weights (gate, up, down per layer)                │   │
│  │  • Layer norms                                           │   │
│  │  • Output head                                           │   │
│  │                                                          │   │
│  │  Size: ~1-4 bytes per parameter (quantization)           │   │
│  │  Example: 3B model Q8 ≈ 3GB                               │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     KV CACHE                             │   │
│  │  • Stores key/value vectors for each position           │   │
│  │  • Size = n_ctx × n_layer × n_head × head_dim × 2        │   │
│  │  • Grows with context length                             │   │
│  │                                                          │   │
│  │  Example: 4096 ctx, 30 layers, 8 heads, 64 dim           │   │
│  │           = 4096 × 30 × 8 × 64 × 2 × 2 bytes ≈ 125MB     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  COMPUTE BUFFERS                         │   │
│  │  • Temporary tensors for computation                     │   │
│  │  • Activation storage                                    │   │
│  │  • Grows with batch size                                 │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

## Special Token Types

| Token Type | ID Function | Purpose |
|------------|-------------|---------|
| BOS | `llama_token_bos()` | Beginning of sequence |
| EOS | `llama_token_eos()` | End of sequence |
| EOT | `llama_token_eot()` | End of turn |
| PAD | `llama_token_pad()` | Padding |
| EOG | `llama_token_is_eog()` | Any end-of-generation token |

```c
// Get special tokens
llama_token bos = llama_token_bos(vocab);
llama_token eos = llama_token_eos(vocab);

// Check if token ends generation
if (llama_token_is_eog(model, token)) {
    // Stop generating
}
```

---

## Thread Safety

llama.cpp has specific thread safety rules:

| Object | Thread Safety |
|--------|---------------|
| `llama_model` | **Thread-safe** for reads, can share across contexts |
| `llama_context` | **NOT thread-safe**, one thread per context |
| `llama_sampler` | **NOT thread-safe**, one thread per sampler |
| `llama_batch` | **NOT thread-safe**, one thread per batch |

**Best Practice**: Create separate contexts for each concurrent inference stream.

---

## Error Handling

llama.cpp uses return codes:

```c
// Model loading
llama_model * model = llama_model_load_from_file(path, params);
if (model == NULL) {
    fprintf(stderr, "Failed to load model\n");
    return 1;
}

// Decoding
int result = llama_decode(ctx, batch);
if (result != 0) {
    if (result == 1) {
        fprintf(stderr, "Could not find KV slot (context full)\n");
    } else {
        fprintf(stderr, "Decode failed: %d\n", result);
    }
}
```

---

## Resource Cleanup

Always free resources in reverse order of creation:

```c
// Create
llama_model * model = llama_model_load_from_file(...);
llama_context * ctx = llama_init_from_model(model, ...);
llama_sampler * smpl = llama_sampler_chain_init(...);
llama_batch batch = llama_batch_init(...);

// ... use ...

// Free (reverse order)
llama_batch_free(batch);
llama_sampler_free(smpl);
llama_free(ctx);
llama_model_free(model);
```

---

## Exercises

### Exercise 1: Trace the Flow
Read the `Forge/Sources/Forge/LLaMa.swift` file and identify:
1. Where is `llama_model_load_from_file` called?
2. Where is `llama_decode` called?
3. Where is `llama_sampler_sample` called?

### Exercise 2: Memory Calculation
Calculate the approximate memory needed for:
- Model: 3B parameters, Q8_0 quantization
- Context: 8192 tokens
- KV Cache: 30 layers, 8 KV heads, 64 head dimension, f16

### Exercise 3: API Exploration
Open `llama.xcframework/.../Headers/llama.h` and find:
1. All functions that start with `llama_token_`
2. All `llama_sampler_init_*` functions
3. The `llama_context_params` struct

---

## Key Takeaways

1. **Model** holds weights, **Context** holds state
2. **Batches** submit tokens for processing
3. **Samplers** are chained together to select tokens
4. Always handle **EOG tokens** to know when to stop
5. Manage memory carefully - models are large

---

## Next Lesson

👉 **[Lesson 02: Understanding GGUF Models →](./02-gguf-models.md)**
