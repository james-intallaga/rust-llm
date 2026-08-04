## Camera Crash Postmortem (LFM2-VL Q8_0, iOS)

### Timeline & Symptoms
- Opening the camera after loading the LFM2-VL 3B Q8_0 model caused crashes / fatal errors.
- Later, the model loaded but multimodal generation returned immediately with EOS (empty response).
- Text-only turns after vision would also exit on EOS as the first generated token.

### Root Causes
- **Logits index mismatch after vision eval:** `llm_sample()` sampled `batch!.n_tokens - 1` even though the multimodal helper didn’t populate our batch, triggering `get_logits_ith` errors.
- **Stale batch state across turns:** Batch token count was nonzero from a previous text turn; vision eval bypassed it, so logits flagging was inconsistent.
- **EOS chosen on first token:** No EOS/`<|im_end|>` bias + higher temperature led the sampler to pick EOS immediately after vision eval.

### Key Fixes
- **Safe sampling index:** If batch is empty, sample with `idx = -1` so llama.cpp uses the last token with logits (`llm_sample()` guard).
- **Reset batch after vision eval:** Clear batch (`llama_batch_clear`) when resetting KV/sampler for a new image turn.
- **Temporary EOS down-bias for first token (vision):** Apply a strong negative bias to EOS and `<|im_end|>` only for the first token after multimodal eval; restore after one token.
- **Lower temp for image turns:** Use `temp = 0.3` (Leap-style) during multimodal generation to reduce early-EOS likelihood.
- **Temporary EOS down-bias for first token (text turns):** Same one-shot guard on the first generated token of text turns; bias is removed immediately after the first token, keeping repetition penalties intact for the rest of the turn.

### What to Watch Going Forward
- Keep `logits_last = true` on the last token of any eval batch.
- When mixing helpers (mtmd) and custom batching, ensure batch bookkeeping matches what the sampler expects.
- For new models/templates, confirm EOS tokens and apply a first-token bias if early termination appears.
- If you hit short replies, check `n_predict` limits before assuming EOS issues.
