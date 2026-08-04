# Lesson 04: Context & Memory Management

Understanding how LLMs maintain state across tokens.

---

## What is Context?

The **context** is the window of tokens the model can "see" when generating. Everything outside this window is forgotten.

```
Context Window (4096 tokens)
┌────────────────────────────────────────────────────────────────┐
│ Token 1 | Token 2 | Token 3 | ... | Token 4095 | Token 4096   │
│ ◄───────────── Model can attend to all of these ────────────▶ │
└────────────────────────────────────────────────────────────────┘
                                                        │
                                                        ▼
                                              Token 4097 (new)
                                              attends to all above
```

---

## Context Length

| Model | Context Length |
|-------|---------------|
| LLaMA 2 | 4,096 |
| LLaMA 3 | 8,192 / 128K |
| LFM2 | 128,000 |
| GPT-4 | 8K / 32K / 128K |

### Setting Context Length

```swift
// In ForgeConfig
let config = ForgeConfig(
    contextLength: 4096,  // Tokens
    // ...
)
```

```c
// In llama.cpp
llama_context_params ctx_params = llama_context_default_params();
ctx_params.n_ctx = 4096;  // Context window size
```

### Memory vs Context Length

Larger context = more memory:

| Context | Layers | Heads | Dim | KV Cache Size |
|---------|--------|-------|-----|---------------|
| 2048 | 30 | 8 | 64 | ~32 MB |
| 4096 | 30 | 8 | 64 | ~64 MB |
| 8192 | 30 | 8 | 64 | ~128 MB |
| 32768 | 30 | 8 | 64 | ~512 MB |

---

## KV Cache

The **Key-Value Cache** stores attention states for previous tokens, avoiding recomputation.

### How Attention Works

```
For each new token, compute attention with ALL previous tokens:

Q (Query):  What am I looking for?
K (Key):    What do past tokens offer?
V (Value):  What information do past tokens have?

Attention = softmax(Q × K^T) × V
```

### Without KV Cache (Slow)

```
Token 1: Compute Q1, K1, V1
Token 2: Compute Q2, K2, V2, attend to K1, V1
Token 3: Compute Q3, K3, V3, attend to K1, V1, K2, V2
...
Token N: Recompute K, V for ALL previous tokens!
```

### With KV Cache (Fast)

```
Token 1: Compute Q1, K1, V1 → Store K1, V1
Token 2: Compute Q2, K2, V2 → Store K2, V2 → Attend using cached K1, V1
Token 3: Compute Q3, K3, V3 → Store K3, V3 → Attend using cached K1, K2, V1, V2
...
Token N: Only compute QN, KN, VN → Use cached K, V for all others
```

### KV Cache Structure

```
Per Layer:
┌─────────────────────────────────────────────────────────────┐
│                         KV CACHE                             │
├──────────────────────────┬──────────────────────────────────┤
│          K Cache         │           V Cache                 │
├──────────────────────────┼──────────────────────────────────┤
│  Pos 0: [head1][head2].. │  Pos 0: [head1][head2]...        │
│  Pos 1: [head1][head2].. │  Pos 1: [head1][head2]...        │
│  Pos 2: [head1][head2].. │  Pos 2: [head1][head2]...        │
│  ...                     │  ...                              │
│  Pos N: [head1][head2].. │  Pos N: [head1][head2]...        │
└──────────────────────────┴──────────────────────────────────┘
```

---

## Position Tracking (nPast)

`nPast` tracks how many tokens are in the context.

```swift
class LLMBase {
    var nPast: Int32 = 0  // Current position in context

    func llm_eval(inputBatch: [Int32]) throws -> Int32 {
        // Each token gets position: nPast, nPast+1, nPast+2, ...
        for i in 0..<inputBatch.count {
            batch.pos[i] = self.nPast + Int32(i)
        }

        // After eval, update nPast
        self.nPast += Int32(inputBatch.count)
    }
}
```

### ⚠️ Critical: Correct Positioning

```swift
// ✅ CORRECT: Each token at consecutive positions
batch.pos[0] = nPast + 0  // Token 0 at position nPast
batch.pos[1] = nPast + 1  // Token 1 at position nPast+1
batch.pos[2] = nPast + 2  // Token 2 at position nPast+2

// ❌ WRONG: All tokens at same position
batch.pos[0] = nPast  // Token 0 at position nPast
batch.pos[1] = nPast  // Token 1 at position nPast (WRONG!)
batch.pos[2] = nPast  // Token 2 at position nPast (WRONG!)
```

Wrong positioning causes:
- Overwritten KV cache entries
- Broken attention patterns
- Incoherent outputs

---

## Recurrent State (Hybrid Models)

Models like LFM2 have both attention AND recurrent layers.

### Transformer-Only (LLaMA)
```
Memory = KV Cache only
State is position-based
```

### Hybrid (LFM2)
```
Memory = KV Cache + Recurrent State
State is both position-based AND sequential
```

### Why BOS Matters for Recurrent Models

```
Recurrent state flows through tokens:

Token 1 (BOS) → State 1 → Token 2 → State 2 → Token 3 → State 3 → ...

If BOS appears mid-sequence:
Token 1 (BOS) → State 1 → Token 2 → Token 3 (BOS!) → STATE RESET → Broken!
```

**This is why BOS must only appear once at the start!**

---

## Context Management

### Tracking Context Usage

```swift
// Check context usage
let usage = Float(nPast) / Float(contextLength)
print("Context: \(nPast)/\(contextLength) (\(Int(usage * 100))%)")

// Example output:
// Context: 1024/4096 (25%)
```

### Context Full Handling

When context fills up, you have options:

1. **Truncate old context**
```swift
if nPast >= contextLength - 100 {
    // Keep only recent tokens
    llama_kv_cache_seq_rm(ctx, 0, 0, 1000)  // Remove first 1000
    nPast -= 1000
}
```

2. **Reset and summarize**
```swift
if nPast >= contextLength - 100 {
    // Summarize conversation, reset
    let summary = summarizeConversation()
    clearContext()
    evalTokens(systemPrompt + summary)
}
```

3. **Sliding window**
```swift
// Some models support sliding window attention
// Older tokens naturally "fall off"
```

---

## Flash Attention

Flash Attention is an optimized attention algorithm that:
- Uses less memory
- Runs faster
- Enables longer contexts

### Enabling Flash Attention

```swift
let config = ForgeConfig(
    flashAttention: true,  // Enable flash attention
    // ...
)
```

```c
llama_context_params ctx_params = llama_context_default_params();
ctx_params.flash_attn = true;
```

### Memory Comparison

| Context | Standard Attention | Flash Attention |
|---------|-------------------|-----------------|
| 4K | 64 MB | 32 MB |
| 8K | 256 MB | 64 MB |
| 32K | 4 GB | 256 MB |

---

## Batch Processing

### Prompt Evaluation

When processing a prompt, evaluate in batches:

```swift
func evaluatePrompt(_ tokens: [Int32]) {
    let batchSize = 512

    for i in stride(from: 0, to: tokens.count, by: batchSize) {
        let end = min(i + batchSize, tokens.count)
        let batch = Array(tokens[i..<end])

        try llm_eval(inputBatch: batch)
    }
}
```

### Why Batch?
- Memory efficiency
- GPU utilization
- Progress feedback

---

## Memory Layout Visualization

```
┌────────────────────────────────────────────────────────────────┐
│                         GPU MEMORY                              │
├────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   MODEL WEIGHTS                          │   │
│  │  Fixed size, loaded once                                 │   │
│  │  Example: 3B Q8 = ~3 GB                                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                     KV CACHE                             │   │
│  │  Grows with context                                      │   │
│  │  Size = n_ctx × n_layer × n_head × head_dim × 2 × dtype  │   │
│  │                                                          │   │
│  │  ┌──────────────────────────────────────────────────┐   │   │
│  │  │ Used: 0 ──────────────── nPast                   │   │   │
│  │  └──────────────────────────────────────────────────┘   │   │
│  │  ┌──────────────────────────────────────────────────┐   │   │
│  │  │ Available: nPast ─────────── n_ctx               │   │   │
│  │  └──────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                  COMPUTE BUFFERS                         │   │
│  │  Temporary, reused each forward pass                     │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└────────────────────────────────────────────────────────────────┘
```

---

## Resetting Context

### Full Reset

```swift
func clearContext() {
    nPast = 0
    llama_kv_cache_clear(ctx)
    resetSampler()
}
```

### Partial Reset (Keep System Prompt)

```swift
func resetKeepingSystem() {
    let systemTokens = systemPromptTokenCount

    // Remove everything after system prompt
    llama_kv_cache_seq_rm(ctx, 0, systemTokens, -1)
    nPast = systemTokens
    resetSampler()
}
```

---

## Exercises

### Exercise 1: Memory Calculation
Calculate KV cache size for:
- Context: 8192 tokens
- Layers: 32
- KV heads: 8
- Head dimension: 128
- Data type: f16 (2 bytes)

### Exercise 2: Position Tracking
Add logging to `llm_eval` in `LLMBase.swift`:
```swift
print("Eval: \(inputBatch.count) tokens at position \(nPast)")
```
Trace a multi-turn conversation.

### Exercise 3: Context Limits
What happens when you:
1. Fill context to 100%?
2. Try to add more tokens?

Test and observe the behavior.

---

## Key Takeaways

1. **Context** = window of tokens model can see
2. **KV Cache** stores attention states for speed
3. **nPast** tracks current position - must be correct!
4. **Recurrent models** are sensitive to BOS placement
5. **Flash Attention** reduces memory usage
6. **Batch processing** for efficient prompt evaluation

---

## Next Lesson

👉 **[Lesson 05: Sampling & Text Generation →](./05-sampling-generation.md)**
