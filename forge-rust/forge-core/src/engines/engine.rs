//! High-level inference engine

use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::context::{ContextParams, LlamaContext};
use crate::error::{ForgeError, Result};
use crate::memory::MemoryTracker;
use crate::model::{LlamaModel, ModelParams};
use crate::multimodal::{MultimodalContext, MultimodalParams};
use crate::sampler::{Sampler, SamplerParams};

/// Configuration for the inference engine
#[derive(Debug, Clone)]
pub struct ForgeConfig {
    /// Model loading parameters
    pub model: ModelParams,
    /// Context parameters
    pub context: ContextParams,
    /// Sampling parameters
    pub sampler: SamplerParams,
    /// Multimodal parameters (if using vision)
    pub multimodal: MultimodalParams,
    /// Maximum tokens to generate per turn
    pub max_tokens: u32,
    /// Memory budget in MB (0 = unlimited)
    pub memory_budget_mb: usize,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            model: ModelParams::default(),
            context: ContextParams::default(),
            sampler: SamplerParams::default(),
            multimodal: MultimodalParams::default(),
            max_tokens: 128,
            memory_budget_mb: 0,
        }
    }
}

impl ForgeConfig {
    /// Create a configuration optimized for mobile devices
    pub fn mobile() -> Self {
        Self {
            model: ModelParams {
                n_gpu_layers: -1,
                use_mmap: true,
                use_mlock: false,
                vocab_only: false,
            },
            context: ContextParams {
                n_ctx: 2048,
                n_batch: 128, // Lower for mobile
                n_ubatch: 128,
                n_threads: 2, // Conservative
                n_threads_batch: 2,
                flash_attention: true,
            },
            sampler: SamplerParams::default(),
            multimodal: MultimodalParams::default(),
            max_tokens: 128,
            memory_budget_mb: 800, // ~800 MB budget for mobile
        }
    }

    /// Set context size
    pub fn with_context_size(mut self, n_ctx: u32) -> Self {
        self.context.n_ctx = n_ctx;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, n_batch: u32) -> Self {
        self.context.n_batch = n_batch;
        self
    }

    /// Set maximum tokens to generate
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set memory budget
    pub fn with_memory_budget(mut self, budget_mb: usize) -> Self {
        self.memory_budget_mb = budget_mb;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.sampler.temperature = temp;
        self
    }
}

/// The main inference engine
///
/// This is the high-level API for running inference. It manages
/// all resources with RAII and provides a clean interface.
///
/// # Multi-turn Conversation Support
///
/// The engine tracks `n_past` to maintain context across turns.
/// For multi-turn conversations:
/// - First turn: Use `generate()` or `generate_turn()` with `is_first_turn: true`
/// - Subsequent turns: Use `generate_turn()` with `is_first_turn: false`
///
/// # BOS Token Handling
///
/// CRITICAL: Only add BOS token on the first turn (`n_past == 0`).
/// The `generate_turn()` method handles this automatically.
/// If using `generate()` directly, the caller must include BOS in the prompt only for first turn.
pub struct ForgeEngine {
    /// Model weights (mmap'd)
    model: LlamaModel,
    /// Inference context (KV cache, scratch buffers)
    context: LlamaContext,
    /// Sampler state
    sampler: Sampler,
    /// Vision context (optional - only for vision models)
    multimodal: Option<MultimodalContext>,
    config: ForgeConfig,
    memory: MemoryTracker,
    /// Current position in the context (token count processed so far)
    n_past: i32,
    /// Last EOG token generated (for feeding back to model)
    last_eog_token: Option<i32>,
    /// Number of tokens to keep when shifting context (system prompt size)
    n_keep: i32,
    /// Lock-free cancellation signal shared with platform wrappers.
    cancellation: Arc<AtomicBool>,
}

#[derive(Default)]
struct Utf8StreamDecoder {
    pending: Vec<u8>,
}

impl Utf8StreamDecoder {
    fn push<F: FnMut(&str)>(&mut self, bytes: &[u8], output: &mut String, callback: &mut F) {
        self.pending.extend_from_slice(bytes);
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(text) => {
                    if !text.is_empty() {
                        output.push_str(text);
                        callback(text);
                    }
                    self.pending.clear();
                    break;
                }
                Err(error) if error.valid_up_to() > 0 => {
                    let valid_len = error.valid_up_to();
                    let valid = String::from_utf8(self.pending.drain(..valid_len).collect())
                        .expect("validated UTF-8 prefix");
                    output.push_str(&valid);
                    callback(&valid);
                }
                Err(error) if error.error_len().is_some() => {
                    let invalid_len = error.error_len().unwrap_or(1);
                    self.pending.drain(..invalid_len);
                    output.push('\u{fffd}');
                    callback("\u{fffd}");
                }
                Err(_) => break,
            }
        }
    }

    fn finish<F: FnMut(&str)>(&mut self, output: &mut String, callback: &mut F) {
        if !self.pending.is_empty() {
            let text = String::from_utf8_lossy(&self.pending).into_owned();
            output.push_str(&text);
            callback(&text);
            self.pending.clear();
        }
    }
}

impl ForgeEngine {
    fn begin_generation(&self) {
        self.cancellation.store(false, Ordering::Release);
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.cancellation.load(Ordering::Acquire) {
            Err(ForgeError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn make_context_room(&mut self, required: i32) -> Result<()> {
        let n_ctx = self.context.n_ctx() as i32;
        if required >= n_ctx {
            return Err(ForgeError::InvalidParameter(format!(
                "input batch of {required} tokens does not fit context size {n_ctx}"
            )));
        }
        while self.n_past + required >= n_ctx {
            if !self
                .context
                .shift_context_if_needed(&mut self.n_past, self.n_keep)
            {
                return Err(ForgeError::InvalidParameter(format!(
                    "context is full at {} tokens and cannot be shifted",
                    self.n_past
                )));
            }
        }
        Ok(())
    }

    /// Return the lock-free cancellation signal used by FFI wrappers.
    pub fn cancellation_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancellation)
    }

    /// Request cancellation of the active generation.
    pub fn cancel(&self) {
        self.cancellation.store(true, Ordering::Release);
    }

    /// Create a new engine with a text-only model
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the GGUF model file
    /// * `config` - Engine configuration
    pub fn new<P: AsRef<Path>>(model_path: P, config: ForgeConfig) -> Result<Self> {
        let memory = MemoryTracker::new(if config.memory_budget_mb > 0 {
            Some(config.memory_budget_mb)
        } else {
            None
        });

        memory.log_status("init");

        let model_path_buf = model_path.as_ref().to_path_buf();

        // Load model
        let model = LlamaModel::load(&model_path_buf, config.model.clone())?;
        let model_bytes = model.tensor_bytes();
        if !memory.alloc(model_bytes) {
            return Err(ForgeError::MemoryBudgetExceeded {
                requested_mb: model_bytes.div_ceil(1024 * 1024),
                available_mb: config.memory_budget_mb,
            });
        }

        // Create context
        let context = LlamaContext::new(&model, config.context.clone())?;

        // Create sampler with model reference (enables DRY sampler)
        let sampler = Sampler::new(config.sampler.clone(), Some(model.as_ptr()));

        memory.log_status("after_load");

        Ok(Self {
            model,
            context,
            sampler,
            multimodal: None,
            config,
            memory,
            n_past: 0,
            last_eog_token: None,
            n_keep: 0,
            cancellation: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Create a new engine with vision capabilities
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to the GGUF model file
    /// * `clip_path` - Path to the CLIP model file (mmproj-*.gguf)
    /// * `config` - Engine configuration
    pub fn with_vision<P: AsRef<Path>>(
        model_path: P,
        clip_path: P,
        config: ForgeConfig,
    ) -> Result<Self> {
        let clip_path = clip_path.as_ref().to_path_buf();
        let mut engine = Self::new(model_path, config)?;

        // mmproj tensors are close to their GGUF file size; include that known
        // lower-bound in budget reporting before loading the projector.
        let mmproj_bytes = std::fs::metadata(&clip_path)?.len() as usize;
        if !engine.memory.alloc(mmproj_bytes) {
            return Err(ForgeError::MemoryBudgetExceeded {
                requested_mb: mmproj_bytes.div_ceil(1024 * 1024),
                available_mb: engine.config.memory_budget_mb,
            });
        }

        // Load multimodal context
        let mm =
            MultimodalContext::new(&clip_path, &engine.model, engine.config.multimodal.clone())?;
        engine.multimodal = Some(mm);

        engine.memory.log_status("after_clip");

        Ok(engine)
    }

    /// Generate text from a prompt
    ///
    /// # Arguments
    ///
    /// * `prompt` - The input prompt
    /// * `callback` - Called for each generated token
    ///
    /// # Returns
    ///
    /// The complete generated text
    pub fn generate<F>(&mut self, prompt: &str, mut callback: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.begin_generation();
        self.memory.log_status("generate_start");

        // Tokenize prompt
        let tokens = self.context.tokenize(prompt, true)?;
        log::debug!(
            "Tokenized {} chars into {} tokens",
            prompt.len(),
            tokens.len()
        );

        // Clear context for new generation
        self.context.clear_kv_cache();
        self.n_past = 0;

        // Reset sampler state
        self.sampler.reset();

        // Process prompt in batches
        let batch_size = self.config.context.n_batch as i32;
        for chunk in tokens.chunks(batch_size as usize) {
            self.check_cancelled()?;
            self.make_context_room(chunk.len() as i32)?;
            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_tokens(
                chunk, npast, 0, // seq_id
            );

            self.context.decode()?;

            // Accept tokens for repeat penalty tracking
            for &token in chunk {
                self.sampler.accept(token);
            }

            self.n_past += chunk.len() as i32;
        }

        self.memory.log_status("after_prompt");

        // Generate tokens
        let mut output = String::new();
        let mut utf8 = Utf8StreamDecoder::default();
        let max_tokens = self.config.max_tokens as i32;

        for _ in 0..max_tokens {
            self.check_cancelled()?;
            self.make_context_room(1)?;
            // Sample next token
            let token = self.sampler.sample(self.context.as_ptr(), -1);

            // Check for end of generation
            if self.context.is_eog(token) {
                self.last_eog_token = Some(token);
                self.sampler.accept(token);
                let npast = self.n_past;
                self.context.batch_mut().reset();
                self.context.batch_mut().add_token(token, npast, 0, false);
                let _ = self.context.decode();
                self.n_past += 1;
                break;
            }

            // Convert to text
            utf8.push(
                &self.context.token_to_piece_bytes(token),
                &mut output,
                &mut callback,
            );

            // Accept the token for repeat penalty tracking
            self.sampler.accept(token);

            // Prepare next decode
            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_token(token, npast, 0, true);
            self.context.decode()?;
            self.n_past += 1;
        }

        utf8.finish(&mut output, &mut callback);
        self.memory.log_status("after_generate");

        Ok(output)
    }

    /// Generate a response for a conversation turn (multi-turn aware)
    ///
    /// This method properly handles:
    /// - BOS token: Only added on first turn (`n_past == 0`)
    /// - EOG feedback: Previous EOG token is fed back to model
    /// - Context accumulation: Maintains conversation history
    ///
    /// # Arguments
    ///
    /// * `prompt` - The formatted prompt for this turn (WITHOUT BOS - it's added automatically)
    /// * `add_bos` - Whether to add BOS token (should be true only for first turn)
    /// * `callback` - Called for each generated token
    ///
    /// # Multi-turn Example
    ///
    /// ```ignore
    /// // First turn
    /// engine.generate_turn("<|im_start|>user\nHello\n<|im_end|><|im_start|>assistant\n", true, |t| print!("{}", t))?;
    ///
    /// // Second turn (no BOS, context preserved)
    /// engine.generate_turn("<|im_start|>user\nHow are you?\n<|im_end|><|im_start|>assistant\n", false, |t| print!("{}", t))?;
    /// ```
    pub fn generate_turn<F>(
        &mut self,
        prompt: &str,
        add_bos: bool,
        mut callback: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.begin_generation();
        self.memory.log_status("turn_start");

        if add_bos && self.n_past != 0 {
            return Err(ForgeError::InvalidParameter(
                "add_bos can only be true for the first conversation turn; call reset() first"
                    .to_string(),
            ));
        }

        // Tokenize prompt
        let tokens = self.context.tokenize(prompt, add_bos)?;
        log::debug!(
            "Tokenized turn: {} tokens, n_past={}",
            tokens.len(),
            self.n_past
        );

        // Process prompt tokens
        let batch_size = self.config.context.n_batch as i32;
        for chunk in tokens.chunks(batch_size as usize) {
            self.check_cancelled()?;
            self.make_context_room(chunk.len() as i32)?;
            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_tokens(chunk, npast, 0);
            self.context.decode()?;

            for &token in chunk {
                self.sampler.accept(token);
            }

            self.n_past += chunk.len() as i32;
        }

        // Generate response
        let mut output = String::new();
        let mut utf8 = Utf8StreamDecoder::default();
        let max_tokens = self.config.max_tokens as i32;

        for _ in 0..max_tokens {
            self.check_cancelled()?;
            self.make_context_room(1)?;
            let token = self.sampler.sample(self.context.as_ptr(), -1);

            if self.context.is_eog(token) {
                self.last_eog_token = Some(token);
                self.sampler.accept(token);
                let npast = self.n_past;
                self.context.batch_mut().reset();
                self.context.batch_mut().add_token(token, npast, 0, false);
                self.context.decode()?;
                self.n_past += 1;
                break;
            }

            utf8.push(
                &self.context.token_to_piece_bytes(token),
                &mut output,
                &mut callback,
            );

            self.sampler.accept(token);

            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_token(token, npast, 0, true);
            self.context.decode()?;
            self.n_past += 1;
        }

        utf8.finish(&mut output, &mut callback);
        self.memory.log_status("after_turn");

        Ok(output)
    }

    /// Check if this is the first turn (n_past == 0)
    ///
    /// Use this to determine whether to add BOS token in chat template
    pub fn is_first_turn(&self) -> bool {
        self.n_past == 0
    }

    /// Get the current context position (tokens processed so far)
    pub fn n_past(&self) -> i32 {
        self.n_past
    }

    /// Set the number of tokens to preserve when context sliding occurs
    ///
    /// When the context fills up, the engine will automatically shift the context
    /// to make room for new tokens. This setting controls how many tokens at the
    /// beginning of the context are preserved (typically the system prompt).
    ///
    /// # Arguments
    ///
    /// * `n_keep` - Number of tokens to keep (0 = don't preserve any)
    pub fn set_n_keep(&mut self, n_keep: i32) {
        self.n_keep = n_keep;
        log::debug!("Set n_keep = {}", n_keep);
    }

    /// Get the current n_keep setting
    pub fn n_keep(&self) -> i32 {
        self.n_keep
    }

    /// Check if context sliding is supported
    pub fn can_shift_context(&self) -> bool {
        self.context.can_shift()
    }

    /// Generate text from an image (JPEG/PNG bytes) and prompt
    ///
    /// This method decodes the image bytes inside the engine, avoiding
    /// the need to pass large RGBA buffers over the FFI boundary.
    pub fn generate_vision<F>(
        &mut self,
        image_data: &[u8],
        prompt: &str,
        callback: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        use crate::encoders::mtmd_encoder::{MAX_IMAGE_DIMENSION, MAX_IMAGE_PIXELS};
        use image::{io::Limits, io::Reader as ImageReader, GenericImageView};
        use std::io::Cursor;

        const MAX_ENCODED_IMAGE_BYTES: usize = 64 * 1024 * 1024;
        const MAX_DECODER_ALLOCATION_BYTES: u64 = 256 * 1024 * 1024;

        if image_data.is_empty() || image_data.len() > MAX_ENCODED_IMAGE_BYTES {
            return Err(ForgeError::ImageProcessingFailed(format!(
                "Encoded image size must be between 1 byte and {} MiB",
                MAX_ENCODED_IMAGE_BYTES / (1024 * 1024)
            )));
        }

        let mut reader = ImageReader::new(Cursor::new(image_data))
            .with_guessed_format()
            .map_err(|e| {
                ForgeError::ImageProcessingFailed(format!("Failed to inspect image: {e}"))
            })?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_DIMENSION);
        limits.max_image_height = Some(MAX_IMAGE_DIMENSION);
        limits.max_alloc = Some(MAX_DECODER_ALLOCATION_BYTES);
        reader.limits(limits);

        let img = reader.decode().map_err(|e| {
            ForgeError::ImageProcessingFailed(format!("Failed to decode image: {}", e))
        })?;

        let (width, height) = img.dimensions();
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .ok_or_else(|| ForgeError::ImageProcessingFailed("Image size overflow".to_string()))?;
        if pixels > MAX_IMAGE_PIXELS {
            return Err(ForgeError::ImageProcessingFailed(format!(
                "Image contains {pixels} pixels; the limit is {MAX_IMAGE_PIXELS}"
            )));
        }
        let rgba_data = img.to_rgba8().into_raw();

        self.generate_vision_rgba(width, height, &rgba_data, prompt, callback)
    }

    /// Generate text from an image and prompt (vision models only)
    ///
    /// Generate text from an image
    ///
    /// # Arguments
    ///
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `rgba_data` - Raw RGBA pixel data (4 bytes per pixel)
    /// * `prompt` - The text prompt (should contain the image marker)
    /// * `callback` - Called for each generated token
    ///
    /// # Note
    ///
    /// The new mtmd API requires raw RGBA pixel data. To process JPEG/PNG,
    /// decode them first using an image library before calling this function.
    pub fn generate_vision_rgba<F>(
        &mut self,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        prompt: &str,
        mut callback: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.begin_generation();
        self.memory.log_status("vision_start");
        self.check_cancelled()?;

        let mm = self.multimodal.as_mut().ok_or_else(|| {
            ForgeError::InvalidParameter(
                "Vision not enabled. Use with_vision() to create engine.".to_string(),
            )
        })?;

        // Clear context and sampler for fresh vision inference
        self.context.clear_kv_cache();
        self.n_past = 0;
        log::debug!("Resetting sampler for fresh vision inference");
        self.sampler.reset();
        self.context.batch_mut().reset();

        // Capture context pointer for multimodal eval
        let ctx_ptr = self.context.as_ptr();

        // Process image from raw RGBA pixels
        let mut chunks = mm.process_image_rgba(width, height, rgba_data, prompt)?;
        self.memory.log_status("after_image_process");

        let n_batch = self.config.context.n_batch as i32;
        self.n_past = mm.eval_chunks(
            ctx_ptr,
            &mut chunks,
            0, // n_past = 0 (fresh context)
            0, // seq_id = 0
            n_batch,
        )?;

        log::info!("Vision chunks evaluated, n_past = {}", self.n_past);
        self.memory.log_status("after_chunk_eval");

        // Generate text
        let mut output = String::new();
        let mut utf8 = Utf8StreamDecoder::default();
        let max_tokens = self.config.max_tokens as i32;

        for i in 0..max_tokens {
            self.check_cancelled()?;
            self.make_context_room(1)?;
            let token = self.sampler.sample(self.context.as_ptr(), -1);

            if self.context.is_eog(token) {
                if i == 0 {
                    log::warn!("EOS on first token after vision eval - possible prompt issue");
                }
                self.last_eog_token = Some(token);
                self.sampler.accept(token);
                break;
            }

            utf8.push(
                &self.context.token_to_piece_bytes(token),
                &mut output,
                &mut callback,
            );

            self.sampler.accept(token);

            // Prepare next decode
            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_token(token, npast, 0, true);
            self.context.decode()?;
            self.n_past += 1;
        }

        utf8.finish(&mut output, &mut callback);
        self.memory.log_status("after_vision_generate");

        Ok(output)
    }

    /// Check if the engine supports audio input
    pub fn supports_audio(&self) -> bool {
        self.multimodal
            .as_ref()
            .map(|m| m.supports_audio())
            .unwrap_or(false)
    }

    /// Get the expected audio sample rate in Hz
    pub fn audio_sample_rate(&self) -> Option<u32> {
        self.multimodal.as_ref().and_then(|m| m.audio_sample_rate())
    }

    /// Generate text from audio input
    ///
    /// # Arguments
    ///
    /// * `samples` - PCM audio samples (f32, normalized -1.0 to 1.0)
    /// * `prompt` - The text prompt (should contain the media marker)
    /// * `callback` - Called for each generated token
    pub fn generate_audio<F>(
        &mut self,
        samples: &[f32],
        prompt: &str,
        mut callback: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.begin_generation();
        self.memory.log_status("audio_start");
        self.check_cancelled()?;

        let mm = self.multimodal.as_mut().ok_or_else(|| {
            ForgeError::InvalidParameter(
                "Audio not enabled. Use with_vision() to create engine with multimodal support."
                    .to_string(),
            )
        })?;

        if !mm.supports_audio() {
            return Err(ForgeError::AudioProcessingFailed(
                "This model does not support audio input".to_string(),
            ));
        }

        // Clear context and sampler for fresh audio inference
        self.context.clear_kv_cache();
        self.n_past = 0;
        log::debug!("Resetting sampler for fresh audio inference");
        self.sampler.reset();
        self.context.batch_mut().reset();

        // Capture context pointer for multimodal eval
        let ctx_ptr = self.context.as_ptr();

        // Process audio samples
        let mut chunks = mm.process_audio(samples, prompt)?;
        self.memory.log_status("after_audio_process");

        let n_batch = self.config.context.n_batch as i32;
        self.n_past = mm.eval_chunks(
            ctx_ptr,
            &mut chunks,
            0, // n_past = 0 (fresh context)
            0, // seq_id = 0
            n_batch,
        )?;

        log::info!("Audio chunks evaluated, n_past = {}", self.n_past);
        self.memory.log_status("after_audio_chunk_eval");

        // Generate text response
        let mut output = String::new();
        let mut utf8 = Utf8StreamDecoder::default();
        let max_tokens = self.config.max_tokens as i32;

        for i in 0..max_tokens {
            self.check_cancelled()?;
            self.make_context_room(1)?;
            let token = self.sampler.sample(self.context.as_ptr(), -1);

            if self.context.is_eog(token) {
                if i == 0 {
                    log::warn!("EOS on first token after audio eval - possible prompt issue");
                }
                self.last_eog_token = Some(token);
                self.sampler.accept(token);
                break;
            }

            utf8.push(
                &self.context.token_to_piece_bytes(token),
                &mut output,
                &mut callback,
            );

            self.sampler.accept(token);

            // Prepare next decode
            let npast = self.n_past;
            self.context.batch_mut().reset();
            self.context.batch_mut().add_token(token, npast, 0, true);
            self.context.decode()?;
            self.n_past += 1;
        }

        utf8.finish(&mut output, &mut callback);
        self.memory.log_status("after_audio_generate");

        Ok(output)
    }

    /// Get memory statistics
    pub fn memory_stats(&self) -> (f64, f64) {
        (self.memory.current_mb(), self.memory.peak_mb())
    }

    /// Get the context size (n_ctx)
    pub fn context_size(&self) -> u32 {
        self.context.n_ctx()
    }

    /// Tokenize text (for inspection/testing)
    pub fn tokenize(&self, text: &str) -> Result<Vec<i32>> {
        self.context.tokenize(text, false)
    }

    /// Reset the conversation context
    pub fn reset(&mut self) {
        self.cancel();
        self.context.clear_kv_cache();
        self.n_past = 0;
        self.last_eog_token = None;
        self.sampler.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::Utf8StreamDecoder;

    #[test]
    fn utf8_stream_decoder_preserves_split_codepoints() {
        let mut decoder = Utf8StreamDecoder::default();
        let mut output = String::new();
        let mut chunks = Vec::new();
        decoder.push(&[0xf0, 0x9f], &mut output, &mut |s| {
            chunks.push(s.to_owned())
        });
        decoder.push(&[0x98, 0x80], &mut output, &mut |s| {
            chunks.push(s.to_owned())
        });
        decoder.finish(&mut output, &mut |s| chunks.push(s.to_owned()));
        assert_eq!(output, "😀");
        assert_eq!(chunks, vec!["😀"]);
    }
}

// ForgeEngine is automatically cleaned up when dropped
// - LlamaModel::drop() frees the model
// - LlamaContext::drop() frees the context and batch
// - Sampler::drop() frees the sampler
// - MultimodalContext::drop() frees CLIP
