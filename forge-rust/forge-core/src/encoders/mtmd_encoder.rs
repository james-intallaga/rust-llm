//! Multimodal (vision) support with RAII
//!
//! This module provides safe wrappers for mtmd (multimodal) functionality.
//! Note: The mtmd API has changed significantly - image loading now requires
//! raw RGBA pixel data rather than file paths or encoded buffers.

use std::ffi::CString;
use std::path::Path;
use std::ptr::NonNull;

use llama_cpp_sys::{
    llama_batch_free, llama_batch_init, llama_context, llama_decode, llama_get_model, llama_pos,
    llama_seq_id, mtmd_bitmap_free, mtmd_bitmap_get_nx, mtmd_bitmap_get_ny, mtmd_bitmap_init,
    mtmd_bitmap_init_from_audio, mtmd_context, mtmd_context_params, mtmd_context_params_default,
    mtmd_default_marker, mtmd_encode_chunk, mtmd_free, mtmd_get_audio_sample_rate,
    mtmd_get_output_embd, mtmd_init_from_file, mtmd_input_chunk, mtmd_input_chunk_get_n_tokens,
    mtmd_input_chunk_get_tokens_text, mtmd_input_chunk_get_type,
    mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_AUDIO,
    mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_IMAGE,
    mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_TEXT, mtmd_input_chunks, mtmd_input_chunks_free,
    mtmd_input_chunks_get, mtmd_input_chunks_init, mtmd_input_chunks_size, mtmd_input_text,
    mtmd_support_audio, mtmd_support_vision, mtmd_tokenize,
};

use crate::error::{ForgeError, Result};

/// Hard safety limits for images accepted by the public SDK. These limits are
/// deliberately generous enough for high-resolution photography while keeping
/// allocations bounded on mobile devices.
pub(crate) const MAX_IMAGE_DIMENSION: u32 = 16_384;
pub(crate) const MAX_IMAGE_PIXELS: u64 = 32_000_000;

pub(crate) fn validated_pixel_count(width: u32, height: u32) -> Result<usize> {
    if width == 0 || height == 0 {
        return Err(ForgeError::ImageProcessingFailed(
            "Image dimensions must be non-zero".to_string(),
        ));
    }
    if width > MAX_IMAGE_DIMENSION || height > MAX_IMAGE_DIMENSION {
        return Err(ForgeError::ImageProcessingFailed(format!(
            "Image dimensions exceed the {} pixel limit",
            MAX_IMAGE_DIMENSION
        )));
    }

    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| ForgeError::ImageProcessingFailed("Image size overflow".to_string()))?;
    if pixels > MAX_IMAGE_PIXELS {
        return Err(ForgeError::ImageProcessingFailed(format!(
            "Image contains {pixels} pixels; the limit is {MAX_IMAGE_PIXELS}"
        )));
    }

    usize::try_from(pixels)
        .map_err(|_| ForgeError::ImageProcessingFailed("Image size is unsupported".to_string()))
}

pub(crate) fn validated_rgba_len(width: u32, height: u32) -> Result<usize> {
    validated_pixel_count(width, height)?
        .checked_mul(4)
        .ok_or_else(|| ForgeError::ImageProcessingFailed("RGBA buffer size overflow".to_string()))
}

#[cfg(test)]
mod image_limit_tests {
    use super::*;

    #[test]
    fn validates_normal_rgba_size() {
        assert_eq!(validated_rgba_len(1920, 1080).unwrap(), 1920 * 1080 * 4);
    }

    #[test]
    fn rejects_zero_and_excessive_dimensions() {
        assert!(validated_rgba_len(0, 1080).is_err());
        assert!(validated_rgba_len(MAX_IMAGE_DIMENSION + 1, 1).is_err());
    }

    #[test]
    fn rejects_pixel_counts_that_used_to_wrap_u32() {
        assert!(validated_rgba_len(65_536, 65_536).is_err());
    }
}
use crate::model::LlamaModel;

/// Parameters for multimodal context
#[derive(Debug, Clone)]
pub struct MultimodalParams {
    /// Use GPU for image processing
    pub use_gpu: bool,
    /// Number of threads for CPU processing
    pub n_threads: i32,
    /// Print timing information
    pub print_timings: bool,
    /// Warmup (compile Metal pipelines on init)
    pub warmup: bool,
}

impl Default for MultimodalParams {
    fn default() -> Self {
        Self {
            use_gpu: true,
            n_threads: 2,
            print_timings: false,
            warmup: true, // Warmup by default for better first-inference latency
        }
    }
}

impl From<MultimodalParams> for mtmd_context_params {
    fn from(params: MultimodalParams) -> Self {
        let mut c_params = unsafe { mtmd_context_params_default() };
        c_params.use_gpu = params.use_gpu;
        c_params.n_threads = params.n_threads;
        c_params.print_timings = params.print_timings;
        c_params.warmup = params.warmup;
        c_params
    }
}

/// Get the default media marker string (e.g., "<image>" or "<__media__>")
///
/// This marker should be included in the prompt where the image should be inserted.
pub fn get_default_media_marker() -> String {
    unsafe {
        let marker = mtmd_default_marker();
        if marker.is_null() {
            "<__media__>".to_string()
        } else {
            std::ffi::CStr::from_ptr(marker)
                .to_string_lossy()
                .to_string()
        }
    }
}

/// Multimodal (CLIP) context
///
/// Handles image encoding for vision models like LFM2-VL.
/// Automatically frees resources when dropped.
pub struct MultimodalContext {
    ptr: NonNull<mtmd_context>,
}

// Safety: MultimodalContext owns the pointer
unsafe impl Send for MultimodalContext {}

impl MultimodalContext {
    /// Load a multimodal model (CLIP projector)
    ///
    /// # Arguments
    ///
    /// * `clip_path` - Path to the CLIP model file (mmproj-*.gguf)
    /// * `model` - The main LLM model
    /// * `params` - Multimodal parameters
    pub fn new<P: AsRef<Path>>(
        clip_path: P,
        model: &LlamaModel,
        params: MultimodalParams,
    ) -> Result<Self> {
        let path_str = clip_path.as_ref().to_string_lossy().to_string();
        let c_path = CString::new(path_str.clone())?;

        log::info!("Loading multimodal model from: {}", path_str);

        let ptr = unsafe { mtmd_init_from_file(c_path.as_ptr(), model.as_ptr(), params.into()) };

        let ptr = NonNull::new(ptr).ok_or_else(|| ForgeError::MultimodalLoadFailed {
            path: path_str.clone(),
            reason: "mtmd_init_from_file returned null".to_string(),
        })?;

        log::info!("Multimodal model loaded successfully");

        Ok(Self { ptr })
    }

    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut mtmd_context {
        self.ptr.as_ptr()
    }

    /// Check if this multimodal context supports vision
    pub fn supports_vision(&self) -> bool {
        unsafe { mtmd_support_vision(self.ptr.as_ptr()) }
    }

    /// Check if this multimodal context supports audio input
    pub fn supports_audio(&self) -> bool {
        unsafe { mtmd_support_audio(self.ptr.as_ptr()) }
    }

    /// Get the expected audio sample rate in Hz (e.g., 16000 for Whisper)
    /// Returns None if audio is not supported
    pub fn audio_sample_rate(&self) -> Option<u32> {
        let rate = unsafe { mtmd_get_audio_sample_rate(self.ptr.as_ptr()) };
        if rate < 0 {
            None
        } else {
            Some(rate as u32)
        }
    }

    /// Evaluate input chunks (image + text) into the llama context
    ///
    /// This is the critical function that actually pushes the image embeddings
    /// and text tokens into the context so they can be sampled from.
    ///
    /// # Arguments
    ///
    /// * `lctx` - The llama context
    /// * `chunks` - The tokenized input chunks from process_image_rgba
    /// * `n_past` - Current position in context
    /// * `seq_id` - Sequence ID (usually 0)
    /// * `n_batch` - Batch size for processing
    ///
    /// # Returns
    ///
    /// New n_past value after evaluation
    pub fn eval_chunks(
        &mut self,
        lctx: *mut llama_context,
        chunks: &mut InputChunks,
        mut n_past: llama_pos,
        seq_id: llama_seq_id,
        n_batch: i32,
    ) -> Result<i32> {
        let n_chunks = chunks.n_chunks();
        log::info!(
            "Evaluating {} multimodal chunks at n_past={}",
            n_chunks,
            n_past
        );

        if n_chunks == 0 {
            log::warn!("No chunks to evaluate");
            return Ok(n_past);
        }

        for i in 0..n_chunks {
            let is_last = i == n_chunks - 1;

            let chunk = unsafe { mtmd_input_chunks_get(chunks.as_ptr(), i) };
            if chunk.is_null() {
                return Err(ForgeError::ImageProcessingFailed(format!(
                    "Chunk {} is null",
                    i
                )));
            }

            n_past = self.eval_chunk_single(lctx, chunk, n_past, seq_id, n_batch, is_last)?;
        }

        log::info!("All chunks evaluated, final n_past = {}", n_past);

        Ok(n_past)
    }

    /// Evaluate a single chunk (text or image)
    fn eval_chunk_single(
        &mut self,
        lctx: *mut llama_context,
        chunk: *const mtmd_input_chunk,
        mut n_past: llama_pos,
        seq_id: llama_seq_id,
        n_batch: i32,
        logits_last: bool,
    ) -> Result<llama_pos> {
        let chunk_type = unsafe { mtmd_input_chunk_get_type(chunk) };

        // MTMD_INPUT_CHUNK_TYPE_TEXT = 0, IMAGE = 1, AUDIO = 2
        if chunk_type == mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_TEXT {
            n_past = self.eval_text_chunk(lctx, chunk, n_past, seq_id, n_batch, logits_last)?;
        } else if chunk_type == mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_IMAGE {
            n_past = self.eval_image_chunk(lctx, chunk, n_past, seq_id, n_batch, logits_last)?;
        } else if chunk_type == mtmd_input_chunk_type_MTMD_INPUT_CHUNK_TYPE_AUDIO {
            // Audio chunks are processed the same way as image chunks (encode + decode embeddings)
            n_past = self.eval_audio_chunk(lctx, chunk, n_past, seq_id, n_batch, logits_last)?;
        } else {
            return Err(ForgeError::ImageProcessingFailed(format!(
                "Unknown chunk type: {}",
                chunk_type
            )));
        }

        Ok(n_past)
    }

    /// Evaluate a text chunk
    fn eval_text_chunk(
        &self,
        lctx: *mut llama_context,
        chunk: *const mtmd_input_chunk,
        mut n_past: llama_pos,
        seq_id: llama_seq_id,
        n_batch: i32,
        logits_last: bool,
    ) -> Result<llama_pos> {
        let mut n_tokens: usize = 0;
        let tokens = unsafe { mtmd_input_chunk_get_tokens_text(chunk, &mut n_tokens) };

        if tokens.is_null() || n_tokens == 0 {
            log::debug!("Empty text chunk, skipping");
            return Ok(n_past);
        }

        log::debug!("Evaluating text chunk with {} tokens", n_tokens);

        // Create batch
        let mut batch = unsafe { llama_batch_init(n_batch, 0, 1) };

        let mut i = 0usize;
        while i < n_tokens {
            // Clear batch
            batch.n_tokens = 0;

            // Fill batch
            while i < n_tokens && batch.n_tokens < n_batch {
                let j = batch.n_tokens as isize;
                unsafe {
                    *batch.token.offset(j) = *tokens.add(i);
                    *batch.pos.offset(j) = n_past;
                    *batch.n_seq_id.offset(j) = 1;
                    *(*batch.seq_id.offset(j)).offset(0) = seq_id;
                    *batch.logits.offset(j) = 0; // false
                }
                n_past += 1;
                batch.n_tokens += 1;
                i += 1;
            }

            // Set logits for last token if needed
            let is_last_batch = i == n_tokens;
            if logits_last && is_last_batch && batch.n_tokens > 0 {
                unsafe {
                    *batch.logits.offset((batch.n_tokens - 1) as isize) = 1; // true
                }
            }

            // Decode
            let ret = unsafe { llama_decode(lctx, batch) };
            if ret != 0 {
                unsafe {
                    llama_batch_free(batch);
                }
                return Err(ForgeError::ImageProcessingFailed(format!(
                    "llama_decode failed for text chunk with code {}",
                    ret
                )));
            }
        }

        unsafe {
            llama_batch_free(batch);
        }

        Ok(n_past)
    }

    /// Evaluate an image chunk (encode + decode embeddings)
    fn eval_image_chunk(
        &mut self,
        lctx: *mut llama_context,
        chunk: *const mtmd_input_chunk,
        mut n_past: llama_pos,
        seq_id: llama_seq_id,
        n_batch: i32,
        logits_last: bool,
    ) -> Result<llama_pos> {
        log::info!("Encoding image chunk...");

        // Encode the image chunk
        let ret = unsafe { mtmd_encode_chunk(self.ptr.as_ptr(), chunk) };
        if ret != 0 {
            return Err(ForgeError::ImageProcessingFailed(format!(
                "mtmd_encode_chunk failed with code {}",
                ret
            )));
        }

        log::info!("Image encoded, getting embeddings...");

        // Get the output embeddings
        let embd = unsafe { mtmd_get_output_embd(self.ptr.as_ptr()) };
        if embd.is_null() {
            return Err(ForgeError::ImageProcessingFailed(
                "mtmd_get_output_embd returned null".to_string(),
            ));
        }

        // Get number of tokens in this chunk
        let n_tokens = unsafe { mtmd_input_chunk_get_n_tokens(chunk) } as i32;
        log::info!("Image has {} tokens worth of embeddings", n_tokens);

        // Get embedding dimension from model
        let model = unsafe { llama_get_model(lctx) };
        let n_embd = unsafe { llama_cpp_sys::llama_model_n_embd_inp(model) } as i32;

        log::debug!("Embedding dimension: {}", n_embd);

        // Decode embeddings in batches
        // We need to use batch.embd instead of batch.token
        let mut batch = unsafe { llama_batch_init(n_batch, n_embd, 1) };

        let mut i = 0i32;
        while i < n_tokens {
            batch.n_tokens = 0;

            while i < n_tokens && batch.n_tokens < n_batch {
                let j = batch.n_tokens as isize;

                // Copy embedding data for this token
                unsafe {
                    let src = embd.offset((i as isize) * (n_embd as isize));
                    let dst = batch.embd.offset(j * (n_embd as isize));
                    std::ptr::copy_nonoverlapping(src, dst, n_embd as usize);

                    *batch.pos.offset(j) = n_past;
                    *batch.n_seq_id.offset(j) = 1;
                    *(*batch.seq_id.offset(j)).offset(0) = seq_id;
                    *batch.logits.offset(j) = 0;
                }

                n_past += 1;
                batch.n_tokens += 1;
                i += 1;
            }

            // Set logits for last token if needed
            let is_last_batch = i == n_tokens;
            if logits_last && is_last_batch && batch.n_tokens > 0 {
                unsafe {
                    *batch.logits.offset((batch.n_tokens - 1) as isize) = 1;
                }
            }

            // Decode
            let ret = unsafe { llama_decode(lctx, batch) };
            if ret != 0 {
                unsafe {
                    llama_batch_free(batch);
                }
                return Err(ForgeError::ImageProcessingFailed(format!(
                    "llama_decode failed for image embeddings with code {}",
                    ret
                )));
            }
        }

        unsafe {
            llama_batch_free(batch);
        }

        log::info!("Image chunk evaluated, n_past = {}", n_past);

        Ok(n_past)
    }

    /// Evaluate an audio chunk (encode + decode embeddings)
    /// Audio uses the same encoding pipeline as images in mtmd
    fn eval_audio_chunk(
        &mut self,
        lctx: *mut llama_context,
        chunk: *const mtmd_input_chunk,
        mut n_past: llama_pos,
        seq_id: llama_seq_id,
        n_batch: i32,
        logits_last: bool,
    ) -> Result<llama_pos> {
        log::info!("Encoding audio chunk...");

        // Encode the audio chunk (same API as image)
        let ret = unsafe { mtmd_encode_chunk(self.ptr.as_ptr(), chunk) };
        if ret != 0 {
            return Err(ForgeError::AudioProcessingFailed(format!(
                "mtmd_encode_chunk (audio) failed with code {}",
                ret
            )));
        }

        log::info!("Audio encoded, getting embeddings...");

        // Get the output embeddings
        let embd = unsafe { mtmd_get_output_embd(self.ptr.as_ptr()) };
        if embd.is_null() {
            return Err(ForgeError::AudioProcessingFailed(
                "mtmd_get_output_embd returned null".to_string(),
            ));
        }

        // Get number of tokens in this chunk
        let n_tokens = unsafe { mtmd_input_chunk_get_n_tokens(chunk) } as i32;
        log::info!("Audio has {} tokens worth of embeddings", n_tokens);

        // Get embedding dimension from model
        let model = unsafe { llama_get_model(lctx) };
        let n_embd = unsafe { llama_cpp_sys::llama_model_n_embd_inp(model) } as i32;

        log::debug!("Embedding dimension: {}", n_embd);

        // Decode embeddings in batches (same as image)
        let mut batch = unsafe { llama_batch_init(n_batch, n_embd, 1) };

        let mut i = 0i32;
        while i < n_tokens {
            batch.n_tokens = 0;

            while i < n_tokens && batch.n_tokens < n_batch {
                let j = batch.n_tokens as isize;

                unsafe {
                    let src = embd.offset((i as isize) * (n_embd as isize));
                    let dst = batch.embd.offset(j * (n_embd as isize));
                    std::ptr::copy_nonoverlapping(src, dst, n_embd as usize);

                    *batch.pos.offset(j) = n_past;
                    *batch.n_seq_id.offset(j) = 1;
                    *(*batch.seq_id.offset(j)).offset(0) = seq_id;
                    *batch.logits.offset(j) = 0;
                }

                n_past += 1;
                batch.n_tokens += 1;
                i += 1;
            }

            // Set logits for last token if needed
            let is_last_batch = i == n_tokens;
            if logits_last && is_last_batch && batch.n_tokens > 0 {
                unsafe {
                    *batch.logits.offset((batch.n_tokens - 1) as isize) = 1;
                }
            }

            // Decode
            let ret = unsafe { llama_decode(lctx, batch) };
            if ret != 0 {
                unsafe {
                    llama_batch_free(batch);
                }
                return Err(ForgeError::AudioProcessingFailed(format!(
                    "llama_decode failed for audio embeddings with code {}",
                    ret
                )));
            }
        }

        unsafe {
            llama_batch_free(batch);
        }

        log::info!("Audio chunk evaluated, n_past = {}", n_past);

        Ok(n_past)
    }

    /// Process raw RGBA image pixels and get input chunks for the model
    ///
    /// # Arguments
    ///
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `rgba_data` - Raw RGBA pixel data (4 bytes per pixel)
    /// * `prompt` - Text prompt to combine with the image (should contain image marker)
    ///
    /// # Returns
    ///
    /// Input chunks ready for evaluation
    pub fn process_image_rgba(
        &self,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        prompt: &str,
    ) -> Result<InputChunks> {
        log::info!(
            "Processing image: {}x{} ({} bytes)",
            width,
            height,
            rgba_data.len()
        );

        let expected_size = validated_rgba_len(width, height)?;
        if rgba_data.len() != expected_size {
            return Err(ForgeError::ImageProcessingFailed(format!(
                "Expected {} bytes for {}x{} RGBA, got {}",
                expected_size,
                width,
                height,
                rgba_data.len()
            )));
        }

        log::debug!(
            "Processing RGBA image: {}x{}, prompt: {} chars",
            width,
            height,
            prompt.len()
        );

        // Convert RGBA to RGB (mtmd_bitmap_init expects RGB, not RGBA!)
        let pixel_count = validated_pixel_count(width, height)?;
        let rgb_capacity = pixel_count.checked_mul(3).ok_or_else(|| {
            ForgeError::ImageProcessingFailed("RGB buffer size overflow".to_string())
        })?;
        let mut rgb_data = Vec::with_capacity(rgb_capacity);
        for i in 0..pixel_count {
            rgb_data.push(rgba_data[i * 4]); // R
            rgb_data.push(rgba_data[i * 4 + 1]); // G
            rgb_data.push(rgba_data[i * 4 + 2]); // B
                                                 // Skip alpha (rgba_data[i * 4 + 3])
        }

        log::debug!(
            "Converted RGBA ({} bytes) to RGB ({} bytes)",
            rgba_data.len(),
            rgb_data.len()
        );

        // Create bitmap from RGB pixels
        let bitmap = unsafe { mtmd_bitmap_init(width, height, rgb_data.as_ptr()) };

        if bitmap.is_null() {
            return Err(ForgeError::ImageProcessingFailed(
                "mtmd_bitmap_init returned null".to_string(),
            ));
        }

        // Log actual dimensions from bitmap
        let actual_w = unsafe { mtmd_bitmap_get_nx(bitmap) };
        let actual_h = unsafe { mtmd_bitmap_get_ny(bitmap) };
        log::debug!("Bitmap created: {}x{}", actual_w, actual_h);

        // Create chunks container
        let chunks = unsafe { mtmd_input_chunks_init() };
        if chunks.is_null() {
            unsafe {
                mtmd_bitmap_free(bitmap);
            }
            return Err(ForgeError::ImageProcessingFailed(
                "Failed to create input chunks".to_string(),
            ));
        }

        // Prepare text input struct
        let c_prompt = CString::new(prompt)?;
        let text_input = mtmd_input_text {
            text: c_prompt.as_ptr(),
            text_len: prompt.len(),
            add_special: true,
            parse_special: true,
        };

        // Create array of bitmap pointers
        let bitmap_ptr: *const _ = bitmap;
        let mut bitmap_array = [bitmap_ptr];

        // Tokenize
        let result = unsafe {
            mtmd_tokenize(
                self.ptr.as_ptr(),
                chunks,
                &text_input,
                bitmap_array.as_mut_ptr(),
                1, // n_bitmaps
            )
        };

        // Free bitmap (tokenization makes a copy)
        unsafe {
            mtmd_bitmap_free(bitmap);
        }

        if result != 0 {
            unsafe {
                mtmd_input_chunks_free(chunks);
            }
            return Err(ForgeError::ImageProcessingFailed(format!(
                "mtmd_tokenize failed with code {}",
                result
            )));
        }

        let n_chunks = unsafe { mtmd_input_chunks_size(chunks) };
        log::info!("Image tokenized: {} chunks", n_chunks);

        Ok(InputChunks {
            ptr: NonNull::new(chunks).unwrap(),
            n_chunks,
        })
    }

    /// Process raw PCM audio samples and get input chunks for the model
    ///
    /// # Arguments
    ///
    /// * `samples` - Raw PCM audio samples (f32, normalized -1.0 to 1.0)
    /// * `prompt` - Text prompt to combine with the audio (should contain media marker)
    ///
    /// # Returns
    ///
    /// Input chunks ready for evaluation
    ///
    /// # Note
    ///
    /// The audio should be at the sample rate expected by the model.
    /// Use `audio_sample_rate()` to get the expected rate.
    pub fn process_audio(&self, samples: &[f32], prompt: &str) -> Result<InputChunks> {
        if !self.supports_audio() {
            return Err(ForgeError::AudioProcessingFailed(
                "This model does not support audio input".to_string(),
            ));
        }

        log::info!("Processing audio: {} samples", samples.len());

        if samples.is_empty() {
            return Err(ForgeError::AudioProcessingFailed(
                "Empty audio samples".to_string(),
            ));
        }

        // Create audio bitmap from PCM samples
        let bitmap = unsafe { mtmd_bitmap_init_from_audio(samples.len(), samples.as_ptr()) };

        if bitmap.is_null() {
            return Err(ForgeError::AudioProcessingFailed(
                "mtmd_bitmap_init_from_audio returned null".to_string(),
            ));
        }

        log::debug!("Audio bitmap created: {} samples", samples.len());

        // Create chunks container
        let chunks = unsafe { mtmd_input_chunks_init() };
        if chunks.is_null() {
            unsafe {
                mtmd_bitmap_free(bitmap);
            }
            return Err(ForgeError::AudioProcessingFailed(
                "Failed to create input chunks".to_string(),
            ));
        }

        // Prepare text input struct
        let c_prompt = CString::new(prompt)?;
        let text_input = mtmd_input_text {
            text: c_prompt.as_ptr(),
            text_len: prompt.len(),
            add_special: true,
            parse_special: true,
        };

        // Create array of bitmap pointers (audio uses same API as image)
        let bitmap_ptr: *const _ = bitmap;
        let mut bitmap_array = [bitmap_ptr];

        // Tokenize
        let result = unsafe {
            mtmd_tokenize(
                self.ptr.as_ptr(),
                chunks,
                &text_input,
                bitmap_array.as_mut_ptr(),
                1, // n_bitmaps (audio counts as 1 "bitmap")
            )
        };

        // Free bitmap (tokenization makes a copy)
        unsafe {
            mtmd_bitmap_free(bitmap);
        }

        if result != 0 {
            unsafe {
                mtmd_input_chunks_free(chunks);
            }
            return Err(ForgeError::AudioProcessingFailed(format!(
                "mtmd_tokenize (audio) failed with code {}",
                result
            )));
        }

        let n_chunks = unsafe { mtmd_input_chunks_size(chunks) };
        log::info!("Audio tokenized: {} chunks", n_chunks);

        Ok(InputChunks {
            ptr: NonNull::new(chunks).unwrap(),
            n_chunks,
        })
    }
}

impl Drop for MultimodalContext {
    fn drop(&mut self) {
        log::debug!("Freeing multimodal context");
        unsafe {
            mtmd_free(self.ptr.as_ptr());
        }
        log::debug!("Multimodal context freed");
    }
}

/// Tokenized input chunks (text + image)
pub struct InputChunks {
    ptr: NonNull<mtmd_input_chunks>,
    /// Number of chunks (text chunks + image chunks)
    n_chunks: usize,
}

// Safety: InputChunks owns the pointer
unsafe impl Send for InputChunks {}

impl InputChunks {
    /// Get the raw pointer
    pub fn as_ptr(&self) -> *mut mtmd_input_chunks {
        self.ptr.as_ptr()
    }

    /// Get the number of chunks
    pub fn n_chunks(&self) -> usize {
        self.n_chunks
    }
}

impl Drop for InputChunks {
    fn drop(&mut self) {
        log::debug!("Freeing input chunks");
        unsafe {
            mtmd_input_chunks_free(self.ptr.as_ptr());
        }
    }
}

// ============================================================================
// MediaEncoder trait implementation
// ============================================================================

use crate::traits::{EncoderCapabilities, MediaEncoder, MediaMarker};

impl MediaEncoder for MultimodalContext {
    fn media_marker(&self) -> MediaMarker {
        get_default_media_marker()
    }

    fn capabilities(&self) -> EncoderCapabilities {
        EncoderCapabilities {
            vision: self.supports_vision(),
            audio: self.supports_audio(),
            // Audio output is handled by the AudioDecoder (vocoder), not the encoder
            audio_output: false,
        }
    }
}
