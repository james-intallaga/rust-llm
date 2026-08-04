//! forge-core: Safe Rust wrapper for llama.cpp
//!
//! This crate provides a memory-safe, RAII-based API for running LLM inference
//! using llama.cpp. All resources are automatically cleaned up when dropped.
//!
//! # Key Features
//!
//! - **RAII Memory Management**: All C resources wrapped with `Drop` trait
//! - **Arena Allocators**: Efficient batch memory reuse
//! - **Streaming Iterators**: Memory-efficient token generation
//! - **Memory Budgets**: Preflight limits for model and multimodal weights
//!
//! # Example
//!
//! ```ignore
//! use forge_core::{ForgeEngine, ForgeConfig};
//!
//! let config = ForgeConfig::default()
//!     .with_context_size(2048)
//!     .with_batch_size(128);
//!
//! let engine = ForgeEngine::new("model.gguf", config)?;
//!
//! for token in engine.generate("Hello, world!")? {
//!     print!("{}", token);
//! }
//! ```

pub mod core;
pub mod decoders;
pub mod encoders;
pub mod engines;
pub mod error;
pub mod ffi;
pub mod traits;
pub mod types;

// Backwards compatibility: re-export core modules at crate root
// This allows existing code using `forge_core::model` to still work
pub use core::batch;
pub use core::context;
pub use core::memory;
pub use core::model;
pub use core::sampler;

// Backwards compatibility: re-export multimodal at crate root
pub use encoders::mtmd_encoder as multimodal;

// Backwards compatibility: re-export engine at crate root
pub use engines::engine;

// Re-exports of main types
pub use core::BatchArena;
pub use core::MemoryTracker;
pub use core::{ContextParams, LlamaContext};
pub use core::{LlamaModel, ModelParams};
pub use core::{Sampler, SamplerParams};
pub use encoders::{get_default_media_marker, InputChunks, MultimodalContext, MultimodalParams};
pub use engines::{ForgeConfig, ForgeEngine};
pub use error::{ForgeError, Result};

use std::sync::{Mutex, OnceLock};

fn backend_refcount() -> &'static Mutex<usize> {
    static REFCOUNT: OnceLock<Mutex<usize>> = OnceLock::new();
    REFCOUNT.get_or_init(|| Mutex::new(0))
}

/// Initialize the llama.cpp backend.
/// Must be called once before using any other functions.
pub fn init() {
    let mut count = backend_refcount()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *count == 0 {
        unsafe {
            llama_cpp_sys::llama_backend_init();
        }
        log::info!("forge-core: llama backend initialized");
    }
    *count += 1;
}

/// Cleanup the llama.cpp backend.
/// Should be called when completely done with inference.
pub fn cleanup() {
    let mut count = backend_refcount()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if *count == 0 {
        log::warn!("forge-core: cleanup called without matching init");
        return;
    }
    *count -= 1;
    if *count == 0 {
        unsafe {
            llama_cpp_sys::llama_backend_free();
        }
        log::info!("forge-core: llama backend freed");
    }
}
