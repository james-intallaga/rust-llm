# Performance Tuning & Optimization

Running LLMs on mobile devices requires a careful balance between speed, memory usage, and output quality. This guide provides actionable tips for optimizing Forge SDK.

## 1. Quantization (The Most Important Factor)

Always use quantized models in `.gguf` format. Quantization reduces the precision of model weights (e.g., from 16-bit to 4-bit), which drastically reduces memory footprint and increases speed.

-   **Recommended**: `Q4_K_M` (Best balance of quality and speed).
-   **High Speed**: `Q2_K` or `IQ4_XS` (Lower quality, very fast).
-   **High Quality**: `Q8_0` (Close to original, but large and slow).

## 2. Metal Acceleration (GPU)

Ensure Metal is enabled. Forge SDK automatically uses the GPU on Apple Silicon devices.

```swift
var config = ForgeConfig.default
config.useGPU = true // Enabled by default
```

**Tip**: Metal is significantly faster for prompt processing (pre-fill) and long context generation.

## 3. Context Management

Inference speed slows down as the context fills up.

-   **Context Size**: Set `contextSize` only as high as you need. For most chat apps, `2048` or `4096` is sufficient.
-   **KV Cache Shifting**: Forge SDK automatically handles "KV Shift" when the context limit is reached, discarding the oldest parts of the conversation to make room for new tokens.
-   **Flash Attention**: Enable `useFlashAttention` for a 10-20% speed boost and lower memory usage during the evaluation phase.

## 4. Threading

By default, the SDK uses the number of physical CPU cores.

```swift
// Default behavior
config.threadCount = Int32(ProcessInfo.processInfo.processorCount)
```

**Optimization**: On some high-end iPhones, reducing the thread count by 1 or 2 can actually *increase* performance by reducing thermal throttling and leaving room for system tasks.

## 5. Performance Monitoring

You can monitor tokens-per-second to benchmark your configuration:

```swift
var startTime: Date?
var tokenCount = 0

forge.onToken = { token in
    if startTime == nil { startTime = Date() }
    tokenCount += 1
}

forge.onComplete = { _ in
    let elapsed = Date().timeIntervalSince(startTime!)
    print("Speed: \(Double(tokenCount) / elapsed) tokens/sec")
}
```

## 6. Cold Start vs. Warm Start

-   **mmap**: Enabling `useMMap` (default: `true`) allows the OS to load the model file lazily, making the initial `load()` call nearly instantaneous.
-   **mlock**: If you have plenty of RAM, setting `useMlock` can prevent the OS from swapping the model to disk, ensuring consistent performance.
