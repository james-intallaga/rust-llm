//! Context management with RAII

use std::ptr::NonNull;

use llama_cpp_sys::{
    c_char, c_float, llama_context, llama_context_default_params, llama_context_params,
    llama_decode, llama_flash_attn_type_LLAMA_FLASH_ATTN_TYPE_DISABLED,
    llama_flash_attn_type_LLAMA_FLASH_ATTN_TYPE_ENABLED, llama_free, llama_get_logits_ith,
    llama_get_memory, llama_memory_can_shift, llama_memory_clear, llama_memory_seq_add,
    llama_memory_seq_rm, llama_model_get_vocab, llama_n_ctx, llama_new_context_with_model,
    llama_token_to_piece, llama_tokenize, llama_vocab_is_eog,
};

use crate::batch::BatchArena;
use crate::error::{ForgeError, Result};
use crate::model::LlamaModel; // Still needed for LlamaContext::new() signature

/// Parameters for creating a context
#[derive(Debug, Clone)]
pub struct ContextParams {
    /// Context size (number of tokens that can be processed)
    pub n_ctx: u32,
    /// Batch size for prompt processing
    pub n_batch: u32,
    /// Micro-batch size for continuous batching
    pub n_ubatch: u32,
    /// Number of threads for generation
    pub n_threads: i32,
    /// Number of threads for batch processing
    pub n_threads_batch: i32,
    /// Enable Flash Attention when supported by the backend
    pub flash_attention: bool,
}

impl Default for ContextParams {
    fn default() -> Self {
        Self {
            n_ctx: 2048,
            n_batch: 128, // Lower default for mobile
            n_ubatch: 128,
            n_threads: 2, // Conservative for mobile
            n_threads_batch: 2,
            flash_attention: true,
        }
    }
}

impl From<ContextParams> for llama_context_params {
    fn from(params: ContextParams) -> Self {
        let mut c_params = unsafe { llama_context_default_params() };
        c_params.n_ctx = params.n_ctx;
        c_params.n_batch = params.n_batch;
        c_params.n_ubatch = params.n_ubatch;
        c_params.n_threads = params.n_threads;
        c_params.n_threads_batch = params.n_threads_batch;
        c_params.flash_attn_type = if params.flash_attention {
            llama_flash_attn_type_LLAMA_FLASH_ATTN_TYPE_ENABLED
        } else {
            llama_flash_attn_type_LLAMA_FLASH_ATTN_TYPE_DISABLED
        };
        c_params
    }
}

/// An inference context
///
/// This struct wraps the raw `llama_context` pointer and includes
/// a pre-allocated batch arena. Both are properly freed when dropped.
pub struct LlamaContext {
    ptr: NonNull<llama_context>,
    /// Raw pointer to the llama_model (not LlamaModel wrapper)
    /// This avoids issues with pointer-to-pointer dereferencing
    model_ptr: *mut llama_cpp_sys::llama_model,
    batch: BatchArena,
    n_ctx: u32,
}

// Safety: LlamaContext owns the pointer and controls access
unsafe impl Send for LlamaContext {}

impl LlamaContext {
    /// Create a new context for the given model
    ///
    /// # Arguments
    ///
    /// * `model` - The loaded model to create context for
    /// * `params` - Context creation parameters
    pub fn new(model: &LlamaModel, params: ContextParams) -> Result<Self> {
        log::info!(
            "Creating context: n_ctx={}, n_batch={}",
            params.n_ctx,
            params.n_batch
        );

        let batch_size = params.n_batch as i32;
        let c_params: llama_context_params = params.clone().into();

        let ptr = unsafe { llama_new_context_with_model(model.as_ptr(), c_params) };

        let ptr = NonNull::new(ptr).ok_or_else(|| {
            ForgeError::ContextCreationFailed(
                "llama_new_context_with_model returned null".to_string(),
            )
        })?;

        let n_ctx = unsafe { llama_n_ctx(ptr.as_ptr()) };

        log::info!("Context created: actual n_ctx={}", n_ctx);

        // Create batch arena with the configured batch size
        let batch = BatchArena::new(batch_size, 1);

        Ok(Self {
            ptr,
            model_ptr: model.as_ptr(), // Store the raw C pointer directly
            batch,
            n_ctx,
        })
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut llama_context {
        self.ptr.as_ptr()
    }

    /// Get the actual context size
    pub fn n_ctx(&self) -> u32 {
        self.n_ctx
    }

    /// Get mutable access to the batch arena
    pub fn batch_mut(&mut self) -> &mut BatchArena {
        &mut self.batch
    }

    /// Get the batch arena
    pub fn batch(&self) -> &BatchArena {
        &self.batch
    }

    /// Clear the KV cache (memory)
    pub fn clear_kv_cache(&mut self) {
        log::debug!("Clearing KV cache");
        unsafe {
            // Get the memory handle from context, then clear it
            let mem = llama_get_memory(self.ptr.as_ptr());
            if !mem.is_null() {
                llama_memory_clear(mem, true); // true = clear everything
            }
        }
    }

    /// Check if context supports sliding window
    pub fn can_shift(&self) -> bool {
        unsafe {
            let mem = llama_get_memory(self.ptr.as_ptr());
            if mem.is_null() {
                return false;
            }
            llama_memory_can_shift(mem)
        }
    }

    /// Perform context window sliding when approaching context limit
    ///
    /// This implements the llama.cpp context shifting algorithm:
    /// 1. Keep the first `n_keep` tokens (system prompt)
    /// 2. Remove half of the remaining tokens from the middle
    /// 3. Shift the remaining tokens' positions
    ///
    /// # Arguments
    ///
    /// * `n_past` - Current token position (will be updated)
    /// * `n_keep` - Number of tokens at the start to always keep (system prompt)
    ///
    /// # Returns
    ///
    /// `true` if a shift was performed, `false` if no shift was needed or possible
    pub fn shift_context_if_needed(&mut self, n_past: &mut i32, n_keep: i32) -> bool {
        let n_ctx = self.n_ctx as i32;

        // Check if we're approaching the context limit (90% full)
        if *n_past < (n_ctx * 9 / 10) {
            return false; // No shift needed yet
        }

        unsafe {
            let mem = llama_get_memory(self.ptr.as_ptr());
            if mem.is_null() {
                log::warn!("Cannot shift: memory handle is null");
                return false;
            }

            // Check if memory supports shifting
            if !llama_memory_can_shift(mem) {
                log::warn!("Cannot shift: memory does not support shifting");
                return false;
            }

            // Calculate how many tokens to discard
            let n_left = *n_past - n_keep;
            if n_left <= 0 {
                log::warn!("Cannot shift: n_past ({}) <= n_keep ({})", *n_past, n_keep);
                return false;
            }

            let n_discard = n_left / 2;

            log::info!(
                "Context shift: n_past={}, n_ctx={}, n_keep={}, n_discard={}",
                *n_past,
                n_ctx,
                n_keep,
                n_discard
            );

            // Step 1: Remove tokens in range [n_keep, n_keep + n_discard)
            // seq_id = 0 (default sequence for single-user)
            let removed = llama_memory_seq_rm(mem, 0, n_keep, n_keep + n_discard);
            if !removed {
                log::warn!("llama_memory_seq_rm failed");
                return false;
            }

            // Step 2: Shift remaining tokens' positions by -n_discard
            llama_memory_seq_add(mem, 0, n_keep + n_discard, *n_past, -n_discard);

            // Step 3: Update n_past
            *n_past -= n_discard;

            log::info!("Context shifted: new n_past={}", *n_past);

            true
        }
    }

    /// Decode the current batch
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, `Err` with error code on failure
    pub fn decode(&mut self) -> Result<()> {
        let batch = self.batch.as_batch();
        let result = unsafe { llama_decode(self.ptr.as_ptr(), batch) };

        if result != 0 {
            return Err(ForgeError::DecodeFailed(result));
        }

        Ok(())
    }

    /// Get logits for the i-th token in the batch
    ///
    /// # Safety
    ///
    /// The returned pointer is valid until the next decode call.
    pub fn get_logits(&self, i: i32) -> *mut c_float {
        unsafe { llama_get_logits_ith(self.ptr.as_ptr(), i) }
    }

    /// Tokenize a string
    ///
    /// # Arguments
    ///
    /// * `text` - The text to tokenize
    /// * `add_special` - Whether to add special tokens (BOS, etc.)
    pub fn tokenize(&self, text: &str, add_special: bool) -> Result<Vec<i32>> {
        let vocab = unsafe { llama_model_get_vocab(self.model_ptr) };

        // First, get the number of tokens needed
        let text_bytes = text.as_bytes();
        let text_len = text_bytes.len() as i32;

        // Allocate buffer for tokens (overestimate: 1 token per char + some extra)
        let max_tokens = text_len + 64;
        let mut tokens = vec![0i32; max_tokens as usize];

        let n_tokens = unsafe {
            llama_tokenize(
                vocab,
                text.as_ptr() as *const c_char,
                text_len,
                tokens.as_mut_ptr(),
                max_tokens,
                add_special,
                true, // parse_special
            )
        };

        if n_tokens < 0 {
            return Err(ForgeError::TokenizationFailed(format!(
                "llama_tokenize returned {}",
                n_tokens
            )));
        }

        tokens.truncate(n_tokens as usize);
        Ok(tokens)
    }

    /// Convert a token to its raw byte representation.
    ///
    /// Token boundaries do not necessarily align with UTF-8 character boundaries,
    /// so callers must decode a stream of these bytes incrementally.
    pub fn token_to_piece_bytes(&self, token: i32) -> Vec<u8> {
        let vocab = unsafe { llama_model_get_vocab(self.model_ptr) };

        let mut buf = vec![0u8; 128];
        let len = unsafe {
            llama_token_to_piece(
                vocab,
                token,
                buf.as_mut_ptr() as *mut c_char,
                buf.len() as i32,
                0,     // lstrip
                false, // special
            )
        };

        if len < 0 {
            // Token requires more space
            let needed = (-len) as usize;
            buf.resize(needed, 0);
            let len = unsafe {
                llama_token_to_piece(
                    vocab,
                    token,
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len() as i32,
                    0,
                    false,
                )
            };
            buf.truncate(len as usize);
        } else {
            buf.truncate(len as usize);
        }

        buf
    }

    /// Convert a token to text. Prefer `token_to_piece_bytes` for streaming.
    pub fn token_to_piece(&self, token: i32) -> String {
        String::from_utf8_lossy(&self.token_to_piece_bytes(token)).into_owned()
    }

    /// Check if a token is end-of-generation
    pub fn is_eog(&self, token: i32) -> bool {
        let vocab = unsafe { llama_model_get_vocab(self.model_ptr) };
        unsafe { llama_vocab_is_eog(vocab, token) }
    }
}

impl Drop for LlamaContext {
    fn drop(&mut self) {
        log::debug!("Freeing context");
        // BatchArena will be dropped automatically (RAII)
        unsafe {
            llama_free(self.ptr.as_ptr());
        }
        log::debug!("Context freed");
    }
}
