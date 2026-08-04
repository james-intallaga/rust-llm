//! Token sampling with RAII and proper sampler chain
//!
//! The sampler chain order follows the documented best practices:
//! 1. Logit Bias (if any)
//! 2. Penalties (repeat, frequency, presence)
//! 3. DRY (Don't Repeat Yourself)
//! 4. Top-N-Sigma (if enabled)
//! 5. Top-K
//! 6. Typical-P (if enabled)
//! 7. Top-P (nucleus)
//! 8. Min-P
//! 9. XTC (if enabled)
//! 10. Temperature
//! 11. Distribution Sampler (final selection)

use std::ptr::NonNull;

use llama_cpp_sys::{
    c_float, llama_context, llama_model, llama_model_get_vocab, llama_model_n_ctx_train,
    llama_sampler, llama_sampler_accept, llama_sampler_chain_add,
    llama_sampler_chain_default_params, llama_sampler_chain_init, llama_sampler_free,
    llama_sampler_init_dist, llama_sampler_init_dry, llama_sampler_init_greedy,
    llama_sampler_init_min_p, llama_sampler_init_penalties, llama_sampler_init_temp,
    llama_sampler_init_top_k, llama_sampler_init_top_p, llama_sampler_init_typical,
    llama_sampler_reset, llama_sampler_sample,
};

/// Sampling parameters - comprehensive set matching Swift implementation
#[derive(Debug, Clone)]
pub struct SamplerParams {
    // Core sampling
    /// Temperature (0.0 = greedy, higher = more random)
    pub temperature: f32,
    /// Top-K sampling (0 = disabled)
    pub top_k: i32,
    /// Top-P (nucleus) sampling (1.0 = disabled)
    pub top_p: f32,
    /// Min-P sampling (0.0 = disabled)
    pub min_p: f32,
    /// Typical-P sampling (1.0 = disabled)
    pub typical_p: f32,

    // Penalties
    /// Repeat penalty (1.0 = disabled)
    pub repeat_penalty: f32,
    /// Number of tokens to consider for repeat penalty
    pub repeat_last_n: i32,
    /// Frequency penalty (0.0 = disabled)
    pub frequency_penalty: f32,
    /// Presence penalty (0.0 = disabled)
    pub presence_penalty: f32,

    // DRY sampler (Don't Repeat Yourself)
    /// DRY multiplier (0.0 = disabled)
    pub dry_multiplier: f32,
    /// DRY base
    pub dry_base: f32,
    /// DRY allowed length before penalty applies
    pub dry_allowed_length: i32,
    /// DRY penalty window
    pub dry_penalty_last_n: i32,

    // Random seed
    /// Seed for random sampling (None = random)
    pub seed: Option<u32>,
}

impl Default for SamplerParams {
    fn default() -> Self {
        Self {
            // Core - LFM2-VL recommended values
            temperature: 0.3,
            top_k: 40,
            top_p: 0.95,
            min_p: 0.15,
            typical_p: 1.0, // Disabled by default

            // Penalties
            repeat_penalty: 1.15,
            repeat_last_n: 64,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,

            // DRY (Don't Repeat Yourself) - helps prevent repetition
            dry_multiplier: 0.8,
            dry_base: 1.75,
            dry_allowed_length: 2,
            dry_penalty_last_n: 256,

            // Random seed
            seed: None,
        }
    }
}

/// A token sampler with proper chain implementation
///
/// Wraps llama_sampler_chain with RAII for automatic cleanup.
/// Implements the correct sampler order as documented.
pub struct Sampler {
    ptr: NonNull<llama_sampler>,
    params: SamplerParams,
}

// Safety: Sampler owns the pointer
unsafe impl Send for Sampler {}

impl Sampler {
    /// Create a new sampler chain with the given parameters
    ///
    /// The chain is built in the correct order:
    /// 1. Penalties → 2. DRY → 3. Top-K → 4. Typical-P → 5. Top-P
    ///    → 6. Min-P → 7. Temperature → 8. Distribution
    pub fn new(params: SamplerParams, model: Option<*const llama_model>) -> Self {
        log::info!(
            "Creating sampler chain: temp={}, top_k={}, top_p={}, min_p={}, repeat_penalty={}",
            params.temperature,
            params.top_k,
            params.top_p,
            params.min_p,
            params.repeat_penalty
        );

        // Greedy mode: skip the chain entirely
        if params.temperature <= 0.0 {
            let ptr = unsafe { llama_sampler_init_greedy() };
            let ptr = NonNull::new(ptr).expect("Failed to create greedy sampler");
            return Self { ptr, params };
        }

        // Initialize sampler chain
        let chain_params = unsafe { llama_sampler_chain_default_params() };
        let chain = unsafe { llama_sampler_chain_init(chain_params) };

        if chain.is_null() {
            panic!("Failed to create sampler chain");
        }

        // Generate random seed if not specified
        let seed = params.seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u32)
                .unwrap_or(12345)
        });

        unsafe {
            // ============================================================
            // SAMPLER CHAIN ORDER (from model-tuning-guide.md)
            // ============================================================

            // 1. Logit Bias - skipped for now (requires explicit bias list)

            // 2. Penalties (repeat, frequency, presence)
            if params.repeat_penalty != 1.0
                || params.frequency_penalty != 0.0
                || params.presence_penalty != 0.0
            {
                log::debug!(
                    "Adding penalties: repeat={}, freq={}, presence={}",
                    params.repeat_penalty,
                    params.frequency_penalty,
                    params.presence_penalty
                );
                llama_sampler_chain_add(
                    chain,
                    llama_sampler_init_penalties(
                        params.repeat_last_n,
                        params.repeat_penalty as c_float,
                        params.frequency_penalty as c_float,
                        params.presence_penalty as c_float,
                    ),
                );
            }

            // 3. DRY (Don't Repeat Yourself)
            if params.dry_multiplier > 0.0 {
                if let Some(model_ptr) = model {
                    let vocab = llama_model_get_vocab(model_ptr);
                    let n_ctx_train = llama_model_n_ctx_train(model_ptr);
                    log::debug!(
                        "Adding DRY: multiplier={}, base={}, allowed_len={}, n_ctx_train={}",
                        params.dry_multiplier,
                        params.dry_base,
                        params.dry_allowed_length,
                        n_ctx_train
                    );
                    llama_sampler_chain_add(
                        chain,
                        llama_sampler_init_dry(
                            vocab,
                            n_ctx_train, // FIXED: use n_ctx_train, not n_vocab
                            params.dry_multiplier as c_float,
                            params.dry_base as c_float,
                            params.dry_allowed_length,
                            params.dry_penalty_last_n,
                            std::ptr::null_mut(), // No custom sequence breakers
                            0,
                        ),
                    );
                } else {
                    log::warn!("DRY sampler requires model, skipping");
                }
            }

            // 4. Top-N-Sigma - not commonly used, skipped

            // 5. Top-K
            if params.top_k > 0 {
                log::debug!("Adding top_k: {}", params.top_k);
                llama_sampler_chain_add(chain, llama_sampler_init_top_k(params.top_k));
            }

            // 6. Typical-P
            if params.typical_p > 0.0 && params.typical_p < 1.0 {
                log::debug!("Adding typical_p: {}", params.typical_p);
                llama_sampler_chain_add(
                    chain,
                    llama_sampler_init_typical(params.typical_p as c_float, 1),
                );
            }

            // 7. Top-P (nucleus)
            if params.top_p > 0.0 && params.top_p < 1.0 {
                log::debug!("Adding top_p: {}", params.top_p);
                llama_sampler_chain_add(
                    chain,
                    llama_sampler_init_top_p(params.top_p as c_float, 1),
                );
            }

            // 8. Min-P
            if params.min_p > 0.0 {
                log::debug!("Adding min_p: {}", params.min_p);
                llama_sampler_chain_add(
                    chain,
                    llama_sampler_init_min_p(params.min_p as c_float, 1),
                );
            }

            // 9. XTC - not commonly used, skipped

            // 10. Temperature
            log::debug!("Adding temperature: {}", params.temperature);
            llama_sampler_chain_add(
                chain,
                llama_sampler_init_temp(params.temperature as c_float),
            );

            // 11. Distribution Sampler (MUST BE LAST)
            log::debug!("Adding distribution sampler with seed: {}", seed);
            llama_sampler_chain_add(chain, llama_sampler_init_dist(seed));
        }

        let ptr = NonNull::new(chain).expect("Sampler chain should not be null");

        Self { ptr, params }
    }

    /// Create a new sampler without model reference (DRY will be disabled)
    pub fn new_without_model(params: SamplerParams) -> Self {
        Self::new(params, None)
    }

    /// Create a greedy sampler (always picks highest probability token)
    pub fn greedy() -> Self {
        Self::new(
            SamplerParams {
                temperature: 0.0,
                ..Default::default()
            },
            None,
        )
    }

    /// Sample a token from the context
    ///
    /// # Arguments
    ///
    /// * `ctx` - The context to sample from
    /// * `idx` - Index in the batch to sample from (-1 for last)
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn sample(&self, ctx: *mut llama_context, idx: i32) -> i32 {
        unsafe { llama_sampler_sample(self.ptr.as_ptr(), ctx, idx) }
    }

    /// Accept a token (update internal state for repeat penalty, etc.)
    ///
    /// CRITICAL: Must be called for every token that gets evaluated,
    /// not just sampled tokens. This keeps the repeat penalty up to date.
    pub fn accept(&self, token: i32) {
        unsafe {
            llama_sampler_accept(self.ptr.as_ptr(), token);
        }
    }

    /// Reset the sampler state
    ///
    /// CRITICAL for multimodal: Must be called before each new image
    /// to clear accumulated state (repeat penalty, mirostat mu, RNG).
    /// See docs/troubleshooting/multimodal-issues.md
    pub fn reset(&self) {
        log::debug!("Resetting sampler state");
        unsafe {
            llama_sampler_reset(self.ptr.as_ptr());
        }
    }

    /// Get the sampling parameters
    pub fn params(&self) -> &SamplerParams {
        &self.params
    }

    /// Get the raw pointer (for advanced usage)
    pub fn as_ptr(&self) -> *mut llama_sampler {
        self.ptr.as_ptr()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        log::debug!("Freeing sampler chain");
        unsafe {
            llama_sampler_free(self.ptr.as_ptr());
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new_without_model(SamplerParams::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params() {
        let params = SamplerParams::default();
        assert_eq!(params.temperature, 0.3);
        assert_eq!(params.top_k, 40);
        assert_eq!(params.top_p, 0.95);
        assert_eq!(params.min_p, 0.15);
        assert_eq!(params.repeat_penalty, 1.15);
        assert_eq!(params.dry_multiplier, 0.8);
    }
}
