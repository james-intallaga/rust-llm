# Lesson 10: Performance Optimization

Maximizing inference speed and efficiency on Apple devices.

---

## Performance Metrics

### Key Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| **Tokens/second (TPS)** | Generation speed | 15-40+ |
| **Time to First Token (TTFT)** | Prompt processing | < 1-3s |
| **Memory Usage** | RAM/VRAM footprint | < 80% available |
| **Model Load Time** | Startup latency | < 5-10s |

### Measuring Performance

```swift
func measureTPS(_ tokens: [Int32]) -> Double {
    let start = CFAbsoluteTimeGetCurrent()

    var tokenCount = 0
    while generating {
        let token = llm_sample()
        tokenCount += 1

        if llama_token_is_eog(model, token) { break }
        _ = try? llm_eval(inputBatch: [token])
    }

    let elapsed = CFAbsoluteTimeGetCurrent() - start
    return Double(tokenCount) / elapsed
}
```

---

## Metal Acceleration

### Enabling Metal

```swift
let config = ForgeConfig(
    modelPath: path,
    useMetal: true,     // Use GPU
    gpuLayers: 99       // Offload all layers
)
```

```c
llama_model_params params = llama_model_default_params();
params.n_gpu_layers = 99;  // -1 or 99 for all layers
```

### Metal vs CPU

| Device | Metal TPS | CPU TPS | Improvement |
|--------|-----------|---------|-------------|
| M1 Mac | 35-50 | 5-10 | 5-7x |
| A16 iPhone | 20-30 | 3-5 | 6-8x |
| A15 iPhone | 15-25 | 3-5 | 5-6x |

### Metal Memory

```
GPU Memory Hierarchy:
┌─────────────────────────────────────────────┐
│  Unified Memory (Apple Silicon)             │
│  ┌─────────────────────────────────────┐   │
│  │  System RAM (shared)                 │   │
│  │  • Model weights                     │   │
│  │  • KV Cache                          │   │
│  │  • Compute buffers                   │   │
│  └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘

Discrete GPU:
┌─────────────────┐     ┌─────────────────┐
│   System RAM    │ ←→  │    VRAM         │
│   (CPU)         │     │   (GPU)         │
└─────────────────┘     └─────────────────┘
        PCIe transfer = slow!
```

Apple Silicon's unified memory eliminates transfer overhead.

---

## Flash Attention

### What is Flash Attention?

Optimized attention algorithm that:
- Reduces memory by not materializing full attention matrix
- Uses tiling for cache efficiency
- Enables longer contexts with less memory

### Enabling Flash Attention

```swift
let config = ForgeConfig(
    flashAttention: true,
    // ...
)
```

```c
llama_context_params ctx_params = llama_context_default_params();
ctx_params.flash_attn = true;
```

### Memory Savings

| Context | Standard | Flash | Savings |
|---------|----------|-------|---------|
| 4K | 64 MB | 32 MB | 50% |
| 8K | 256 MB | 64 MB | 75% |
| 32K | 4 GB | 256 MB | 94% |

---

## Batch Size Optimization

### Understanding Batches

```
Prompt: "Hello, how are you today?" (7 tokens)

Batch size = 512:
┌─────────────────────────────────────┐
│ Process all 7 tokens in 1 batch    │  ← 1 decode call
└─────────────────────────────────────┘

Batch size = 2:
┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
│ 2 tok │ │ 2 tok │ │ 2 tok │ │ 1 tok │  ← 4 decode calls
└───────┘ └───────┘ └───────┘ └───────┘
```

### Optimal Batch Size

```swift
// For prompt processing: larger is better
ctx_params.n_batch = 512

// Generation is always 1 token at a time (sequential)
```

| Batch Size | Prompt Speed | Memory | Recommended |
|------------|--------------|--------|-------------|
| 128 | Slow | Low | Memory-constrained |
| 256 | Medium | Medium | Balanced |
| 512 | Fast | Higher | Default |
| 1024 | Fastest | High | Lots of RAM |

---

## Quantization Trade-offs

### Speed vs Quality

| Quantization | Speed | Quality | Memory |
|--------------|-------|---------|--------|
| F16 | Baseline | Best | High |
| Q8_0 | 1.3x | Excellent | 50% |
| Q6_K | 1.5x | Very Good | 40% |
| Q4_K_M | 1.8x | Good | 35% |
| Q4_0 | 2.0x | Acceptable | 30% |

### Choosing Quantization

```
Device Memory        Recommended Quant
< 4 GB (iPhone)  →   Q4_K_M or Q4_0
4-8 GB           →   Q6_K or Q8_0
8+ GB            →   Q8_0 or F16
```

---

## Memory Management

### Estimating Memory

```swift
func estimateMemory(
    modelParams: Float,      // Billions
    quantization: String,    // "Q8_0", "Q4_K_M", etc
    contextLength: Int,
    layers: Int,
    kvHeads: Int,
    headDim: Int
) -> (model: Float, kvCache: Float) {

    // Model size (approximate)
    let bitsPerWeight: Float = switch quantization {
        case "F16": 16
        case "Q8_0": 8
        case "Q4_K_M": 4.5
        case "Q4_0": 4
        default: 8
    }

    let modelGB = modelParams * bitsPerWeight / 8

    // KV Cache (f16)
    let kvBytes = Float(contextLength * layers * kvHeads * headDim * 2 * 2)
    let kvGB = kvBytes / 1e9

    return (modelGB, kvGB)
}

// Example: LFM2-VL 3B Q8_0, 4K context, 30 layers, 8 heads, 64 dim
// Model: 3 * 8 / 8 = 3 GB
// KV: 4096 * 30 * 8 * 64 * 2 * 2 / 1e9 = 0.125 GB
// Total: ~3.1 GB
```

### Memory Optimization Tips

1. **Use smaller context when possible**
```swift
// Don't use 128K context if you only need 4K
contextParams.n_ctx = 4096
```

2. **Unload models when not in use**
```swift
func unloadModel() {
    llama_free(context)
    llama_model_free(model)
    context = nil
    model = nil
}
```

3. **Use memory mapping**
```swift
model_params.use_mmap = true  // Don't load entire model into RAM
```

---

## Optimizing Generation Loop

### Efficient Loop

```swift
func generateOptimized() throws -> String {
    var output = ""
    output.reserveCapacity(2000)  // Pre-allocate

    while true {
        let token = llm_sample()

        // Quick EOG check first
        if llama_token_is_eog(model, token) {
            break
        }

        // Decode to string
        let piece = LLMTokenToStr(token: token)
        output += piece

        // Eval next token
        _ = try llm_eval(inputBatch: [token])
    }

    return output
}
```

### Avoid in Hot Path

```swift
// ❌ BAD: Allocation in loop
while generating {
    let tokens = [Int32](repeating: 0, count: 1)  // Allocates every iteration
    tokens[0] = token
}

// ✅ GOOD: Reuse buffer
var tokenBuffer = [Int32](repeating: 0, count: 1)
while generating {
    tokenBuffer[0] = token
}
```

---

## Async Loading

### Background Model Loading

```swift
func loadModelAsync() async throws {
    // Show loading UI immediately
    await MainActor.run { isLoading = true }

    // Load on background thread
    try await Task.detached(priority: .userInitiated) {
        try self.llm_load_model()
        self.initSampler()
    }.value

    // Update UI
    await MainActor.run {
        isLoading = false
        isReady = true
    }
}
```

### Progress Updates

```swift
func loadWithProgress(progress: @escaping (Float) -> Void) async throws {
    // llama.cpp doesn't provide progress, estimate by steps
    progress(0.1)  // Started

    try await Task.detached {
        self.model = llama_model_load_from_file(self.modelPath, params)
    }.value
    progress(0.6)  // Model loaded

    try await Task.detached {
        self.context = llama_init_from_model(self.model, ctx_params)
    }.value
    progress(0.9)  // Context created

    initSampler()
    progress(1.0)  // Done
}
```

---

## Device-Specific Optimization

### iPhone Optimization

```swift
// Constrained memory
let config = ForgeConfig(
    contextLength: 2048,    // Smaller context
    batchSize: 256,         // Smaller batches
    flashAttention: true,   // Save memory
    gpuLayers: 99           // Full GPU
)

// Use Q4_K_M quantization for smaller models
```

### iPad Optimization

```swift
// More memory available
let config = ForgeConfig(
    contextLength: 4096,
    batchSize: 512,
    flashAttention: true,
    gpuLayers: 99
)

// Q8_0 or Q6_K viable
```

### Mac Optimization

```swift
// Lots of memory
let config = ForgeConfig(
    contextLength: 8192,
    batchSize: 512,
    flashAttention: true,
    gpuLayers: 99
)

// Q8_0 or even F16 for small models
```

---

## Profiling

### Basic Timing

```swift
func profile<T>(_ name: String, _ block: () throws -> T) rethrows -> T {
    let start = CFAbsoluteTimeGetCurrent()
    let result = try block()
    let elapsed = (CFAbsoluteTimeGetCurrent() - start) * 1000
    print("\(name): \(String(format: "%.2f", elapsed)) ms")
    return result
}

// Usage
let tokens = profile("Tokenize") {
    LLMTokenize(prompt)
}

try profile("Eval") {
    try llm_eval(inputBatch: tokens)
}
```

### Instruments Profiling

1. Open Instruments.app
2. Select "Metal System Trace" or "Time Profiler"
3. Run your app
4. Look for:
   - GPU utilization
   - Memory allocations
   - Hot functions

---

## Benchmark Reference

### Expected Performance (Q8_0)

| Model Size | iPhone 15 | M1 Mac | M3 Pro |
|------------|-----------|--------|--------|
| 1B | 35-45 TPS | 60-80 TPS | 80-100 TPS |
| 3B | 15-25 TPS | 35-50 TPS | 50-70 TPS |
| 7B | 8-12 TPS | 20-30 TPS | 35-45 TPS |
| 13B | 3-6 TPS | 12-18 TPS | 20-30 TPS |

---

## Exercises

### Exercise 1: Benchmark
Create a benchmark that measures:
1. Model load time
2. TTFT for a 100-token prompt
3. TPS for 200-token generation

### Exercise 2: Memory Profiling
Monitor memory usage during:
1. Model loading
2. Long conversation (fill context)
3. After clearing context

### Exercise 3: Optimization Comparison
Compare performance with:
1. Flash attention on vs off
2. Different batch sizes
3. Different context lengths

---

## Key Takeaways

1. **Enable Metal** for 5-8x speedup
2. **Use Flash Attention** for memory efficiency
3. **Choose quantization** based on device memory
4. **Optimize batch size** for prompt processing
5. **Load models async** for responsive UI
6. **Profile** to find bottlenecks

---

## Next Lesson

👉 **[Lesson 11: Troubleshooting & Debugging →](../troubleshooting-and-todos/11-troubleshooting-debugging.md)**
