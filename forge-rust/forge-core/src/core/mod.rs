//! Core components: llama.cpp wrappers with RAII
//!
//! These are the fundamental building blocks that wrap llama.cpp C types.

pub mod batch;
pub mod context;
pub mod memory;
pub mod model;
pub mod sampler;

// Re-export main types
pub use batch::BatchArena;
pub use context::{ContextParams, LlamaContext};
pub use memory::MemoryTracker;
pub use model::{LlamaModel, ModelParams};
pub use sampler::{Sampler, SamplerParams};
