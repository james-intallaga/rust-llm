//! Model loading and management with RAII

use std::ffi::CString;
use std::path::Path;
use std::ptr::NonNull;

use llama_cpp_sys::{
    llama_free_model, llama_load_model_from_file, llama_model, llama_model_default_params,
    llama_model_get_vocab, llama_model_n_ctx_train, llama_model_params, llama_model_size,
    llama_vocab_bos, llama_vocab_eos, llama_vocab_eot, llama_vocab_n_tokens,
};

use crate::error::{ForgeError, Result};

/// Parameters for loading a model
#[derive(Debug, Clone)]
pub struct ModelParams {
    /// Number of layers to offload to GPU (0 = CPU only, -1 = all)
    pub n_gpu_layers: i32,
    /// Use memory mapping for model file
    pub use_mmap: bool,
    /// Lock model in memory
    pub use_mlock: bool,
    /// Only load vocabulary (for tokenization)
    pub vocab_only: bool,
}

impl Default for ModelParams {
    fn default() -> Self {
        Self {
            n_gpu_layers: -1, // All layers on GPU by default
            use_mmap: true,
            use_mlock: false,
            vocab_only: false,
        }
    }
}

impl From<ModelParams> for llama_model_params {
    fn from(params: ModelParams) -> Self {
        let mut c_params = unsafe { llama_model_default_params() };
        c_params.n_gpu_layers = params.n_gpu_layers;
        c_params.use_mmap = params.use_mmap;
        c_params.use_mlock = params.use_mlock;
        c_params.vocab_only = params.vocab_only;
        c_params
    }
}

/// A loaded LLaMA model
///
/// This struct wraps the raw `llama_model` pointer and ensures
/// it is properly freed when dropped (RAII pattern).
pub struct LlamaModel {
    ptr: NonNull<llama_model>,
    path: String,
}

// Safety: LlamaModel owns the pointer and doesn't share it mutably
unsafe impl Send for LlamaModel {}
unsafe impl Sync for LlamaModel {}

impl LlamaModel {
    /// Load a model from a GGUF file
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the GGUF model file
    /// * `params` - Model loading parameters
    ///
    /// # Returns
    ///
    /// A loaded model ready for context creation
    pub fn load<P: AsRef<Path>>(path: P, params: ModelParams) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let c_path = CString::new(path_str.clone())?;

        log::info!("Loading model from: {}", path_str);

        let ptr = unsafe { llama_load_model_from_file(c_path.as_ptr(), params.into()) };

        let ptr = NonNull::new(ptr).ok_or_else(|| ForgeError::ModelLoadFailed {
            path: path_str.clone(),
            reason: "llama_load_model_from_file returned null".to_string(),
        })?;

        let vocab = unsafe { llama_model_get_vocab(ptr.as_ptr()) };
        log::info!(
            "Model loaded successfully: {} vocab, {} ctx_train",
            unsafe { llama_vocab_n_tokens(vocab) },
            unsafe { llama_model_n_ctx_train(ptr.as_ptr()) }
        );

        Ok(Self {
            ptr,
            path: path_str,
        })
    }

    /// Get the raw pointer (for use with other llama.cpp functions)
    ///
    /// # Safety
    ///
    /// The returned pointer is valid only while this LlamaModel exists.
    pub fn as_ptr(&self) -> *mut llama_model {
        self.ptr.as_ptr()
    }

    /// Get the vocabulary from the model
    fn get_vocab(&self) -> *const llama_cpp_sys::llama_vocab {
        unsafe { llama_model_get_vocab(self.ptr.as_ptr()) }
    }

    /// Get the vocabulary size
    pub fn n_vocab(&self) -> i32 {
        unsafe { llama_vocab_n_tokens(self.get_vocab()) }
    }

    /// Get the training context size
    pub fn n_ctx_train(&self) -> i32 {
        unsafe { llama_model_n_ctx_train(self.ptr.as_ptr()) }
    }

    /// Total bytes occupied by model tensors across backends.
    pub fn tensor_bytes(&self) -> usize {
        unsafe { llama_model_size(self.ptr.as_ptr()) as usize }
    }

    /// Get the beginning-of-sequence token
    pub fn token_bos(&self) -> i32 {
        unsafe { llama_vocab_bos(self.get_vocab()) }
    }

    /// Get the end-of-sequence token
    pub fn token_eos(&self) -> i32 {
        unsafe { llama_vocab_eos(self.get_vocab()) }
    }

    /// Get the end-of-turn token
    pub fn token_eot(&self) -> i32 {
        unsafe { llama_vocab_eot(self.get_vocab()) }
    }

    /// Get the model file path
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for LlamaModel {
    fn drop(&mut self) {
        log::debug!("Freeing model: {}", self.path);
        unsafe {
            llama_free_model(self.ptr.as_ptr());
        }
        log::debug!("Model freed successfully");
    }
}
