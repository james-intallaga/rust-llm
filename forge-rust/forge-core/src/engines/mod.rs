//! Engines: High-level inference orchestration
//!
//! This module contains engine implementations for different modalities.
//! Currently supports text and vision. Audio coming soon.

pub mod engine;

// Re-export main types
pub use engine::{ForgeConfig, ForgeEngine};
