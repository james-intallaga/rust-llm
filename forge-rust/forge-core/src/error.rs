//! Error types for forge-core

use thiserror::Error;

/// Result type for forge-core operations
pub type Result<T> = std::result::Result<T, ForgeError>;

/// Errors that can occur during inference operations
#[derive(Error, Debug)]
pub enum ForgeError {
    /// Failed to load the model file
    #[error("Failed to load model from '{path}': {reason}")]
    ModelLoadFailed { path: String, reason: String },

    /// Failed to create inference context
    #[error("Failed to create context: {0}")]
    ContextCreationFailed(String),

    /// Failed to load CLIP/multimodal model
    #[error("Failed to load multimodal model from '{path}': {reason}")]
    MultimodalLoadFailed { path: String, reason: String },

    /// Memory budget would be exceeded
    #[error("Operation would exceed memory budget: requested {requested_mb} MB, available {available_mb} MB")]
    MemoryBudgetExceeded {
        requested_mb: usize,
        available_mb: usize,
    },

    /// Tokenization failed
    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),

    /// Decode/inference failed
    #[error("Decode failed with error code {0}")]
    DecodeFailed(i32),

    /// Generation was cancelled by the caller
    #[error("Generation cancelled")]
    Cancelled,

    /// Image processing failed
    #[error("Image processing failed: {0}")]
    ImageProcessingFailed(String),

    /// Audio processing failed
    #[error("Audio processing failed: {0}")]
    AudioProcessingFailed(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Null pointer returned from C API
    #[error("Null pointer returned from llama.cpp: {context}")]
    NullPointer { context: String },

    /// C string conversion error
    #[error("C string conversion failed: {0}")]
    CStringError(#[from] std::ffi::NulError),

    /// UTF-8 conversion error
    #[error("UTF-8 conversion failed: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
