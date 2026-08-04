# Lesson 11: Troubleshooting & Debugging

Diagnosing and fixing common issues with LLM inference.

---

## Diagnostic Checklist

When something goes wrong, check these in order:

```
┌─────────────────────────────────────────────────────────────┐
│                    DIAGNOSTIC FLOW                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. Does the model load?                                     │
│     └─ No → Check file path, format, memory                 │
│                                                              │
│  2. Does tokenization work?                                  │
│     └─ No → Check vocabulary, special tokens                │
│                                                              │
│  3. Does evaluation run without crash?                       │
│     └─ No → Check context size, batch size, memory          │
│                                                              │
│  4. Is output generated?                                     │
│     └─ No → Check sampler, temperature, EOG handling        │
│                                                              │
│  5. Is output quality good?                                  │
│     └─ No → Check chat template, sampling params, BOS       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Common Issues

### Issue 1: Model Fails to Load

**Symptoms:**
- `Failed to load model`
- Crash on load
- Null model pointer

**Diagnosis:**

```swift
func diagnoseModelLoad(path: String) {
    // Check file exists
    let exists = FileManager.default.fileExists(atPath: path)
    print("File exists: \(exists)")

    // Check file size
    if let attrs = try? FileManager.default.attributesOfItem(atPath: path) {
        let size = attrs[.size] as? Int64 ?? 0
        print("File size: \(size / 1_000_000) MB")
    }

    // Check file is readable
    let readable = FileManager.default.isReadableFile(atPath: path)
    print("File readable: \(readable)")

    // Check available memory
    print("Available memory: \(ProcessInfo.processInfo.physicalMemory / 1_000_000_000) GB")
}
```

**Solutions:**

| Cause | Solution |
|-------|----------|
| Wrong path | Verify absolute path |
| File corrupted | Re-download model |
| Insufficient memory | Use smaller quantization |
| Wrong format | Ensure GGUF format |
| Old GGUF version | Update llama.cpp |

---

### Issue 2: Repetition

**Symptoms:**
- "I am well I am well I am well..."
- Same phrase repeated endlessly
- Model stuck in loop

**Root Causes:**

```
┌─────────────────────────────────────────────────────────────┐
│                    REPETITION CAUSES                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. BOS Token Corruption                                     │
│     └─ BOS added on every turn → Breaks recurrent state     │
│                                                              │
│  2. Sampler Not Accepting Tokens                            │
│     └─ llama_sampler_accept not called → No penalty         │
│                                                              │
│  3. Repeat Penalty Too Low                                   │
│     └─ 1.0 or 1.05 → Not enough suppression                 │
│                                                              │
│  4. DRY Sampler Disabled                                     │
│     └─ No phrase-level repetition prevention                 │
│                                                              │
│  5. Wrong KV Cache Position                                  │
│     └─ Tokens overwriting each other → Attention broken      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Diagnosis:**

```swift
func diagnoseRepetition() {
    print("1. BOS handling:")
    print("   nPast before tokenize: \(nPast)")
    print("   Should add BOS: \(nPast == 0)")

    print("2. Sampler accept called: \(samplerAcceptCalled)")

    print("3. Repeat penalty: \(sampleParams.repeat_penalty)")
    print("   Expected: >= 1.15")

    print("4. DRY multiplier: \(sampleParams.dry_multiplier)")
    print("   Expected: >= 0.8")

    print("5. Token positions in last batch:")
    for i in 0..<batch.n_tokens {
        print("   Token \(i): pos \(batch.pos[i])")
    }
}
```

**Fixes:**

```swift
// Fix 1: BOS only on first turn
let shouldAddBos = (self.nPast == 0)

// Fix 2: Call accept after eval
if let smpl = self.sampling {
    llama_sampler_accept(smpl, outputToken)
}

// Fix 3: Increase repeat penalty
sampleParams.repeat_penalty = 1.15

// Fix 4: Enable DRY
sampleParams.dry_multiplier = 0.8

// Fix 5: Correct positioning
batch.pos[i] = self.nPast + Int32(i)
```

---

### Issue 3: Gibberish / Nonsense Output

**Symptoms:**
- Random characters
- Incomplete words
- No coherent meaning

**Causes:**

| Cause | Check |
|-------|-------|
| Wrong tokenization | Are tokens valid? |
| Temperature too high | Is temp > 1.5? |
| Corrupted model | Was download complete? |
| Wrong model format | Is it a chat model? |
| KV cache corruption | Is nPast correct? |

**Diagnosis:**

```swift
func diagnoseGibberish() {
    // Check tokenization
    let text = "Hello, world!"
    let tokens = LLMTokenize(text)
    print("Input: \(text)")
    print("Tokens: \(tokens)")

    // Decode tokens back
    for token in tokens {
        let piece = LLMTokenToStr(token: token)
        print("  \(token) → '\(piece)'")
    }

    // Check temperature
    print("Temperature: \(sampleParams.temp)")

    // Check logits
    let logits = llama_get_logits(context)
    print("Sample logits: \(logits?[0...5])")
}
```

---

### Issue 4: "Stupid" / Wrong Answers

**Symptoms:**
- Model doesn't follow instructions
- Acts as wrong role
- Ignores context
- Hallucinates

**Root Cause: Wrong Prompt Format**

```
❌ Wrong:
"You are an assistant. User says hello."

✅ Correct:
"<|startoftext|><|im_start|>system
You are an assistant.
<|im_end|><|im_start|>user
Hello.
<|im_end|><|im_start|>assistant"
```

**Diagnosis:**

```swift
func diagnosePromptFormat() {
    let prompt = buildPrompt(system: systemPrompt, user: userInput)

    print("=== Full Prompt ===")
    print(prompt)
    print("===================")

    // Check for required markers
    let hasStartOfText = prompt.contains("<|startoftext|>")
    let hasImStart = prompt.contains("<|im_start|>")
    let hasImEnd = prompt.contains("<|im_end|>")
    let hasSystemRole = prompt.contains("<|im_start|>system")
    let hasUserRole = prompt.contains("<|im_start|>user")
    let hasAssistantRole = prompt.contains("<|im_start|>assistant")

    print("Has <|startoftext|>: \(hasStartOfText)")
    print("Has <|im_start|>: \(hasImStart)")
    print("Has <|im_end|>: \(hasImEnd)")
    print("Has system role: \(hasSystemRole)")
    print("Has user role: \(hasUserRole)")
    print("Has assistant role: \(hasAssistantRole)")

    // Check role order
    let systemPos = prompt.range(of: "system")?.lowerBound
    let userPos = prompt.range(of: "user")?.lowerBound
    let assistantPos = prompt.range(of: "assistant")?.lowerBound

    if let s = systemPos, let u = userPos, let a = assistantPos {
        let correctOrder = (s < u) && (u < a)
        print("Correct role order: \(correctOrder)")
    }
}
```

---

### Issue 5: Crashes

**Symptoms:**
- EXC_BAD_ACCESS
- SIGABRT
- Memory errors

**Common Crash Causes:**

| Crash | Cause | Fix |
|-------|-------|-----|
| EXC_BAD_ACCESS in decode | Null context | Check load succeeded |
| EXC_BAD_ACCESS in sample | Null sampler | Initialize sampler |
| Out of memory | Model too large | Use smaller quant |
| Batch overflow | Too many tokens | Reduce batch size |

**Safe Wrappers:**

```swift
func safeEval(_ tokens: [Int32]) throws {
    guard let ctx = context else {
        throw LLMError.contextNotInitialized
    }

    guard tokens.count <= Int(contextParams.n_batch) else {
        throw LLMError.batchTooLarge
    }

    // Use exception catcher for C++ exceptions
    var cppException: NSError?
    ExceptionCatcher.catchException({
        llama_decode(ctx, self.batch)
    }, error: &cppException)

    if let error = cppException {
        throw error
    }
}
```

---

### Issue 6: Generation Doesn't Stop

**Symptoms:**
- Model generates forever
- Never hits EOG
- Output keeps growing

**Causes:**
- EOG check missing
- Wrong EOG token IDs
- Model not trained to stop

**Fix:**

```swift
func generateWithLimits(maxTokens: Int = 2048) throws -> String {
    var output = ""
    var tokenCount = 0

    while tokenCount < maxTokens {
        let token = llm_sample()

        // Check ALL EOG conditions
        if llama_token_is_eog(model, token) {
            print("Stopped: EOG token \(token)")
            break
        }

        if token == llama_token_eos(vocab) {
            print("Stopped: EOS token")
            break
        }

        output += LLMTokenToStr(token: token)
        tokenCount += 1

        _ = try llm_eval(inputBatch: [token])
    }

    if tokenCount >= maxTokens {
        print("Stopped: Max tokens reached")
    }

    return output
}
```

---

## Debugging Tools

### Logging

```swift
class LLMLogger {
    static var enabled = true

    static func log(_ message: String, level: Level = .info) {
        guard enabled else { return }
        let timestamp = Date().formatted(date: .omitted, time: .standard)
        print("[\(timestamp)] [\(level)] \(message)")
    }

    enum Level: String {
        case debug = "DEBUG"
        case info = "INFO"
        case warning = "WARN"
        case error = "ERROR"
    }
}

// Usage
LLMLogger.log("Loading model: \(path)")
LLMLogger.log("Context created: \(contextLength) tokens")
LLMLogger.log("Sampling token at nPast: \(nPast)")
```

### Token Dumping

```swift
func dumpTokens(_ tokens: [Int32]) {
    print("=== Token Dump (\(tokens.count) tokens) ===")
    for (i, token) in tokens.enumerated() {
        let piece = LLMTokenToStr(token: token)
        let escaped = piece.replacingOccurrences(of: "\n", with: "\\n")
        print("  [\(i)] \(token) → '\(escaped)'")
    }
    print("==========================================")
}
```

### State Inspection

```swift
func inspectState() {
    print("=== LLM State ===")
    print("Model loaded: \(model != nil)")
    print("Context created: \(context != nil)")
    print("Sampler initialized: \(sampling != nil)")
    print("nPast: \(nPast)")
    print("Context size: \(contextParams.context)")
    print("Context usage: \(Float(nPast) / Float(contextParams.context) * 100)%")
    print("================")
}
```

---

## Model-Specific Issues

### LFM2 Models

| Issue | Cause | Fix |
|-------|-------|-----|
| Endless repetition | BOS on every turn | Check `nPast == 0` |
| Wrong persona | No chat template | Use `lfm2ChatPrompt` |
| No vision | Missing mmproj | Load CLIP model |

### LLaMA Models

| Issue | Cause | Fix |
|-------|-------|-----|
| Cut-off responses | Wrong EOS | Check token 2 and 128001 |
| Format errors | Wrong template | Use LLaMA 3 format |

### Phi Models

| Issue | Cause | Fix |
|-------|-------|-----|
| Brief responses | System prompt ignored | Use Phi template |
| Repetition | Temperature too high | Lower to 0.3 |

---

## Performance Issues

### Slow Generation

```swift
func diagnoseSlowGeneration() {
    print("=== Performance Check ===")
    print("Metal enabled: \(contextParams.use_metal)")
    print("GPU layers: \(contextParams.n_gpu_layers)")
    print("Flash attention: \(contextParams.flash_attn)")
    print("Batch size: \(contextParams.n_batch)")
    print("Context length: \(contextParams.context)")

    // Check if Metal is actually being used
    // Look for "load_tensors: offloading X layers to GPU" in logs
}
```

### Memory Issues

```swift
func diagnoseMemory() {
    let info = ProcessInfo.processInfo
    print("=== Memory Check ===")
    print("Physical memory: \(info.physicalMemory / 1_000_000_000) GB")

    var taskInfo = task_vm_info_data_t()
    var count = mach_msg_type_number_t(MemoryLayout<task_vm_info>.size) / 4

    if task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO),
                 UnsafeMutableRawPointer(&taskInfo).assumingMemoryBound(to: integer_t.self),
                 &count) == KERN_SUCCESS {
        let used = taskInfo.phys_footprint / 1_000_000
        print("Memory used: \(used) MB")
    }
}
```

---

## Quick Reference: Error Messages

| Error | Meaning | Fix |
|-------|---------|-----|
| `Failed to load model` | Model file issue | Check path, format |
| `Could not find KV slot` | Context full | Clear context or expand |
| `Decode failed` | Evaluation error | Check batch, context |
| `Segmentation fault` | Memory corruption | Check pointers, bounds |
| `Out of memory` | Insufficient RAM | Smaller model/context |

---

## Exercises

### Exercise 1: Add Diagnostics
Add logging to trace:
1. Every tokenization
2. Every eval call
3. Every sample call

### Exercise 2: Error Recovery
Implement graceful error handling:
1. Catch decode failures
2. Auto-recover from context full
3. Retry on transient errors

### Exercise 3: Health Check
Create a health check function that:
1. Verifies model is loaded
2. Tests tokenization
3. Runs a simple generation
4. Reports any issues

---

## Key Takeaways

1. **Follow the diagnostic flow** systematically
2. **BOS token is #1 cause** of repetition
3. **Chat template is #1 cause** of bad answers
4. **Add logging** for debugging
5. **Check model logs** during load
6. **Use safe wrappers** to catch crashes

---

## Congratulations!

You've completed the Forge SDK Tutorial Series! 🎉

### What You've Learned

- ✅ llama.cpp fundamentals
- ✅ GGUF models and quantization
- ✅ Tokenization and special tokens
- ✅ Context and memory management
- ✅ Sampling and text generation
- ✅ Chat templates and prompting
- ✅ Swift wrapper architecture
- ✅ Forge SDK components
- ✅ Multimodal/vision models
- ✅ Performance optimization
- ✅ Troubleshooting and debugging

### Next Steps

1. **Build something!** - Create an app using the SDK
2. **Experiment** - Try different models and parameters
3. **Contribute** - Report issues, suggest improvements
4. **Stay updated** - llama.cpp evolves rapidly

👉 **[Return to Tutorial Index](../tutorial/00-index.md)**
