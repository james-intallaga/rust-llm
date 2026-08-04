#![allow(clippy::not_unsafe_ptr_arg_deref)]

//! C FFI interface for Swift interop
//!
//! This module exposes a C-compatible API that can be called from Swift.
//! All functions follow the pattern:
//!
//! - Opaque handles for complex types
//! - Error codes or null pointers for errors
//! - Callbacks for streaming output

use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::engine::{ForgeConfig, ForgeEngine};

const MAX_CONTEXT_TOKENS: u32 = 1_048_576;
const MAX_BATCH_TOKENS: u32 = 65_536;
const MAX_GENERATED_TOKENS: u32 = 1_048_576;
const MAX_THREADS: i32 = 1_024;
const MAX_TOP_K: i32 = 1_000_000;
const MAX_GPU_LAYERS: i32 = 1_000_000;
const MAX_TEMPERATURE: f32 = 10.0;
const MAX_ENCODED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_AUDIO_EMBEDDING_VALUES: usize = 1_048_576;
const MAX_AUDIO_SAMPLES: usize = 48_000 * 60 * 10;

/// Opaque type for the Forge engine (C representation)
/// This is never constructed directly - only through pointers
#[repr(C)]
pub struct ForgeEngineOpaque {
    _private: [u8; 0],
}

/// Opaque handle to the Forge engine
pub type ForgeHandle = *mut ForgeEngineOpaque;

struct ForgeEngineState {
    engine: Mutex<ForgeEngine>,
    cancellation: Arc<AtomicBool>,
}

// Helper functions to convert between opaque handle and actual engine
impl ForgeEngineOpaque {
    /// Convert an opaque handle to a reference to the actual engine
    ///
    /// # Safety
    ///
    /// The handle must be non-null and point to a valid ForgeEngine.
    #[inline]
    unsafe fn as_engine<'a>(handle: ForgeHandle) -> Option<MutexGuard<'a, ForgeEngine>> {
        if handle.is_null() {
            None
        } else {
            let state = &*(handle as *const ForgeEngineState);
            Some(
                state
                    .engine
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            )
        }
    }

    /// Convert an opaque handle to a mutable reference to the actual engine
    ///
    /// # Safety
    ///
    /// The handle must be non-null and point to a valid ForgeEngine.
    #[inline]
    unsafe fn as_engine_mut<'a>(handle: ForgeHandle) -> Option<MutexGuard<'a, ForgeEngine>> {
        Self::as_engine(handle)
    }

    /// Create an opaque handle from a boxed engine
    #[inline]
    fn from_engine(engine: ForgeEngine) -> ForgeHandle {
        let cancellation = engine.cancellation_handle();
        Box::into_raw(Box::new(ForgeEngineState {
            engine: Mutex::new(engine),
            cancellation,
        })) as ForgeHandle
    }

    /// Free the engine behind an opaque handle
    ///
    /// # Safety
    ///
    /// The handle must be non-null and point to a valid ForgeEngine.
    #[inline]
    unsafe fn destroy(handle: ForgeHandle) {
        if !handle.is_null() {
            let state = Box::from_raw(handle as *mut ForgeEngineState);
            state.cancellation.store(true, Ordering::Release);
            // Wait for any active operation to leave the engine before freeing
            // the mutex and its protected native resources.
            let guard = state
                .engine
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            drop(guard);
            drop(state);
            crate::cleanup();
        }
    }
}

/// Callback type for streaming token output
pub type TokenCallback = extern "C" fn(token: *const c_char, user_data: *mut c_void);

/// Error codes returned by FFI functions
#[repr(C)]
pub enum ForgeResult {
    /// Success
    Ok = 0,
    /// Null pointer passed
    NullPointer = 1,
    /// Model load failed
    ModelLoadFailed = 2,
    /// Context creation failed
    ContextCreationFailed = 3,
    /// Multimodal load failed
    MultimodalLoadFailed = 4,
    /// Tokenization failed
    TokenizationFailed = 5,
    /// Decode failed
    DecodeFailed = 6,
    /// Invalid parameter
    InvalidParameter = 7,
    /// Memory budget exceeded
    MemoryExceeded = 8,
    /// Generation was cancelled by the caller
    Cancelled = 9,
    /// Unknown error
    Unknown = 99,
}

/// Configuration parameters passed from Swift
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ForgeParams {
    /// Context size
    pub n_ctx: u32,
    /// Batch size
    pub n_batch: u32,
    /// Number of threads
    pub n_threads: i32,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Temperature (0.0 = greedy)
    pub temperature: f32,
    /// Top-K sampling
    pub top_k: i32,
    /// Top-P sampling
    pub top_p: f32,
    /// Memory budget in MB (0 = unlimited)
    pub memory_budget_mb: u32,
    /// Enable flash attention
    pub flash_attn: bool,
    /// Number of GPU layers (-1 = all)
    pub n_gpu_layers: i32,
}

impl Default for ForgeParams {
    fn default() -> Self {
        Self {
            n_ctx: 2048,
            n_batch: 128,
            n_threads: 2,
            max_tokens: 128,
            temperature: 0.3,
            top_k: 40,
            top_p: 0.95,
            memory_budget_mb: 0,
            flash_attn: true,
            n_gpu_layers: -1,
        }
    }
}

impl From<ForgeParams> for ForgeConfig {
    fn from(params: ForgeParams) -> Self {
        let mut config = ForgeConfig::default()
            .with_context_size(params.n_ctx)
            .with_batch_size(params.n_batch)
            .with_max_tokens(params.max_tokens)
            .with_temperature(params.temperature)
            .with_memory_budget(params.memory_budget_mb as usize);
        config.context.n_ubatch = params.n_batch;
        config.context.n_threads = params.n_threads;
        config.context.n_threads_batch = params.n_threads;
        config.context.flash_attention = params.flash_attn;
        config.sampler.top_k = params.top_k;
        config.sampler.top_p = params.top_p;
        config.model.n_gpu_layers = params.n_gpu_layers;
        // Keep multimodal offload aligned with the main model. This is especially
        // important in the iOS Simulator, where large Metal shared buffers used
        // by vision projectors are not supported reliably.
        config.multimodal.use_gpu = params.n_gpu_layers != 0;
        config
    }
}

impl ForgeParams {
    fn validate(self) -> bool {
        (1..=MAX_CONTEXT_TOKENS).contains(&self.n_ctx)
            && (1..=MAX_BATCH_TOKENS).contains(&self.n_batch)
            && (1..=MAX_THREADS).contains(&self.n_threads)
            && (1..=MAX_GENERATED_TOKENS).contains(&self.max_tokens)
            && self.temperature.is_finite()
            && (0.0..=MAX_TEMPERATURE).contains(&self.temperature)
            && (0..=MAX_TOP_K).contains(&self.top_k)
            && self.top_p.is_finite()
            && (0.0..=1.0).contains(&self.top_p)
            && (-1..=MAX_GPU_LAYERS).contains(&self.n_gpu_layers)
    }
}

#[cfg(test)]
mod parameter_validation_tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        assert!(ForgeParams::default().validate());
    }

    #[test]
    fn rejects_values_that_would_wrap_or_exhaust_resources() {
        let params = ForgeParams {
            n_batch: u32::MAX,
            ..ForgeParams::default()
        };
        assert!(!params.validate());

        let params = ForgeParams {
            temperature: f32::NAN,
            ..ForgeParams::default()
        };
        assert!(!params.validate());
    }
}

// ============================================================
// LIFECYCLE FUNCTIONS
// ============================================================

/// Initialize the Forge backend. Must be called once before any other functions.
#[no_mangle]
pub extern "C" fn forge_init() {
    crate::init();
}

/// Cleanup the Forge backend. Call when completely done with inference.
#[no_mangle]
pub extern "C" fn forge_cleanup() {
    crate::cleanup();
}

/// Get default parameters
#[no_mangle]
pub extern "C" fn forge_params_default() -> ForgeParams {
    ForgeParams::default()
}

/// Get the default media marker string for image prompts
///
/// Returns a pointer to a static string like "<image>" or "<__media__>".
/// The returned string is valid until the next call to this function.
///
/// # Example
///
/// ```c
/// const char* marker = forge_get_media_marker();
/// // Use marker in prompt: "<|im_start|>user\n{marker}\nWhat is this?\n<|im_end|>"
/// ```
#[no_mangle]
pub extern "C" fn forge_get_media_marker() -> *const c_char {
    // Use a thread-local to store the CString
    thread_local! {
        static MARKER: std::cell::RefCell<CString> = std::cell::RefCell::new(
            CString::new(crate::get_default_media_marker()).unwrap_or_else(|_| CString::new("<image>").unwrap())
        );
    }

    MARKER.with(|m| m.borrow().as_ptr())
}

// ============================================================
// ENGINE FUNCTIONS
// ============================================================

/// Create a new Forge engine for text-only inference
///
/// # Arguments
///
/// * `model_path` - Path to the GGUF model file (null-terminated)
/// * `params` - Configuration parameters
///
/// # Returns
///
/// Handle to the engine, or NULL on failure
#[no_mangle]
pub extern "C" fn forge_engine_create(
    model_path: *const c_char,
    params: *const ForgeParams,
) -> ForgeHandle {
    if model_path.is_null() || params.is_null() {
        log::error!("forge_engine_create: null pointer");
        return ptr::null_mut();
    }

    let path = match unsafe { CStr::from_ptr(model_path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            log::error!("forge_engine_create: invalid UTF-8 in path");
            return ptr::null_mut();
        }
    };

    let params = unsafe { *params };
    if !params.validate() {
        log::error!("forge_engine_create: invalid parameters: {:?}", params);
        return ptr::null_mut();
    }
    let config: ForgeConfig = params.into();

    // Each engine owns a backend reference, so an early global cleanup cannot
    // invalidate a live engine and callers are not required to initialize first.
    crate::init();
    match ForgeEngine::new(path, config) {
        Ok(engine) => ForgeEngineOpaque::from_engine(engine),
        Err(e) => {
            crate::cleanup();
            log::error!("forge_engine_create failed: {}", e);
            ptr::null_mut()
        }
    }
}

/// Create a new Forge engine with vision capabilities
///
/// # Arguments
///
/// * `model_path` - Path to the GGUF model file
/// * `clip_path` - Path to the CLIP model file (mmproj-*.gguf)
/// * `params` - Configuration parameters
///
/// # Returns
///
/// Handle to the engine, or NULL on failure
#[no_mangle]
pub extern "C" fn forge_engine_create_vision(
    model_path: *const c_char,
    clip_path: *const c_char,
    params: *const ForgeParams,
) -> ForgeHandle {
    if model_path.is_null() || clip_path.is_null() || params.is_null() {
        log::error!("forge_engine_create_vision: null pointer");
        return ptr::null_mut();
    }

    let model_path = match unsafe { CStr::from_ptr(model_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let clip_path = match unsafe { CStr::from_ptr(clip_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let params = unsafe { *params };
    if !params.validate() {
        log::error!(
            "forge_engine_create_vision: invalid parameters: {:?}",
            params
        );
        return ptr::null_mut();
    }
    let config: ForgeConfig = params.into();

    crate::init();
    match ForgeEngine::with_vision(model_path, clip_path, config) {
        Ok(engine) => ForgeEngineOpaque::from_engine(engine),
        Err(e) => {
            crate::cleanup();
            log::error!("forge_engine_create_vision failed: {}", e);
            ptr::null_mut()
        }
    }
}

/// Destroy a Forge engine and free all resources
///
/// # Safety
///
/// The handle must have been created by forge_engine_create or forge_engine_create_vision.
/// After calling this, the handle is invalid and must not be used.
#[no_mangle]
pub extern "C" fn forge_engine_destroy(engine: ForgeHandle) {
    unsafe {
        ForgeEngineOpaque::destroy(engine);
    }
    log::debug!("Engine destroyed");
}

/// Request cancellation of the active generation without waiting for the engine lock.
/// Safe to call from a different thread while a generation callback is running.
#[no_mangle]
pub extern "C" fn forge_cancel(engine: ForgeHandle) -> ForgeResult {
    if engine.is_null() {
        return ForgeResult::NullPointer;
    }
    let state = unsafe { &*(engine as *const ForgeEngineState) };
    state.cancellation.store(true, Ordering::Release);
    ForgeResult::Ok
}

// ============================================================
// GENERATION FUNCTIONS
// ============================================================

/// Generate text from a prompt (single-turn, clears context)
///
/// # Arguments
///
/// * `engine` - Engine handle
/// * `prompt` - Input prompt (null-terminated)
/// * `callback` - Called for each generated token
/// * `user_data` - Passed to callback
///
/// # Returns
///
/// ForgeResult::Ok on success, error code on failure
#[no_mangle]
pub extern "C" fn forge_generate(
    engine: ForgeHandle,
    prompt: *const c_char,
    callback: TokenCallback,
    user_data: *mut c_void,
) -> ForgeResult {
    if prompt.is_null() {
        return ForgeResult::NullPointer;
    }

    let mut engine = match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => e,
        None => return ForgeResult::NullPointer,
    };
    let prompt = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(_) => return ForgeResult::InvalidParameter,
    };

    let result = engine.generate(prompt, |token| {
        if let Ok(c_token) = CString::new(token) {
            callback(c_token.as_ptr(), user_data);
        }
    });

    match result {
        Ok(_) => ForgeResult::Ok,
        Err(crate::ForgeError::TokenizationFailed(_)) => ForgeResult::TokenizationFailed,
        Err(crate::ForgeError::DecodeFailed(_)) => ForgeResult::DecodeFailed,
        Err(crate::ForgeError::Cancelled) => ForgeResult::Cancelled,
        Err(_) => ForgeResult::Unknown,
    }
}

/// Generate a response for a conversation turn (multi-turn aware)
///
/// This properly handles:
/// - BOS token: Only added when add_bos is true (should be first turn only)
/// - EOG feedback: Previous EOG token is fed back to model
/// - Context accumulation: Maintains conversation history
///
/// # Arguments
///
/// * `engine` - Engine handle
/// * `prompt` - Formatted prompt for this turn (null-terminated)
/// * `add_bos` - Whether to add BOS token (true for first turn only)
/// * `callback` - Called for each generated token
/// * `user_data` - Passed to callback
///
/// # Example Usage
///
/// ```c
/// // First turn - add BOS
/// forge_generate_turn(engine, "<|im_start|>user\nHello\n<|im_end|><|im_start|>assistant\n",
///                     true, callback, user_data);
///
/// // Second turn - no BOS, context preserved
/// forge_generate_turn(engine, "<|im_start|>user\nHow are you?\n<|im_end|><|im_start|>assistant\n",
///                     false, callback, user_data);
/// ```
#[no_mangle]
pub extern "C" fn forge_generate_turn(
    engine: ForgeHandle,
    prompt: *const c_char,
    add_bos: bool,
    callback: TokenCallback,
    user_data: *mut c_void,
) -> ForgeResult {
    if prompt.is_null() {
        return ForgeResult::NullPointer;
    }

    let mut engine = match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => e,
        None => return ForgeResult::NullPointer,
    };
    let prompt = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(_) => return ForgeResult::InvalidParameter,
    };

    let result = engine.generate_turn(prompt, add_bos, |token| {
        if let Ok(c_token) = CString::new(token) {
            callback(c_token.as_ptr(), user_data);
        }
    });

    match result {
        Ok(_) => ForgeResult::Ok,
        Err(crate::ForgeError::TokenizationFailed(_)) => ForgeResult::TokenizationFailed,
        Err(crate::ForgeError::DecodeFailed(_)) => ForgeResult::DecodeFailed,
        Err(crate::ForgeError::Cancelled) => ForgeResult::Cancelled,
        Err(_) => ForgeResult::Unknown,
    }
}

/// Check if this is the first turn (n_past == 0)
///
/// Use this to determine whether to add BOS token in chat template
#[no_mangle]
pub extern "C" fn forge_is_first_turn(engine: ForgeHandle) -> bool {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.is_first_turn(),
        None => true,
    }
}

/// Get the current context position (tokens processed so far)
#[no_mangle]
pub extern "C" fn forge_n_past(engine: ForgeHandle) -> i32 {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.n_past(),
        None => 0,
    }
}

/// Set the number of tokens to preserve when context sliding occurs
///
/// When the context fills up, the engine will automatically shift the context
/// to make room for new tokens. This setting controls how many tokens at the
/// beginning are preserved (typically the system prompt).
///
/// # Arguments
///
/// * `engine` - Engine handle
/// * `n_keep` - Number of tokens to keep (0 = don't preserve any)
#[no_mangle]
pub extern "C" fn forge_set_n_keep(engine: ForgeHandle, n_keep: i32) {
    if let Some(mut e) = unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        e.set_n_keep(n_keep);
    }
}

/// Get the current n_keep setting
#[no_mangle]
pub extern "C" fn forge_get_n_keep(engine: ForgeHandle) -> i32 {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.n_keep(),
        None => 0,
    }
}

/// Check if context sliding is supported
///
/// Returns true if the engine's memory supports shifting (context sliding).
/// Most models support this, but some specialized models may not.
#[no_mangle]
pub extern "C" fn forge_can_shift_context(engine: ForgeHandle) -> bool {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.can_shift_context(),
        None => false,
    }
}

/// Generate text from an image (JPEG/PNG) and prompt
///
/// # Arguments
///
/// * `engine` - Engine handle (must have vision enabled)
/// * `image_data` - Encoded image data (JPEG, PNG, etc.)
/// * `image_len` - Length of image data
/// * `prompt` - Text prompt (null-terminated, should contain image marker)
/// * `callback` - Called for each generated token
/// * `user_data` - Passed to callback
#[no_mangle]
pub extern "C" fn forge_generate_vision(
    engine: ForgeHandle,
    image_data: *const u8,
    image_len: usize,
    prompt: *const c_char,
    callback: TokenCallback,
    user_data: *mut c_void,
) -> ForgeResult {
    if image_data.is_null() || prompt.is_null() {
        return ForgeResult::NullPointer;
    }
    if image_len == 0 || image_len > MAX_ENCODED_IMAGE_BYTES {
        return ForgeResult::InvalidParameter;
    }

    let mut engine = match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => e,
        None => return ForgeResult::NullPointer,
    };

    let image_bytes = unsafe { std::slice::from_raw_parts(image_data, image_len) };
    let prompt = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(_) => return ForgeResult::InvalidParameter,
    };

    let result = engine.generate_vision(image_bytes, prompt, |token| {
        if let Ok(c_token) = CString::new(token) {
            callback(c_token.as_ptr(), user_data);
        }
    });

    match result {
        Ok(_) => ForgeResult::Ok,
        Err(crate::ForgeError::InvalidParameter(_)) => ForgeResult::InvalidParameter,
        Err(crate::ForgeError::ImageProcessingFailed(_)) => ForgeResult::MultimodalLoadFailed,
        Err(crate::ForgeError::DecodeFailed(_)) => ForgeResult::DecodeFailed,
        Err(crate::ForgeError::Cancelled) => ForgeResult::Cancelled,
        Err(_) => ForgeResult::Unknown,
    }
}

/// Generate text from an image and prompt (vision models)
///
/// # Arguments
///
/// * `engine` - Engine handle (must have vision enabled)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `rgba_data` - Raw RGBA pixel data (4 bytes per pixel)
/// * `rgba_len` - Length of RGBA data (should be width * height * 4)
/// * `prompt` - Text prompt (null-terminated, should contain image marker)
/// * `callback` - Called for each generated token
/// * `user_data` - Passed to callback
#[no_mangle]
pub extern "C" fn forge_generate_vision_rgba(
    engine: ForgeHandle,
    width: u32,
    height: u32,
    rgba_data: *const u8,
    rgba_len: usize,
    prompt: *const c_char,
    callback: TokenCallback,
    user_data: *mut c_void,
) -> ForgeResult {
    if rgba_data.is_null() || prompt.is_null() {
        return ForgeResult::NullPointer;
    }

    let expected_len = match crate::encoders::mtmd_encoder::validated_rgba_len(width, height) {
        Ok(len) => len,
        Err(_) => return ForgeResult::InvalidParameter,
    };
    if rgba_len != expected_len {
        return ForgeResult::InvalidParameter;
    }

    let mut engine = match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => e,
        None => return ForgeResult::NullPointer,
    };
    let rgba = unsafe { std::slice::from_raw_parts(rgba_data, rgba_len) };
    let prompt = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(_) => return ForgeResult::InvalidParameter,
    };

    let result = engine.generate_vision_rgba(width, height, rgba, prompt, |token| {
        if let Ok(c_token) = CString::new(token) {
            callback(c_token.as_ptr(), user_data);
        }
    });

    match result {
        Ok(_) => ForgeResult::Ok,
        Err(crate::ForgeError::InvalidParameter(_)) => ForgeResult::InvalidParameter,
        Err(crate::ForgeError::ImageProcessingFailed(_)) => ForgeResult::MultimodalLoadFailed,
        Err(crate::ForgeError::DecodeFailed(_)) => ForgeResult::DecodeFailed,
        Err(crate::ForgeError::Cancelled) => ForgeResult::Cancelled,
        Err(_) => ForgeResult::Unknown,
    }
}

/// Reset the conversation context (clear KV cache)
#[no_mangle]
pub extern "C" fn forge_reset(engine: ForgeHandle) -> ForgeResult {
    match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(mut e) => {
            e.reset();
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}

/// Get current memory usage in MB
#[no_mangle]
pub extern "C" fn forge_memory_current_mb(engine: ForgeHandle) -> f64 {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => {
            let (current, _) = e.memory_stats();
            current
        }
        None => 0.0,
    }
}

/// Get peak memory usage in MB
#[no_mangle]
pub extern "C" fn forge_memory_peak_mb(engine: ForgeHandle) -> f64 {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => {
            let (_, peak) = e.memory_stats();
            peak
        }
        None => 0.0,
    }
}

// ============================================================
// AUDIO DECODER FUNCTIONS (Vocoder)
// ============================================================

use crate::decoders::audio::{AudioDecoder, DEFAULT_SAMPLE_RATE};
use crate::traits::Decoder;

/// Opaque type for the audio decoder (vocoder)
#[repr(C)]
pub struct ForgeAudioDecoderOpaque {
    _private: [u8; 0],
}

/// Opaque handle to the audio decoder
pub type ForgeAudioDecoderHandle = *mut ForgeAudioDecoderOpaque;

impl ForgeAudioDecoderOpaque {
    #[inline]
    unsafe fn as_decoder_mut<'a>(handle: ForgeAudioDecoderHandle) -> Option<&'a mut AudioDecoder> {
        if handle.is_null() {
            None
        } else {
            Some(&mut *(handle as *mut AudioDecoder))
        }
    }

    #[inline]
    fn from_decoder(decoder: AudioDecoder) -> ForgeAudioDecoderHandle {
        Box::into_raw(Box::new(decoder)) as ForgeAudioDecoderHandle
    }

    #[inline]
    unsafe fn destroy(handle: ForgeAudioDecoderHandle) {
        if !handle.is_null() {
            drop(Box::from_raw(handle as *mut AudioDecoder));
        }
    }
}

/// Audio output configuration
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ForgeAudioConfig {
    /// FFT size (default: 1280 for LFM2.5-Audio)
    pub n_fft: u32,
    /// Hop length in samples (default: 320)
    pub hop_length: u32,
    /// Sample rate in Hz (default: 24000)
    pub sample_rate: u32,
}

impl Default for ForgeAudioConfig {
    fn default() -> Self {
        Self {
            n_fft: 1280,
            hop_length: 320,
            sample_rate: 24000,
        }
    }
}

/// Get default audio configuration for LFM2.5-Audio
#[no_mangle]
pub extern "C" fn forge_audio_config_default() -> ForgeAudioConfig {
    ForgeAudioConfig::default()
}

/// Create a new audio decoder (vocoder) with default parameters
///
/// # Returns
///
/// Handle to the decoder, or NULL on failure
#[no_mangle]
pub extern "C" fn forge_audio_decoder_create() -> ForgeAudioDecoderHandle {
    match AudioDecoder::new() {
        Ok(decoder) => ForgeAudioDecoderOpaque::from_decoder(decoder),
        Err(e) => {
            log::error!("forge_audio_decoder_create failed: {}", e);
            ptr::null_mut()
        }
    }
}

/// Create a new audio decoder with custom parameters
///
/// # Arguments
///
/// * `config` - Audio configuration
///
/// # Returns
///
/// Handle to the decoder, or NULL on failure
#[no_mangle]
pub extern "C" fn forge_audio_decoder_create_with_config(
    config: *const ForgeAudioConfig,
) -> ForgeAudioDecoderHandle {
    if config.is_null() {
        return forge_audio_decoder_create();
    }

    let config = unsafe { *config };
    match AudioDecoder::with_params(
        config.n_fft as usize,
        config.hop_length as usize,
        config.sample_rate,
    ) {
        Ok(decoder) => ForgeAudioDecoderOpaque::from_decoder(decoder),
        Err(e) => {
            log::error!("forge_audio_decoder_create_with_config failed: {}", e);
            ptr::null_mut()
        }
    }
}

/// Destroy an audio decoder and free resources
///
/// # Safety
///
/// The handle must have been created by forge_audio_decoder_create.
/// After calling this, the handle is invalid.
#[no_mangle]
pub extern "C" fn forge_audio_decoder_destroy(decoder: ForgeAudioDecoderHandle) {
    unsafe {
        ForgeAudioDecoderOpaque::destroy(decoder);
    }
    log::debug!("Audio decoder destroyed");
}

/// Process embeddings and generate audio samples
///
/// This is the main vocoder function. It takes magnitude/phase embeddings
/// from the LLM and converts them to PCM audio samples.
///
/// # Arguments
///
/// * `decoder` - Audio decoder handle
/// * `embeddings` - Model output embeddings (magnitude + phase, interleaved)
/// * `embedding_len` - Number of embedding values (should be n_fft_bins * 2)
/// * `output` - Buffer to write PCM samples (f32, -1.0 to 1.0)
/// * `output_capacity` - Size of output buffer in samples
/// * `samples_written` - Output: number of samples actually written
///
/// # Returns
///
/// ForgeResult::Ok on success, error code on failure
#[no_mangle]
pub extern "C" fn forge_audio_decoder_process(
    decoder: ForgeAudioDecoderHandle,
    embeddings: *const f32,
    embedding_len: usize,
    output: *mut f32,
    output_capacity: usize,
    samples_written: *mut usize,
) -> ForgeResult {
    if embeddings.is_null() || output.is_null() || samples_written.is_null() {
        return ForgeResult::NullPointer;
    }
    if embedding_len == 0 || embedding_len > MAX_AUDIO_EMBEDDING_VALUES {
        return ForgeResult::InvalidParameter;
    }

    let decoder = match unsafe { ForgeAudioDecoderOpaque::as_decoder_mut(decoder) } {
        Some(d) => d,
        None => return ForgeResult::NullPointer,
    };

    let embeddings = unsafe { std::slice::from_raw_parts(embeddings, embedding_len) };

    match decoder.process_embeddings(embeddings) {
        Ok(chunk) => {
            let samples = &chunk.samples;
            let to_write = samples.len().min(output_capacity);

            unsafe {
                std::ptr::copy_nonoverlapping(samples.as_ptr(), output, to_write);
                *samples_written = to_write;
            }

            if samples.len() > output_capacity {
                log::warn!(
                    "Audio buffer too small: {} samples, capacity {}",
                    samples.len(),
                    output_capacity
                );
            }

            ForgeResult::Ok
        }
        Err(e) => {
            log::error!("forge_audio_decoder_process failed: {}", e);
            unsafe {
                *samples_written = 0;
            }
            ForgeResult::InvalidParameter
        }
    }
}

/// Flush remaining audio samples at end of stream
///
/// Call this after processing all frames to get the final samples
/// that are still in the overlap buffer.
///
/// # Arguments
///
/// * `decoder` - Audio decoder handle
/// * `output` - Buffer to write PCM samples
/// * `output_capacity` - Size of output buffer in samples
/// * `samples_written` - Output: number of samples actually written
///
/// # Returns
///
/// ForgeResult::Ok on success
#[no_mangle]
pub extern "C" fn forge_audio_decoder_flush(
    decoder: ForgeAudioDecoderHandle,
    output: *mut f32,
    output_capacity: usize,
    samples_written: *mut usize,
) -> ForgeResult {
    if output.is_null() || samples_written.is_null() {
        return ForgeResult::NullPointer;
    }

    let decoder = match unsafe { ForgeAudioDecoderOpaque::as_decoder_mut(decoder) } {
        Some(d) => d,
        None => return ForgeResult::NullPointer,
    };

    let chunk = decoder.flush_audio();
    let samples = &chunk.samples;
    let to_write = samples.len().min(output_capacity);

    unsafe {
        std::ptr::copy_nonoverlapping(samples.as_ptr(), output, to_write);
        *samples_written = to_write;
    }

    ForgeResult::Ok
}

/// Reset the audio decoder state
///
/// Call this when starting a new audio stream.
#[no_mangle]
pub extern "C" fn forge_audio_decoder_reset(decoder: ForgeAudioDecoderHandle) -> ForgeResult {
    match unsafe { ForgeAudioDecoderOpaque::as_decoder_mut(decoder) } {
        Some(d) => {
            d.reset();
            ForgeResult::Ok
        }
        None => ForgeResult::NullPointer,
    }
}

/// Get the expected embedding size per frame
///
/// This is the number of f32 values expected per call to process.
/// For LFM2.5-Audio with n_fft=1280, this is 1282 (641 bins * 2).
#[no_mangle]
pub extern "C" fn forge_audio_decoder_embedding_size(decoder: ForgeAudioDecoderHandle) -> usize {
    match unsafe { ForgeAudioDecoderOpaque::as_decoder_mut(decoder) } {
        Some(d) => d.embedding_size(),
        None => 0,
    }
}

/// Get the sample rate of the audio decoder
#[no_mangle]
pub extern "C" fn forge_audio_decoder_sample_rate(_decoder: ForgeAudioDecoderHandle) -> u32 {
    DEFAULT_SAMPLE_RATE
}

// ============================================================
// AUDIO INPUT FUNCTIONS (Encoder)
// ============================================================

/// Check if the engine supports audio input
///
/// Returns true if the engine was created with an audio-capable model.
#[no_mangle]
pub extern "C" fn forge_engine_supports_audio(engine: ForgeHandle) -> bool {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.supports_audio(),
        None => false,
    }
}

/// Get the expected audio sample rate for input
///
/// Returns the sample rate in Hz (e.g., 16000 for most speech models).
/// Returns 0 if audio is not supported.
#[no_mangle]
pub extern "C" fn forge_engine_audio_sample_rate(engine: ForgeHandle) -> u32 {
    match unsafe { ForgeEngineOpaque::as_engine(engine) } {
        Some(e) => e.audio_sample_rate().unwrap_or(0),
        None => 0,
    }
}

/// Generate a response from audio input
///
/// This processes raw PCM audio samples and generates a text or audio response.
///
/// # Arguments
///
/// * `engine` - Engine handle (must have audio enabled)
/// * `samples` - PCM audio samples (f32, normalized -1.0 to 1.0)
/// * `n_samples` - Number of audio samples
/// * `prompt` - Text prompt (should contain media marker, null-terminated)
/// * `callback` - Called for each generated token
/// * `user_data` - Passed to callback
///
/// # Returns
///
/// ForgeResult::Ok on success, error code on failure
#[no_mangle]
pub extern "C" fn forge_generate_audio(
    engine: ForgeHandle,
    samples: *const f32,
    n_samples: usize,
    prompt: *const c_char,
    callback: TokenCallback,
    user_data: *mut c_void,
) -> ForgeResult {
    if samples.is_null() || prompt.is_null() {
        return ForgeResult::NullPointer;
    }
    if n_samples == 0 || n_samples > MAX_AUDIO_SAMPLES {
        return ForgeResult::InvalidParameter;
    }

    let mut engine = match unsafe { ForgeEngineOpaque::as_engine_mut(engine) } {
        Some(e) => e,
        None => return ForgeResult::NullPointer,
    };

    let audio_samples = unsafe { std::slice::from_raw_parts(samples, n_samples) };
    let prompt = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(_) => return ForgeResult::InvalidParameter,
    };

    let result = engine.generate_audio(audio_samples, prompt, |token| {
        if let Ok(c_token) = CString::new(token) {
            callback(c_token.as_ptr(), user_data);
        }
    });

    match result {
        Ok(_) => ForgeResult::Ok,
        Err(crate::ForgeError::InvalidParameter(_)) => ForgeResult::InvalidParameter,
        Err(crate::ForgeError::AudioProcessingFailed(_)) => ForgeResult::MultimodalLoadFailed,
        Err(crate::ForgeError::DecodeFailed(_)) => ForgeResult::DecodeFailed,
        Err(crate::ForgeError::Cancelled) => ForgeResult::Cancelled,
        Err(_) => ForgeResult::Unknown,
    }
}
