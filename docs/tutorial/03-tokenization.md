# Lesson 03: Tokenization Deep Dive

Understanding how text becomes numbers and back.

---

## What is Tokenization?

LLMs don't process text directly - they process **tokens** (integer IDs). Tokenization is the process of converting text to token IDs and back.

```
"Hello, world!" ──▶ [15496, 11, 995, 0] ──▶ Model ──▶ [198, 5765] ──▶ "\nSure"
                    Encode                           Decode
```

---

## Tokenization Methods

### Byte-Pair Encoding (BPE)

Most modern LLMs use BPE, which:
1. Starts with individual characters
2. Iteratively merges frequent pairs
3. Creates a vocabulary of subwords

```
Vocabulary building:
"low"    → ["l", "o", "w"]
"lower"  → ["l", "o", "w", "e", "r"]
"lowest" → ["l", "o", "w", "e", "s", "t"]

After merging:
"lo" appears often → merge to single token
"low" appears often → merge to single token

Final:
"lowest" → ["low", "est"]
```

### Token Types

| Type | Example | Description |
|------|---------|-------------|
| Word | "hello" | Complete word |
| Subword | "##ing" | Word piece |
| Character | "a" | Single character |
| Byte | "<0x0A>" | Raw byte |
| Special | "<\|endoftext\|>" | Control token |

---

## Special Tokens

Special tokens control model behavior:

### Common Special Tokens

| Token | Purpose | When to Add |
|-------|---------|-------------|
| **BOS** (Beginning of Sequence) | Marks sequence start | First turn only |
| **EOS** (End of Sequence) | Marks sequence end | After complete response |
| **EOT** (End of Turn) | Marks turn boundary | After each message |
| **PAD** | Padding for batching | When batching sequences |

### LFM2 Special Tokens

| Token | ID | String |
|-------|-----|--------|
| PAD | 0 | `<\|pad\|>` |
| BOS | 1 | `<\|startoftext\|>` |
| EOT | 2 | `<\|endoftext\|>` |
| FIM_PRE | 3 | `<\|fim_pre\|>` |
| FIM_MID | 4 | `<\|fim_mid\|>` |
| FIM_SUF | 5 | `<\|fim_suf\|>` |
| IM_START | 6 | `<\|im_start\|>` |
| IM_END | 7 | `<\|im_end\|>` |

### EOG (End of Generation)

EOG tokens signal that generation should stop:

```c
// Check if token is end-of-generation
if (llama_token_is_eog(model, token)) {
    // Stop generating
}
```

For LFM2, both token 2 (`<|endoftext|>`) and token 7 (`<|im_end|>`) are EOG.

---

## The BOS Token Problem

**⚠️ CRITICAL**: BOS should only appear once at the start of the conversation.

### Wrong (causes repetition):

```
Turn 1: <BOS>system\nYou are helpful.<EOT>user\nHi<EOT>assistant\n
Turn 2: <BOS>user\nHow are you?<EOT>assistant\n   ← WRONG! Extra BOS
Turn 3: <BOS>user\nTell me a joke<EOT>assistant\n ← WRONG! Extra BOS
```

### Correct:

```
Turn 1: <BOS>system\nYou are helpful.<EOT>user\nHi<EOT>assistant\n
Turn 2: user\nHow are you?<EOT>assistant\n         ← No BOS
Turn 3: user\nTell me a joke<EOT>assistant\n       ← No BOS
```

### Implementation Fix

```swift
// In LLMBase.swift
func LLMTokenize(_ input: String) -> [Int32] {
    // Only add BOS on first turn
    let shouldAddBos = (self.nPast == 0)

    let tokens = llama_tokenize(
        vocab,
        input,
        /* add_bos: */ shouldAddBos,
        /* special: */ true
    )
    return tokens
}
```

---

## Tokenization in Practice

### Encoding (Text → Tokens)

```c
const char * text = "Hello, how are you?";
llama_token tokens[256];

int n_tokens = llama_tokenize(
    vocab,           // Vocabulary
    text,            // Input text
    strlen(text),    // Text length
    tokens,          // Output buffer
    256,             // Max tokens
    true,            // Add BOS
    false            // Parse special tokens
);
```

### Decoding (Tokens → Text)

```c
llama_token token = 15496;  // "Hello"
char piece[64];

int n_chars = llama_token_to_piece(
    vocab,           // Vocabulary
    token,           // Token ID
    piece,           // Output buffer
    64,              // Max chars
    0,               // Flags
    true             // Render special tokens
);

printf("%s", piece);  // Prints "Hello"
```

### Swift Wrapper

```swift
// Encode
let tokens = ai.llm.LLMTokenize("Hello, how are you?")

// Decode
let text = ai.llm.LLMTokenToStr(token: tokenId)
```

---

## Token Positions

Each token has a position in the context:

```
Position:  0     1       2       3      4      5       6
Token:   [BOS] [Hello] [,]    [how]  [are]  [you]   [?]
```

### Position Tracking

```swift
// Track position with nPast
var nPast: Int32 = 0

// After evaluating tokens
nPast += Int32(tokens.count)

// Next token goes at position nPast
batch.pos[0] = nPast
```

### KV Cache Positions

The position is used to index into the KV cache:

```
KV Cache:
┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
│ 0  │ 1  │ 2  │ 3  │ 4  │ 5  │ 6  │ 7  │ 8  │... │
├────┼────┼────┼────┼────┼────┼────┼────┼────┼────┤
│BOS │Hello│ , │how │are │you │ ? │ I  │'m  │... │
└────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘

Position must match! Wrong positions = wrong attention.
```

---

## Vocabulary Size

Modern LLMs have large vocabularies:

| Model | Vocab Size |
|-------|------------|
| LLaMA 2 | 32,000 |
| LLaMA 3 | 128,000 |
| LFM2 | 65,536 |
| GPT-4 | ~100,000 |

Larger vocabularies:
- ✅ Better handling of rare words
- ✅ More efficient encoding
- ❌ Larger embedding matrices

---

## Streaming Output

When generating, output tokens incrementally:

```swift
func Predict(input: String, callback: (String, Double) -> Bool) {
    while generating {
        // Sample next token
        let token = llm_sample()

        // Check for EOG
        if llama_token_is_eog(model, token) {
            break
        }

        // Decode to text
        let piece = LLMTokenToStr(token: token)

        // Stream to user
        if !callback(piece, 0.0) {
            break  // User cancelled
        }
    }
}
```

---

## Common Issues

### Issue 1: Garbled Output

**Cause**: Token decoded without context

```swift
// Wrong: Each token decoded separately loses multi-byte chars
for token in tokens {
    print(decode(token))  // Might split UTF-8
}

// Correct: Accumulate and decode together
var allTokens: [Int32] = []
for token in tokens {
    allTokens.append(token)
    let text = decode(allTokens)
    print(text)
}
```

### Issue 2: Wrong Special Tokens

**Cause**: Not using model's actual special token IDs

```swift
// Wrong: Hardcoded IDs
let bos = 1

// Correct: Get from vocabulary
let bos = llama_token_bos(vocab)
```

### Issue 3: Missing Tokens in Output

**Cause**: Not handling special tokens in decoding

```c
// Include special tokens in output
llama_token_to_piece(vocab, token, buf, size, 0, true);
//                                           └── render_special
```

---

## Exercises

### Exercise 1: Token Analysis
Take a sentence and:
1. Tokenize it manually (guess the tokens)
2. Compare with actual tokenization
3. Count total tokens

### Exercise 2: Special Token Hunt
In the model loading logs, find:
1. All EOG tokens
2. The chat template
3. Token IDs for `<|im_start|>` and `<|im_end|>`

### Exercise 3: Position Tracking
Trace through `LLMBase.swift` and find:
1. Where is `nPast` initialized?
2. Where is `nPast` updated?
3. How is `nPast` used in `llm_eval`?

---

## Key Takeaways

1. **Tokenization** converts text ↔ integer IDs
2. **BPE** creates subword tokens
3. **BOS** only on first turn, **EOG** to stop generation
4. **Positions** must be tracked correctly for attention
5. **Stream** tokens for responsive UX

---

## Next Lesson

👉 **[Lesson 04: Context & Memory Management →](./04-context-memory.md)**
