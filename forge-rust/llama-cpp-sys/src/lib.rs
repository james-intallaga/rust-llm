//! llama-cpp-sys: Low-level FFI bindings to llama.cpp
//!
//! This crate provides raw, unsafe bindings to the llama.cpp C API.
//! These bindings are generated automatically by bindgen from the
//! llama.xcframework headers.
//!
//! **WARNING:** All functions in this crate are unsafe and require
//! careful memory management. Use `forge-core` for a safe API.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

// Include the generated bindings
// These files are created by build.rs using bindgen

#[cfg(feature = "generate")]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/llama_bindings.rs"
));

#[cfg(feature = "generate")]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mtmd_bindings.rs"));

// For initial compilation without generated bindings, provide stubs
#[cfg(not(feature = "generate"))]
mod stubs {
    use libc::{c_char, c_float, c_int, c_void, size_t};

    // ============================================================
    // LLAMA TYPES (stubs - will be replaced by bindgen)
    // ============================================================

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_model {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_context {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_sampler {
        _unused: [u8; 0],
    }

    pub type llama_token = i32;
    pub type llama_pos = i32;
    pub type llama_seq_id = i32;

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct llama_model_params {
        pub n_gpu_layers: i32,
        pub split_mode: i32,
        pub main_gpu: i32,
        pub vocab_only: bool,
        pub use_mmap: bool,
        pub use_mlock: bool,
        pub check_tensors: bool,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct llama_context_params {
        pub n_ctx: u32,
        pub n_batch: u32,
        pub n_ubatch: u32,
        pub n_seq_max: u32,
        pub n_threads: i32,
        pub n_threads_batch: i32,
        pub flash_attn: bool,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_batch {
        pub n_tokens: i32,
        pub token: *mut llama_token,
        pub embd: *mut c_float,
        pub pos: *mut llama_pos,
        pub n_seq_id: *mut i32,
        pub seq_id: *mut *mut llama_seq_id,
        pub logits: *mut i8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct llama_token_data {
        pub id: llama_token,
        pub logit: c_float,
        pub p: c_float,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_token_data_array {
        pub data: *mut llama_token_data,
        pub size: size_t,
        pub selected: i64,
        pub sorted: bool,
    }

    // ============================================================
    // LLAMA FUNCTIONS (stubs)
    // ============================================================

    extern "C" {
        // Backend
        pub fn llama_backend_init();
        pub fn llama_backend_free();

        // Model
        pub fn llama_model_default_params() -> llama_model_params;
        pub fn llama_load_model_from_file(
            path_model: *const c_char,
            params: llama_model_params,
        ) -> *mut llama_model;
        pub fn llama_free_model(model: *mut llama_model);
        pub fn llama_n_vocab(model: *const llama_model) -> i32;
        pub fn llama_n_ctx_train(model: *const llama_model) -> i32;

        // Context
        pub fn llama_context_default_params() -> llama_context_params;
        pub fn llama_new_context_with_model(
            model: *mut llama_model,
            params: llama_context_params,
        ) -> *mut llama_context;
        pub fn llama_free(ctx: *mut llama_context);
        pub fn llama_n_ctx(ctx: *const llama_context) -> u32;
        pub fn llama_get_model(ctx: *const llama_context) -> *const llama_model;

        // Batch
        pub fn llama_batch_init(n_tokens: i32, embd: i32, n_seq_max: i32) -> llama_batch;
        pub fn llama_batch_free(batch: llama_batch);

        // Decode
        pub fn llama_decode(ctx: *mut llama_context, batch: llama_batch) -> i32;
        pub fn llama_get_logits(ctx: *mut llama_context) -> *mut c_float;
        pub fn llama_get_logits_ith(ctx: *mut llama_context, i: i32) -> *mut c_float;

        // Tokens
        pub fn llama_token_bos(model: *const llama_model) -> llama_token;
        pub fn llama_token_eos(model: *const llama_model) -> llama_token;
        pub fn llama_token_eot(model: *const llama_model) -> llama_token;
        pub fn llama_token_is_eog(model: *const llama_model, token: llama_token) -> bool;
        pub fn llama_token_to_piece(
            model: *const llama_model,
            token: llama_token,
            buf: *mut c_char,
            length: i32,
            lstrip: i32,
            special: bool,
        ) -> i32;
        pub fn llama_tokenize(
            model: *const llama_model,
            text: *const c_char,
            text_len: i32,
            tokens: *mut llama_token,
            n_tokens_max: i32,
            add_special: bool,
            parse_special: bool,
        ) -> i32;

        // Memory management (newer API - replaces kv_cache_*)
        // llama_memory_t is an opaque pointer to the memory/KV cache
        pub fn llama_get_memory(ctx: *const llama_context) -> *mut c_void;
        pub fn llama_memory_clear(mem: *mut c_void, data: bool);

        // Remove tokens in range [p0, p1) for a sequence
        // Returns false if partial removal not supported
        pub fn llama_memory_seq_rm(
            mem: *mut c_void,
            seq_id: llama_seq_id,
            p0: llama_pos,
            p1: llama_pos,
        ) -> bool;

        pub fn llama_memory_seq_cp(
            mem: *mut c_void,
            seq_id_src: llama_seq_id,
            seq_id_dst: llama_seq_id,
            p0: llama_pos,
            p1: llama_pos,
        );

        // Shift token positions by delta for tokens in [p0, p1)
        pub fn llama_memory_seq_add(
            mem: *mut c_void,
            seq_id: llama_seq_id,
            p0: llama_pos,
            p1: llama_pos,
            delta: llama_pos,
        );

        // Check if memory supports shifting (context sliding)
        pub fn llama_memory_can_shift(mem: *mut c_void) -> bool;

        // Sampling Chain
        pub fn llama_sampler_chain_default_params() -> llama_sampler_chain_params;
        pub fn llama_sampler_chain_init(params: llama_sampler_chain_params) -> *mut llama_sampler;
        pub fn llama_sampler_chain_add(chain: *mut llama_sampler, smpl: *mut llama_sampler);
        pub fn llama_sampler_chain_n(chain: *const llama_sampler) -> i32;

        // Individual Samplers
        pub fn llama_sampler_init_greedy() -> *mut llama_sampler;
        pub fn llama_sampler_init_dist(seed: u32) -> *mut llama_sampler;
        pub fn llama_sampler_init_temp(t: c_float) -> *mut llama_sampler;
        pub fn llama_sampler_init_top_k(k: i32) -> *mut llama_sampler;
        pub fn llama_sampler_init_top_p(p: c_float, min_keep: size_t) -> *mut llama_sampler;
        pub fn llama_sampler_init_min_p(p: c_float, min_keep: size_t) -> *mut llama_sampler;
        pub fn llama_sampler_init_typical(p: c_float, min_keep: size_t) -> *mut llama_sampler;
        pub fn llama_sampler_init_penalties(
            penalty_last_n: i32,
            penalty_repeat: c_float,
            penalty_freq: c_float,
            penalty_present: c_float,
        ) -> *mut llama_sampler;
        pub fn llama_sampler_init_dry(
            vocab: *const llama_vocab,
            n_vocab: i32,
            dry_multiplier: c_float,
            dry_base: c_float,
            dry_allowed_length: i32,
            dry_penalty_last_n: i32,
            seq_breakers: *const *const c_char,
            num_breakers: size_t,
        ) -> *mut llama_sampler;
        pub fn llama_sampler_init_logit_bias(
            n_vocab: i32,
            n_logit_bias: i32,
            logit_bias: *const llama_logit_bias,
        ) -> *mut llama_sampler;

        // Sampler operations
        pub fn llama_sampler_free(smpl: *mut llama_sampler);
        pub fn llama_sampler_reset(smpl: *mut llama_sampler);
        pub fn llama_sampler_accept(smpl: *mut llama_sampler, token: llama_token);
        pub fn llama_sampler_sample(
            smpl: *mut llama_sampler,
            ctx: *mut llama_context,
            idx: i32,
        ) -> llama_token;

        // Vocab
        pub fn llama_model_get_vocab(model: *const llama_model) -> *const llama_vocab;
        pub fn llama_vocab_n_tokens(vocab: *const llama_vocab) -> i32;
    }

    // Additional types for sampling
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct llama_vocab {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct llama_sampler_chain_params {
        pub no_perf: bool,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct llama_logit_bias {
        pub token: llama_token,
        pub bias: c_float,
    }

    // ============================================================
    // MTMD TYPES (multimodal stubs)
    // ============================================================

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct mtmd_context {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct mtmd_input_chunks {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct mtmd_bitmap {
        _unused: [u8; 0],
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone, Default)]
    pub struct mtmd_context_params {
        pub use_gpu: bool,
        pub print_timings: bool,
        pub n_threads: i32,
        pub verbosity: i32,
        /// Whether to warm up the model (compile Metal pipelines, etc.)
        pub warmup: bool,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct mtmd_input_text {
        pub text: *const c_char,
        pub add_special: bool,
        pub parse_special: bool,
    }

    impl Default for mtmd_input_text {
        fn default() -> Self {
            Self {
                text: std::ptr::null(),
                add_special: true,
                parse_special: true,
            }
        }
    }

    extern "C" {
        // MTMD Context
        pub fn mtmd_context_params_default() -> mtmd_context_params;
        pub fn mtmd_init_from_file(
            path: *const c_char,
            model: *mut llama_model,
            params: mtmd_context_params,
        ) -> *mut mtmd_context;
        pub fn mtmd_free(ctx: *mut mtmd_context);

        /// Check if the multimodal context supports vision
        pub fn mtmd_support_vision(ctx: *const mtmd_context) -> bool;

        /// Get the default media marker string (e.g., "<image>" or "<__media__>")
        pub fn mtmd_default_marker() -> *const c_char;

        // MTMD Input
        pub fn mtmd_input_chunks_init() -> *mut mtmd_input_chunks;
        pub fn mtmd_input_chunks_free(chunks: *mut mtmd_input_chunks);

        /// Get the number of chunks after tokenization
        pub fn mtmd_input_chunks_size(chunks: *const mtmd_input_chunks) -> size_t;

        /// Get total token count from chunks
        pub fn mtmd_helper_get_n_tokens(chunks: *const mtmd_input_chunks) -> i32;

        // MTMD Tokenize
        pub fn mtmd_tokenize(
            ctx: *mut mtmd_context,
            output: *mut mtmd_input_chunks,
            text: *const mtmd_input_text,
            bitmaps: *mut *const mtmd_bitmap,
            n_bitmaps: size_t,
        ) -> i32;

        // MTMD Bitmap
        pub fn mtmd_bitmap_init(nx: i32, ny: i32, data: *const u8) -> *mut mtmd_bitmap;
        pub fn mtmd_bitmap_free(bmp: *mut mtmd_bitmap);

        /// Helper to load bitmap from memory buffer
        pub fn mtmd_helper_bitmap_init_from_buf(
            ctx: *const mtmd_context,
            data: *const u8,
            len: size_t,
        ) -> *mut mtmd_bitmap;

        /// Helper to load bitmap from file with context
        pub fn mtmd_helper_bitmap_init_from_file(
            ctx: *const mtmd_context,
            path: *const c_char,
        ) -> *mut mtmd_bitmap;

        /// Get bitmap width
        pub fn mtmd_bitmap_get_nx(bmp: *const mtmd_bitmap) -> i32;

        /// Get bitmap height
        pub fn mtmd_bitmap_get_ny(bmp: *const mtmd_bitmap) -> i32;

        // MTMD Eval
        pub fn mtmd_helper_eval_chunks(
            mctx: *mut mtmd_context,
            lctx: *mut llama_context,
            chunks: *mut mtmd_input_chunks,
            n_past: i32,
            seq_id: i32,
            n_batch: i32,
            logits_last: bool,
            new_n_past: *mut i32,
        ) -> i32;
    }
}

#[cfg(not(feature = "generate"))]
pub use stubs::*;

// Re-export libc types for convenience
pub use libc::{c_char, c_float, c_int, c_void, size_t};
