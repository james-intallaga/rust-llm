# Model Tuning Quick Reference

---

## ⚠️ CRITICAL FIXES (Must Apply)

| # | Fix | File | Impact |
|---|-----|------|--------|
| 1 | **BOS token only on first turn** (`nPast == 0`) | `LLMBase.swift` | Fixes repetition in recurrent models |
| 2 | **Chat template with role markers** (`<\|im_start\|>`, `<\|im_end\|>`) | `LLMBase.swift`, `LLaMa_MModal.swift` | Fixes role confusion / "stupid answers" |
| 3 | **KV cache position = `nPast + i`** | `LLMBase.swift` | Fixes broken/incoherent responses |
| 4 | **Feed EOG token back to model** | `LLMBase.swift` | Fixes multi-turn context loss |
| 5 | **Enable DRY sampler** (`dry_multiplier: 0.8`) | `LLaMa.swift`, Profile JSON | Prevents phrase repetition |
| 6 | **Call `llama_sampler_accept` in `llm_eval`** | `LLaMa.swift` | Updates penalty tracking |

---

## Universal Settings (All SLMs)

| Setting | Value/Rule | Why |
|---------|------------|-----|
| BOS Token | Only on first turn (`nPast == 0`) | Prevents memory corruption in recurrent models |
| Token Position | `nPast + i` for each token | Correct KV cache indexing |
| EOG Handling | Feed EOG token back to model | Proper turn transitions |
| Sampler Accept | Call in `llm_eval` after decode | Updates penalty state |
| `repeat_penalty` | 1.15 | Prevents repetition |
| `repeat_last_n` | 64 | Penalty window size |
| `top_k` | 40 | Standard top-k |
| `top_p` | 0.95 | Nucleus sampling |
| `min_p` | 0.15 | Filters low probability |

---

## LFM2-VL 3B Specific Settings

| Setting | Value | Notes |
|---------|-------|-------|
| `temperature` | 0.3 | Lower for focused responses |
| `dry_multiplier` | 0.8 | Don't Repeat Yourself sampler |
| `dry_base` | 1.75 | DRY exponential base |
| `dry_allowed_length` | 2 | Min sequence before penalty |
| `dry_penalty_last_n` | 256 | DRY window size |
| BOS Token ID | 1 (`<\|startoftext\|>`) | |
| EOS Token ID | 7 (`<\|im_end\|>`) | Also EOG |
| EOT Token ID | 2 (`<\|endoftext\|>`) | Also EOG |
| Image Marker | `<image>` (ID: 396) | For vision inputs |

---

## Chat Template (LFM2)

**First Turn:**
```
<|startoftext|><|im_start|>system
{system}
<|im_end|><|im_start|>user
{user}
<|im_end|><|im_start|>assistant
```

**Subsequent Turns:**
```
<|im_start|>user
{user}
<|im_end|><|im_start|>assistant
```

**Vision (First Turn):**
```
<|startoftext|><|im_start|>system
{system}
<|im_end|><|im_start|>user
<image>
{user}
<|im_end|><|im_start|>assistant
```

---

## Sampler Chain Order

```
Logit Bias → Penalties → DRY → Top-N-Sigma → Top-K → Typical → Top-P → Min-P → XTC → Temp → Dist
```

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| Repetition | BOS on every turn | Only add BOS at `nPast == 0` |
| Repetition | Weak penalty | Increase `repeat_penalty` to 1.15+ |
| Broken output | Wrong positions | Use `nPast + i` for positions |
| Role confusion | Wrong template | Use model's native chat format |
| Empty response | EOG banned | Don't ban EOS/EOT tokens |
