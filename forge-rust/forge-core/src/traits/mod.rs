//! Traits: Universal interfaces for multimodal pipelines
//!
//! This module defines the core abstractions that allow different
//! media types (audio, video, etc.) to be plugged into the SDK.

pub mod decoder;
pub mod encoder;

// Re-export main traits
pub use decoder::{AudioFormat, Decoder};
pub use encoder::{EncoderCapabilities, MediaEncoder, MediaMarker};
