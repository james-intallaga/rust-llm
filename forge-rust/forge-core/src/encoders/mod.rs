//! Encoders: Convert media inputs to embeddings
//!
//! This module contains encoder implementations for different media types.

pub mod mtmd_encoder;

// Re-export main types for convenience
pub use mtmd_encoder::{
    get_default_media_marker, InputChunks, MultimodalContext, MultimodalParams,
};
