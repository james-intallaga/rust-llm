# Lesson 07: Swift Wrapper Architecture

Understanding how Forge SDK bridges Swift and llama.cpp.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        SWIFT LAYER                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────┐   ┌────────────────┐   ┌────────────────┐   │
│  │   ForgeSDK     │   │      AI        │   │  ModelProfile  │   │
│  │  (High-level)  │   │ (Convenience)  │   │  (Config)      │   │
│  └───────┬────────┘   └───────┬────────┘   └───────┬────────┘   │
│          │                    │                    │             │
│          └────────────────────┼────────────────────┘             │
│                               │                                  │
│                    ┌──────────▼──────────┐                       │
│                    │      LLMBase        │                       │
│                    │  (Abstract Base)    │                       │
│                    └──────────┬──────────┘                       │
│                               │                                  │
│              ┌────────────────┼────────────────┐                 │
│              │                │                │                 │
│    ┌─────────▼────────┐  ┌────▼─────┐  ┌──────▼───────┐         │
│    │    LLaMa.swift   │  │  GPT.swift│  │LLaMa_MModal │         │
│    │  (llama.cpp)     │  │  (GPT-2)  │  │  (Vision)   │         │
│    └─────────┬────────┘  └──────────┘  └──────┬───────┘         │
│              │                                 │                 │
└──────────────┼─────────────────────────────────┼─────────────────┘
               │                                 │
┌──────────────▼─────────────────────────────────▼─────────────────┐
│                          C LAYER                                  │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐ │
│  │   llama.h       │   │   clip.h        │   │   mtmd.h        │ │
│  │   (LLM Core)    │   │   (Vision)      │   │   (Multimodal)  │ │
│  └─────────────────┘   └─────────────────┘   └─────────────────┘ │
│                                                                   │
│                    llama.cpp / ggml                               │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

---

## Swift-C Interoperability

### Importing C Libraries

```swift
// In LLaMa.swift
import llama      // llama.cpp headers
import llava      // Vision model headers
```

### Package.swift Configuration

```swift
// Forge/Package.swift
targets: [
    .target(
        name: "llama",
        path: "../llama.cpp",
        // C compilation settings
    ),
    .target(
        name: "Forge",
        dependencies: ["llama", "llava"],
        path: "Sources/Forge"
    )
]
```

### C Types in Swift

| C Type | Swift Type |
|--------|------------|
| `int32_t` | `Int32` |
| `float` | `Float` |
| `const char *` | `UnsafePointer<CChar>?` |
| `void *` | `UnsafeMutableRawPointer?` |
| `llama_model *` | `OpaquePointer?` |

---

## Class Hierarchy

### LLMBase (Abstract Base)

```swift
public class LLMBase {
    // Common state
    var model: OpaquePointer?        // llama_model *
    var context: OpaquePointer?      // llama_context *
    var sampling: OpaquePointer?     // llama_sampler *
    var batch: llama_batch?

    var nPast: Int32 = 0             // Position tracker
    var sampleParams: ModelSampleParams
    var system_prompt: String?

    // Abstract methods (override in subclass)
    func llm_load_model() throws
    func llm_eval(inputBatch: [Int32]) throws -> Int32
    func llm_sample() -> Int32

    // Common implementations
    func Predict(_ input: String, _ callback: (String, Double) -> Bool) throws -> String
    func LLMTokenize(_ input: String) -> [Int32]
    func LLMTokenToStr(token: Int32) -> String
}
```

### LLaMa (llama.cpp Implementation)

```swift
public class LLaMa: LLMBase {
    // Overrides abstract methods
    override func llm_load_model() throws {
        // Call llama_model_load_from_file()
    }

    override func llm_eval(inputBatch: [Int32]) throws -> Int32 {
        // Call llama_decode()
    }

    override func llm_sample() -> Int32 {
        // Call llama_sampler_sample()
    }

    // Additional methods
    func initSampler()
    func resetSampler()
}
```

### LLaMa_MModal (Vision Extension)

```swift
public class LLaMa_MModal: LLaMa {
    var clip_model: OpaquePointer?   // clip_ctx *
    var clip_ctx: OpaquePointer?     // mtmd_context *

    // Vision-specific methods
    func loadCLIP(path: String) throws
    func PredictVision(_ input: String,
                       _ image: Data,
                       _ callback: (String, Double) -> Bool) throws -> String
    func _eval_img(_ image: Data) throws
}
```

---

## Memory Management

### C Object Lifecycle

```swift
class LLaMa: LLMBase {
    deinit {
        // Free in reverse order of creation
        if let batch = batch {
            llama_batch_free(batch)
        }
        if let sampling = sampling {
            llama_sampler_free(sampling)
        }
        if let context = context {
            llama_free(context)
        }
        if let model = model {
            llama_model_free(model)
        }
    }
}
```

### Unsafe Pointer Handling

```swift
// Getting C string from Swift String
func tokenize(_ text: String) -> [Int32] {
    return text.withCString { cString in
        var tokens = [llama_token](repeating: 0, count: 4096)
        let count = llama_tokenize(
            vocab,
            cString,
            Int32(strlen(cString)),
            &tokens,
            Int32(tokens.count),
            true,
            true
        )
        return Array(tokens.prefix(Int(count)))
    }
}
```

### Buffer Management

```swift
// Working with llama_batch
func createBatch(tokens: [Int32]) {
    guard var batch = self.batch else { return }

    batch.n_tokens = Int32(tokens.count)

    for i in 0..<tokens.count {
        batch.token[i] = tokens[i]
        batch.pos[i] = nPast + Int32(i)
        batch.n_seq_id[i] = 1
        batch.seq_id[i]![0] = 0
        batch.logits[i] = (i == tokens.count - 1) ? 1 : 0
    }

    self.batch = batch
}
```

---

## Exception Handling

### C Exception Catching

llama.cpp can throw C++ exceptions. We catch them with a helper:

```swift
// ExceptionCatcher.h
@interface ExceptionCatcher : NSObject
+ (BOOL)catchException:(void(^)(void))tryBlock error:(NSError **)error;
@end
```

```swift
// Usage in Swift
public func llm_eval(inputBatch: [Int32]) throws -> Int32 {
    var exception: NSError?
    var result: Int32 = 0

    let success = ExceptionCatcher.catchException({
        result = llama_decode(self.context, batch)
    }, error: &exception)

    if !success, let error = exception {
        throw error
    }

    return result
}
```

---

## Async/Await Integration

### Background Loading

```swift
public func loadModel_sync(
    _ inference: ModelInference,
    contextParams: ModelAndContextParams,
    sampleParams: ModelSampleParams
) async throws {
    // Run on background thread
    try await Task.detached(priority: .userInitiated) {
        try self.llm_load_model()
        self.initSampler()
    }.value
}
```

### Streaming with Callbacks

```swift
// Callback-based streaming
func Predict(_ input: String,
             _ callback: @escaping (String, Double) -> Bool) throws -> String {
    var output = ""

    while generating {
        let token = llm_sample()

        if llama_token_is_eog(model, token) {
            break
        }

        let piece = LLMTokenToStr(token: token)
        output += piece

        // Callback on main thread for UI updates
        DispatchQueue.main.async {
            _ = callback(piece, 0.0)
        }
    }

    return output
}
```

---

## Configuration System

### ForgeConfig

```swift
public struct ForgeConfig {
    public var modelPath: String
    public var contextLength: Int32 = 4096
    public var batchSize: Int32 = 512
    public var temperature: Float = 0.3
    public var topP: Float = 0.95
    public var topK: Int32 = 40
    public var useMetal: Bool = true
    public var flashAttention: Bool = true

    // Convert to llama.cpp params
    func asContextParams() -> ModelAndContextParams {
        var params = ModelAndContextParams()
        params.context = contextLength
        params.n_batch = batchSize
        params.use_metal = useMetal
        params.flash_attn = flashAttention
        return params
    }

    func asSampleParams() -> ModelSampleParams {
        var params = ModelSampleParams.default
        params.temp = temperature
        params.top_p = topP
        params.top_k = topK
        return params
    }
}
```

### ModelProfile (External Config)

```swift
public struct ModelProfile: Codable {
    public var id: String
    public var name: String
    public var context: ContextConfig?
    public var sampling: SamplingConfig?

    // Load from JSON file
    public static func load(from url: URL) throws -> ModelProfile {
        let data = try Data(contentsOf: url)
        return try JSONDecoder().decode(ModelProfile.self, from: data)
    }

    // Auto-detect from model filename
    public static func autoDetect(for modelPath: String) -> ModelProfile? {
        let filename = URL(fileURLWithPath: modelPath)
            .lastPathComponent.lowercased()

        if filename.contains("lfm2-vl") {
            return loadBuiltIn(id: "lfm2-vl-3b")
        }
        // ... more patterns
    }
}
```

---

## Data Flow

### Complete Inference Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     USER APPLICATION                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. User calls: ai.predict("Hello")                             │
│                                                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                         AI.swift                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  2. Forwards to: llm.Predict("Hello", callback)                 │
│                                                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                       LLMBase.swift                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  3. Build prompt: lfm2ChatPrompt(system, "Hello")               │
│  4. Tokenize: LLMTokenize(prompt) → [token IDs]                 │
│                                                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                        LLaMa.swift                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  5. Evaluate: llm_eval([tokens]) → llama_decode()               │
│  6. Loop:                                                        │
│     a. Sample: llm_sample() → llama_sampler_sample()            │
│     b. Check EOG: llama_token_is_eog()                          │
│     c. Decode: LLMTokenToStr(token)                             │
│     d. Callback: callback(piece)                                │
│     e. Eval new token: llm_eval([new_token])                    │
│                                                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                         llama.cpp                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  7. GPU compute via Metal                                        │
│  8. Update KV cache                                              │
│  9. Return logits                                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Thread Safety

### Thread Model

```
Main Thread              Background Thread
     │                         │
     │  loadModel()            │
     │ ───────────────────────▶│ Loading...
     │                         │ llama_model_load()
     │                         │ llama_init_from_model()
     │ ◀─────────────────────── Ready
     │                         │
     │  predict()              │
     │ ───────────────────────▶│ Generating...
     │                         │ llama_decode()
     │ ◀── callback("Hi") ─────│ llama_sampler_sample()
     │ ◀── callback("!") ──────│
     │ ◀── callback("") ───────│ EOG
     │                         │
```

### Rules

1. **One context per thread** - Don't share `llama_context` across threads
2. **Model can be shared** - `llama_model` is thread-safe for reads
3. **Callbacks on main thread** - For UI updates

---

## Exercises

### Exercise 1: Class Exploration
Read `LLMBase.swift` and list:
1. All public methods
2. All instance variables
3. Which methods are meant to be overridden

### Exercise 2: Memory Lifecycle
Trace the lifecycle of a `LLaMa` instance:
1. What's allocated in `init`?
2. What's allocated in `llm_load_model`?
3. What's freed in `deinit`?

### Exercise 3: Add Logging
Add print statements to trace a complete inference:
```swift
print("1. Tokenizing: \(input)")
print("2. Token count: \(tokens.count)")
print("3. Evaluating batch...")
print("4. Sampling token: \(token)")
```

---

## Key Takeaways

1. **LLMBase** defines the interface, subclasses implement
2. **Swift-C bridging** uses unsafe pointers
3. **Memory must be manually managed** for C objects
4. **Exception catching** needed for C++ exceptions
5. **Async/await** for background loading
6. **Callbacks** for streaming responses

---

## Next Lesson

👉 **[Lesson 08: Forge SDK Deep Dive →](./08-forge-sdk-deep-dive.md)**
