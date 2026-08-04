# Lesson 06: Chat Templates & Prompting

Understanding how to format prompts for chat-style models.

---

## Why Chat Templates Matter

Modern LLMs are trained on specific prompt formats. Using the wrong format causes:
- ❌ Role confusion (model acts as user)
- ❌ Hallucination (invents context)
- ❌ Poor response quality
- ❌ Ignoring instructions

Using the correct format:
- ✅ Model understands its role
- ✅ Follows system instructions
- ✅ Coherent multi-turn conversation
- ✅ Proper response termination

---

## Anatomy of a Chat Prompt

```
┌─────────────────────────────────────────────────────────────┐
│                     CHAT PROMPT                              │
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────┐  │
│  │ BOS TOKEN (Beginning of Sequence)                     │  │
│  │ Only appears once at the very start                   │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ SYSTEM MESSAGE                                        │  │
│  │ Role marker + Instructions + End marker               │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ USER MESSAGE                                          │  │
│  │ Role marker + User input + End marker                 │  │
│  └───────────────────────────────────────────────────────┘  │
│                           │                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ ASSISTANT (Open - no end marker)                      │  │
│  │ Role marker only, model continues from here           │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## Common Template Formats

### LFM2 / ChatML Format

```
<|startoftext|><|im_start|>system
You are a helpful assistant.
<|im_end|><|im_start|>user
Hello, how are you?
<|im_end|><|im_start|>assistant
```

### LLaMA 2 Chat Format

```
<s>[INST] <<SYS>>
You are a helpful assistant.
<</SYS>>

Hello, how are you? [/INST]
```

### LLaMA 3 Format

```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>

You are a helpful assistant.<|eot_id|><|start_header_id|>user<|end_header_id|>

Hello, how are you?<|eot_id|><|start_header_id|>assistant<|end_header_id|>

```

### Phi Format

```
<|system|>
You are a helpful assistant.<|end|>
<|user|>
Hello, how are you?<|end|>
<|assistant|>
```

---

## LFM2 Template Deep Dive

### Special Tokens

| Token | ID | Purpose |
|-------|-----|---------|
| `<\|startoftext\|>` | 1 | BOS - Start of conversation |
| `<\|endoftext\|>` | 2 | EOT - End of text (EOG) |
| `<\|im_start\|>` | 6 | Start of role |
| `<\|im_end\|>` | 7 | End of role (EOG) |

### First Turn Template

```swift
func lfm2ChatPrompt(system: String, user: String) -> String {
    return """
<|startoftext|><|im_start|>system
\(system)
<|im_end|><|im_start|>user
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

**Result:**
```
<|startoftext|><|im_start|>system
You are Amma, a friendly assistant.
<|im_end|><|im_start|>user
Hello!
<|im_end|><|im_start|>assistant
```

### Subsequent Turn Template

```swift
func lfm2UserTurn(user: String) -> String {
    return """
<|im_start|>user
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

**Note:** No `<|startoftext|>` on subsequent turns!

---

## Multi-Turn Conversation Flow

```
Turn 1 (First message):
┌────────────────────────────────────────────────────────────┐
│ <|startoftext|><|im_start|>system                         │
│ You are Amma.                                              │
│ <|im_end|><|im_start|>user                                │
│ Hello!                                                      │
│ <|im_end|><|im_start|>assistant                           │
│ Hi! How can I help you today?<|im_end|>  ← Model generates │
└────────────────────────────────────────────────────────────┘

Turn 2 (Second message):
┌────────────────────────────────────────────────────────────┐
│ <|im_start|>user                                          │
│ What's the weather?                                        │
│ <|im_end|><|im_start|>assistant                           │
│ I don't have weather data...<|im_end|>  ← Model generates  │
└────────────────────────────────────────────────────────────┘

Full context after Turn 2:
┌────────────────────────────────────────────────────────────┐
│ <|startoftext|><|im_start|>system                         │
│ You are Amma.                                              │
│ <|im_end|><|im_start|>user                                │
│ Hello!                                                      │
│ <|im_end|><|im_start|>assistant                           │
│ Hi! How can I help you today?<|im_end|>                    │
│ <|im_start|>user                                          │
│ What's the weather?                                        │
│ <|im_end|><|im_start|>assistant                           │
│ I don't have weather data...<|im_end|>                     │
└────────────────────────────────────────────────────────────┘
```

---

## Vision/Multimodal Templates

For models with vision capabilities, embed the image marker:

### LFM2-VL Template

```swift
func lfm2VisionPrompt(system: String, user: String) -> String {
    return """
<|startoftext|><|im_start|>system
\(system)
<|im_end|><|im_start|>user
<image>
\(user)
<|im_end|><|im_start|>assistant
"""
}
```

**Image Position:**
```
<|im_start|>user
<image>           ← Image tokens inserted here
What's in this picture?
<|im_end|>
```

### Image Processing Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Image     │     │    CLIP     │     │   Embed     │
│   File      │ ──▶ │  Encoder    │ ──▶ │   Tokens    │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
┌─────────────────────────────────────────────────────────┐
│  <|im_start|>user                                       │
│  [256 image embedding tokens]                            │
│  What is this?                                          │
│  <|im_end|><|im_start|>assistant                       │
└─────────────────────────────────────────────────────────┘
```

---

## Common Mistakes

### Mistake 1: Plain Text Prompts

```
❌ Wrong:
"You are an assistant. User says: Hello. Assistant:"

✅ Correct:
"<|startoftext|><|im_start|>system
You are an assistant.
<|im_end|><|im_start|>user
Hello
<|im_end|><|im_start|>assistant"
```

### Mistake 2: BOS on Every Turn

```
❌ Wrong:
Turn 1: <|startoftext|>...<|im_end|>
Turn 2: <|startoftext|>...<|im_end|>  ← Extra BOS!

✅ Correct:
Turn 1: <|startoftext|>...<|im_end|>
Turn 2: <|im_start|>user...          ← No BOS
```

### Mistake 3: Closing Assistant Tag

```
❌ Wrong:
<|im_start|>assistant
<|im_end|>    ← Model has nothing to generate!

✅ Correct:
<|im_start|>assistant
              ← Leave open for model to continue
```

### Mistake 4: Missing EOG Processing

```
❌ Wrong:
Model outputs: "Hello!<|im_end|>"
Display to user: "Hello!<|im_end|>"  ← Shows raw token

✅ Correct:
Model outputs: "Hello!<|im_end|>"
Check: llama_token_is_eog(token) → true → Stop
Display: "Hello!"
```

---

## Implementation in Forge SDK

### LLMBase.swift - Predict Function

```swift
public func Predict(
    _ input: String,
    _ callback: @escaping (String, Double) -> Bool
) throws -> String {

    // Check if this is the first turn
    let isFirstTurn = (self.nPast == 0)

    if isFirstTurn {
        // Build first turn with system prompt
        let systemPrompt = self.system_prompt ?? "You are a helpful assistant."
        let fullPrompt = lfm2ChatPrompt(system: systemPrompt, user: input)
        let tokens = LLMTokenize(fullPrompt)
        // ... evaluate tokens ...
    } else {
        // Build subsequent turn (no system, no BOS)
        let turnPrompt = lfm2UserTurn(user: input)
        let tokens = LLMTokenize(turnPrompt)
        // ... evaluate tokens ...
    }

    // Generation loop
    while generating {
        let token = llm_sample()

        // Check for end of generation
        if llama_token_is_eog(model, token) {
            // Feed EOG back to model for state update
            _ = try? llm_eval(inputBatch: [token])
            break
        }

        // Output token
        let piece = LLMTokenToStr(token: token)
        callback(piece, 0.0)
    }
}
```

---

## Template Detection

Models include their template in metadata:

```python
# From GGUF metadata
tokenizer.chat_template = """
{{bos_token}}{% for message in messages %}
{{'<|im_start|>' + message['role'] + '\n' + message['content'] + '<|im_end|>' + '\n'}}
{% endfor %}
{% if add_generation_prompt %}
{{'<|im_start|>assistant\n'}}
{% endif %}
"""
```

### Reading Template in Code

```c
// Get chat template from model
const char * tmpl = llama_model_meta_val_str(model, "tokenizer.chat_template");
```

---

## System Prompt Best Practices

### Good System Prompts

```
✅ Clear identity:
"You are Amma, a friendly personal assistant."

✅ Specific capabilities:
"You are Amma, a friendly personal assistant with vision capabilities.
You can see and describe images."

✅ Behavior guidelines:
"You are Amma, a helpful assistant. Be concise and friendly.
If you don't know something, say so."
```

### Avoid

```
❌ Too long:
(500 words of instructions...)

❌ Contradictory:
"Be brief. Provide detailed explanations."

❌ Role-play heavy:
"You are a wizard from a magical realm..."
(Unless specifically needed)
```

---

## Exercises

### Exercise 1: Template Analysis
Find the chat template in the model loading logs. Parse it to understand:
1. What tokens mark role boundaries?
2. Where does the BOS token go?
3. How is the assistant turn started?

### Exercise 2: Multi-Turn Trace
Add logging to `Predict()` in `LLMBase.swift`:
```swift
print("Input prompt: \(fullPrompt)")
print("Token count: \(tokens.count)")
```
Observe how the prompt changes across turns.

### Exercise 3: Template Comparison
Compare templates for 3 different models:
1. LFM2
2. LLaMA 3
3. Phi

What are the similarities? Differences?

---

## Key Takeaways

1. **Use the model's native template** - don't invent your own
2. **BOS only once** at the start of conversation
3. **Leave assistant turn open** for generation
4. **Handle EOG tokens** to know when to stop
5. **Feed EOG back** to model for proper state
6. **Image marker inside user turn** for vision models

---

## Template Quick Reference

### LFM2 First Turn
```
<|startoftext|><|im_start|>system
{system}
<|im_end|><|im_start|>user
{user}
<|im_end|><|im_start|>assistant
```

### LFM2 Subsequent Turn
```
<|im_start|>user
{user}
<|im_end|><|im_start|>assistant
```

### LFM2 Vision First Turn
```
<|startoftext|><|im_start|>system
{system}
<|im_end|><|im_start|>user
<image>
{user}
<|im_end|><|im_start|>assistant
```

---

## Next Lesson

👉 **[Lesson 07: Swift Wrapper Architecture →](./07-swift-wrapper-architecture.md)**
