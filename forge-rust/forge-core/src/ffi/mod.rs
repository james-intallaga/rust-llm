//! FFI: C-compatible interface for Swift/Kotlin
//!
//! This module provides the C FFI interface that mobile apps use.
//! All functions here are `extern "C"` and use raw pointers.

#[allow(clippy::module_inception)]
mod ffi;

// Re-export all FFI types and functions
pub use ffi::*;
