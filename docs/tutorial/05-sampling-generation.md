# Lesson 05: Sampling & Text Generation

Master the art of controlling model output through sampling parameters.

---

## What is Sampling?

After the model processes input, it outputs **logits** - raw scores for each token in the vocabulary. Sampling converts these logits into a probability distribution and selects the next token.

```
┌─────────────────────────────────────────────────────────────┐
│                    SAMPLING PIPELINE                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Logits (raw scores)                                         │
│  [2.1, 0.5, 3.8, -1.2, 1.9, ...]  (65536 values)            │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              SAMPLER CHAIN                           │    │
│  │  1. Logit Bias                                       │    │
│  │  2. Penalties (repeat, frequency, presence)          │    │
│  │  3. DRY (Don't Repeat Yourself)                      │    │
│  │  4. Top-K                                            │    │
│  │  5. Top-P (nucleus)                                  │    │
│  │  6. Min-P                                            │    │
│  │  7. Temperature                                      │    │
│  │  8. Distribution sampler                             │    │
│  └─────────────────────────────────────────────────────┘    │
│         │                                                    │
│         ▼                                                    │
│  Selected Token ID: 1847                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## The Sampler Chain

Samplers are applied in order. **Order matters!**

```
Logit Bias → Penalties → DRY → Top-N-Sigma → Top-K → Typical → Top-P → Min-P → XTC → Temp → Dist
```

### Why Order Matters

```
Example: 65536 tokens

After Top-K (k=40):     40 tokens remain
After Top-P (p=0.95):   ~20 tokens remain
After Min-P (p=0.15):   ~10 tokens remain
After Temperature:      Probabilities adjusted
After Dist:             1 token selected
```

If you apply Temperature before Top-K, you get different results!

---

## Core Sampling Parameters

### 1. Temperature

Controls randomness. Lower = more deterministic, higher = more creative.

```
Temperature = 0.0:  Always pick highest probability token (greedy)
Temperature = 0.3:  Very focused, predictable
Temperature = 0.7:  Balanced creativity
Temperature = 1.0:  Normal distribution
Temperature = 1.5:  More random, potentially incoherent
```

**Visualization:**

```
Original probabilities:
Token A: 0.5  ████████████████████
Token B: 0.3  ████████████
Token C: 0.1  ████
Token D: 0.1  ████

Temperature = 0.3 (sharpen):
Token A: 0.85 ██████████████████████████████████
Token B: 0.12 █████
Token C: 0.02 █
Token D: 0.01

Temperature = 1.5 (flatten):
Token A: 0.35 ██████████████
Token B: 0.30 ████████████
Token C: 0.18 ███████
Token D: 0.17 ███████
```

**Recommendations:**
| Use Case | Temperature |
|----------|-------------|
| Code generation | 0.1 - 0.3 |
| Factual Q&A | 0.3 - 0.5 |
| Chat/conversation | 0.5 - 0.7 |
| Creative writing | 0.8 - 1.0 |
| Brainstorming | 1.0 - 1.3 |

### 2. Top-K

Keep only the K highest probability tokens.

```c
llama_sampler_init_top_k(40)  // Keep top 40 tokens
```

```
Before Top-K (65536 tokens):
Token 1: 0.15
Token 2: 0.12
Token 3: 0.08
...
Token 40: 0.001
Token 41: 0.0009  ← Removed
...

After Top-K (40 tokens):
Only tokens 1-40 remain
```

**Recommendations:**
- Default: 40
- More diverse: 100
- More focused: 10-20

### 3. Top-P (Nucleus Sampling)

Keep tokens until cumulative probability exceeds P.

```c
llama_sampler_init_top_p(0.95, 1)  // Keep tokens summing to 95%
```

```
Sorted by probability:
Token A: 0.40  cumsum: 0.40
Token B: 0.25  cumsum: 0.65
Token C: 0.15  cumsum: 0.80
Token D: 0.10  cumsum: 0.90
Token E: 0.05  cumsum: 0.95 ← Cutoff at p=0.95
Token F: 0.03  cumsum: 0.98 ← Removed
Token G: 0.02  cumsum: 1.00 ← Removed
```

**Recommendations:**
- Default: 0.95
- More focused: 0.85-0.90
- More diverse: 0.98-0.99

### 4. Min-P

Remove tokens with probability below `min_p × max_probability`.

```c
llama_sampler_init_min_p(0.15, 1)  // Remove if prob < 0.15 × max_prob
```

```
Max probability = 0.40

Threshold = 0.40 × 0.15 = 0.06

Token A: 0.40  ✓ Keep (above 0.06)
Token B: 0.25  ✓ Keep
Token C: 0.15  ✓ Keep
Token D: 0.10  ✓ Keep
Token E: 0.05  ✗ Remove (below 0.06)
Token F: 0.03  ✗ Remove
```

**Why Min-P is useful:**
- Adapts to the probability distribution
- Works better than fixed Top-K for varying confidence
- Recommended over Top-K alone

---

## Penalty Samplers

### 5. Repeat Penalty

Penalizes tokens that appeared recently.

```c
llama_sampler_init_penalties(
    64,     // repeat_last_n: Look back this many tokens
    1.15,   // repeat_penalty: Multiply logit by 1/penalty
    0.0,    // frequency_penalty
    0.0     // presence_penalty
)
```

```
Recent tokens: ["the", "cat", "sat", "on", "the", "mat"]

Without penalty:
"the": 0.25 → "the": 0.25

With repeat_penalty=1.15:
"the": 0.25 → "the": 0.25 / 1.15 = 0.217
```

**Recommendations:**
- Default: 1.15
- Stronger: 1.2-1.3
- Weaker: 1.05-1.1

### 6. Frequency Penalty

Penalizes based on how often token appeared.

```c
// Part of penalties sampler
frequency_penalty = 0.5
```

```
"the" appeared 3 times
New logit = original_logit - (3 × 0.5) = original - 1.5
```

### 7. Presence Penalty

Penalizes any token that appeared (binary).

```c
// Part of penalties sampler
presence_penalty = 0.5
```

```
"the" appeared (at least once)
New logit = original_logit - 0.5
```

---

## Advanced Samplers

### 8. DRY (Don't Repeat Yourself)

Prevents repeating sequences of tokens, not just individual tokens.

```c
llama_sampler_init_dry(
    vocab,
    n_vocab,
    0.8,    // dry_multiplier: Strength of penalty
    1.75,   // dry_base: Exponential base
    2,      // dry_allowed_length: Min sequence before penalty
    256,    // dry_penalty_last_n: Look-back window
    NULL, 0 // Sequence breakers
)
```

**How DRY works:**

```
Previous output: "I think that the problem is that the solution is"

Without DRY:
Model might continue: "that the answer is that the result is..."
(repeating "that the X is" pattern)

With DRY:
Penalizes starting sequences we've seen before
Model forced to vary: "complex. Let me explain..."
```

**When to use:**
- Long-form generation
- Models prone to repetitive patterns
- When repeat_penalty alone isn't enough

### 9. Top-N-Sigma

Statistical filtering based on standard deviation.

```c
llama_sampler_init_top_n_sigma(0.0)  // 0 = disabled
```

### 10. XTC (Exclude Top Choices)

Randomly excludes top tokens to force creativity.

```c
llama_sampler_init_xtc(
    0.0,   // probability: Chance of applying
    0.1    // threshold: Tokens above this prob may be excluded
)
```

---

## Complete Sampler Chain Implementation

```swift
func initSampler() {
    let sparams = llama_sampler_chain_default_params()
    sampling = llama_sampler_chain_init(sparams)

    guard let sampling = sampling, let model = model else { return }
    let vocab = llama_model_get_vocab(model)

    // 1. Logit bias (optional)
    if sampleParams.logit_bias_count > 0 {
        llama_sampler_chain_add(sampling,
            llama_sampler_init_logit_bias(
                llama_vocab_n_tokens(vocab),
                sampleParams.logit_bias_count,
                sampleParams.logit_bias))
    }

    // 2. Penalties
    llama_sampler_chain_add(sampling,
        llama_sampler_init_penalties(
            Int32(sampleParams.repeat_last_n),  // 64
            sampleParams.repeat_penalty,         // 1.15
            sampleParams.frequencyPenalty,       // 0.0
            sampleParams.presencePenalty))       // 0.0

    // 3. DRY sampler
    if sampleParams.dry_multiplier > 0 {
        llama_sampler_chain_add(sampling,
            llama_sampler_init_dry(
                vocab,
                Int32(llama_vocab_n_tokens(vocab)),
                sampleParams.dry_multiplier,      // 0.8
                sampleParams.dry_base,            // 1.75
                Int32(sampleParams.dry_allowed_length),  // 2
                Int32(sampleParams.dry_penalty_last_n),  // 256
                nil, 0))
    }

    // 4. Top-K
    llama_sampler_chain_add(sampling,
        llama_sampler_init_top_k(sampleParams.top_k))  // 40

    // 5. Top-P
    llama_sampler_chain_add(sampling,
        llama_sampler_init_top_p(sampleParams.top_p, 1))  // 0.95

    // 6. Min-P
    llama_sampler_chain_add(sampling,
        llama_sampler_init_min_p(sampleParams.min_p, 1))  // 0.15

    // 7. Temperature
    llama_sampler_chain_add(sampling,
        llama_sampler_init_temp(sampleParams.temp))  // 0.3

    // 8. Distribution sampler (final selection)
    llama_sampler_chain_add(sampling,
        llama_sampler_init_dist(UInt32.random(in: 0..<UInt32.max)))
}
```

---

## Sampler Accept

**Critical**: Call `llama_sampler_accept` after each token to update penalty tracking.

```swift
// In llm_eval, after decoding:
if let smpl = self.sampling {
    llama_sampler_accept(smpl, outputToken)
}
```

Without this, repeat penalty won't work correctly!

---

## Parameter Presets

### Focused/Deterministic
```json
{
  "temperature": 0.3,
  "top_p": 0.90,
  "top_k": 20,
  "min_p": 0.15,
  "repeat_penalty": 1.15
}
```

### Balanced
```json
{
  "temperature": 0.7,
  "top_p": 0.95,
  "top_k": 40,
  "min_p": 0.10,
  "repeat_penalty": 1.10
}
```

### Creative
```json
{
  "temperature": 1.0,
  "top_p": 0.98,
  "top_k": 100,
  "min_p": 0.05,
  "repeat_penalty": 1.05
}
```

---

## Debugging Sampling

### Issue: Repetition

```
Output: "I am well I am well I am well..."
```

**Fixes:**
1. ✅ Increase `repeat_penalty` (1.15 → 1.2)
2. ✅ Enable DRY sampler (`dry_multiplier: 0.8`)
3. ✅ Check `llama_sampler_accept` is called
4. ✅ Check BOS token only on first turn

### Issue: Too Random

```
Output: "The cat went to the purple banana economics yesterday..."
```

**Fixes:**
1. ✅ Lower temperature (1.0 → 0.5)
2. ✅ Lower Top-P (0.95 → 0.85)
3. ✅ Increase Min-P (0.05 → 0.15)

### Issue: Too Boring

```
Output: "I don't know. I don't know. I'm not sure."
```

**Fixes:**
1. ✅ Increase temperature (0.3 → 0.7)
2. ✅ Increase Top-P (0.90 → 0.95)
3. ✅ Enable XTC for variety

---

## Exercises

### Exercise 1: Temperature Experiment
Generate the same prompt 5 times at temperatures 0.1, 0.3, 0.5, 0.7, 1.0.
Compare the outputs.

### Exercise 2: Trace the Chain
In `LLaMa.swift`, find `initSampler()` and:
1. List all samplers in order
2. Identify the parameter for each
3. Find where `llama_sampler_sample` is called

### Exercise 3: Fix Repetition
Given a model that outputs "Hello Hello Hello...":
1. What parameters would you change?
2. In what order would you try them?

---

## Key Takeaways

1. **Temperature** = randomness dial
2. **Top-K/Top-P/Min-P** = filter candidates
3. **Penalties** = prevent repetition
4. **DRY** = prevent phrase repetition
5. **Order matters** in sampler chain
6. **Always call `llama_sampler_accept`**

---

## Quick Reference

```
Logit Bias → Penalties → DRY → Top-N-Sigma → Top-K → Typical → Top-P → Min-P → XTC → Temp → Dist
```

| Parameter | Default | Range | Effect |
|-----------|---------|-------|--------|
| `temperature` | 0.3 | 0.0-2.0 | Randomness |
| `top_k` | 40 | 1-100+ | Candidate count |
| `top_p` | 0.95 | 0.5-1.0 | Nucleus cutoff |
| `min_p` | 0.15 | 0.0-0.5 | Probability floor |
| `repeat_penalty` | 1.15 | 1.0-2.0 | Repeat suppression |
| `dry_multiplier` | 0.8 | 0.0-2.0 | Pattern suppression |

---

## Next Lesson

👉 **[Lesson 06: Chat Templates & Prompting →](./06-chat-templates.md)**
