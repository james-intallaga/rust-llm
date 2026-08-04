//! Audio Decoder (Vocoder)
//!
//! This module contains the ISTFT (Inverse Short-Time Fourier Transform)
//! vocoder implementation for converting LLM output embeddings to audio.
//!
//! ## Algorithm
//!
//! The vocoder uses streaming overlap-add ISTFT:
//! 1. Model outputs magnitude/phase embeddings per frame
//! 2. Convert to complex spectrum
//! 3. Mirror for Hermitian symmetry
//! 4. Inverse FFT
//! 5. Apply Hann window
//! 6. Overlap-add accumulation
//! 7. Normalize and extract samples
//!
//! ## LFM2.5-Audio Parameters
//!
//! - n_fft: 1280
//! - hop_length: 320
//! - sample_rate: 24000 Hz

use std::f32::consts::PI;
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

use crate::error::{ForgeError, Result};
use crate::traits::{AudioFormat, Decoder};

/// Default FFT size for LFM2.5-Audio
pub const DEFAULT_N_FFT: usize = 1280;

/// Default hop length for LFM2.5-Audio
pub const DEFAULT_HOP_LENGTH: usize = 320;

/// Default sample rate for LFM2.5-Audio
pub const DEFAULT_SAMPLE_RATE: u32 = 24000;

/// Audio output chunk (PCM samples)
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// Raw PCM samples (f32, normalized -1.0 to 1.0)
    pub samples: Vec<f32>,
}

impl AudioChunk {
    /// Create a new audio chunk
    pub fn new(samples: Vec<f32>) -> Self {
        Self { samples }
    }

    /// Check if chunk is empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Get number of samples
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}

/// Streaming ISTFT vocoder for audio synthesis
///
/// Converts spectral embeddings from the LLM to audio samples
/// using overlap-add inverse STFT.
pub struct StreamingIstft {
    /// FFT size
    n_fft: usize,
    /// Hop length (samples between frames)
    hop_length: usize,
    /// Number of FFT bins (n_fft / 2 + 1)
    n_fft_bins: usize,
    /// Precomputed Hann window
    hann_window: Vec<f32>,
    /// Inverse FFT planner
    ifft: Arc<dyn Fft<f32>>,
    /// Overlap buffer for reconstruction
    overlap_buffer: Vec<f32>,
    /// Window sum buffer for normalization
    window_sum_buffer: Vec<f32>,
    /// Padding samples to remove from start
    padding_to_remove: usize,
    /// Scratch buffer for FFT computation
    scratch: Vec<Complex<f32>>,
}

impl StreamingIstft {
    /// Create a new streaming ISTFT vocoder
    ///
    /// # Arguments
    /// * `n_fft` - FFT size (e.g., 1280 for LFM2.5-Audio)
    /// * `hop_length` - Hop length in samples (e.g., 320)
    pub fn new(n_fft: usize, hop_length: usize) -> Self {
        // Create FFT planner and get inverse FFT
        let mut planner = FftPlanner::new();
        let ifft = planner.plan_fft_inverse(n_fft);
        let scratch_len = ifft.get_inplace_scratch_len();

        // Precompute Hann window (periodic)
        let hann_window: Vec<f32> = (0..n_fft)
            .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / n_fft as f32).cos()))
            .collect();

        Self {
            n_fft,
            hop_length,
            n_fft_bins: n_fft / 2 + 1,
            hann_window,
            ifft,
            overlap_buffer: vec![0.0; n_fft],
            window_sum_buffer: vec![0.0; n_fft],
            padding_to_remove: (n_fft - hop_length) / 2,
            scratch: vec![Complex::new(0.0, 0.0); scratch_len],
        }
    }

    /// Create with default LFM2.5-Audio parameters
    pub fn new_default() -> Self {
        Self::new(DEFAULT_N_FFT, DEFAULT_HOP_LENGTH)
    }

    /// Reset the vocoder state for a new audio stream
    pub fn reset(&mut self) {
        self.overlap_buffer.fill(0.0);
        self.window_sum_buffer.fill(0.0);
        self.padding_to_remove = (self.n_fft - self.hop_length) / 2;
    }

    /// Process a single spectral frame
    ///
    /// # Arguments
    /// * `frame_spectrum` - Complex spectrum as interleaved [real, imag, real, imag, ...]
    ///   Length should be n_fft_bins * 2
    ///
    /// # Returns
    /// Audio samples for this frame (up to hop_length samples)
    pub fn process_frame(&mut self, frame_spectrum: &[f32]) -> Result<Vec<f32>> {
        // Validate input size
        let expected_len = self.n_fft_bins * 2;
        if frame_spectrum.len() != expected_len {
            return Err(ForgeError::InvalidParameter(format!(
                "Expected {} values ({}*2), got {}",
                expected_len,
                self.n_fft_bins,
                frame_spectrum.len()
            )));
        }

        // Build full complex spectrum with Hermitian symmetry
        let mut spectrum: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); self.n_fft];

        // Copy positive frequencies
        for j in 0..self.n_fft_bins {
            spectrum[j] = Complex::new(frame_spectrum[j * 2], frame_spectrum[j * 2 + 1]);
        }

        // Mirror negative frequencies (conjugate symmetry)
        for j in 1..(self.n_fft_bins - 1) {
            let mirror_idx = self.n_fft - j;
            spectrum[mirror_idx] = Complex::new(
                spectrum[j].re,
                -spectrum[j].im, // conjugate
            );
        }

        // Perform inverse FFT in-place
        self.ifft
            .process_with_scratch(&mut spectrum, &mut self.scratch);

        // Apply Hann window and accumulate into overlap buffer
        // Also accumulate window^2 for normalization
        for (j, value) in spectrum.iter().enumerate().take(self.n_fft) {
            let window = self.hann_window[j];
            // rustfft doesn't normalize, so we divide by n_fft
            let sample = value.re / self.n_fft as f32;

            self.window_sum_buffer[j] += window * window;
            self.overlap_buffer[j] += sample * window;
        }

        // Extract hop_length samples with normalization
        let mut output = Vec::with_capacity(self.hop_length);
        for i in 0..self.hop_length {
            let sample = if self.window_sum_buffer[i] > 1e-8 {
                self.overlap_buffer[i] / self.window_sum_buffer[i]
            } else {
                self.overlap_buffer[i]
            };
            output.push(sample);
        }

        // Shift buffers left by hop_length
        self.overlap_buffer.copy_within(self.hop_length.., 0);
        self.overlap_buffer[self.n_fft - self.hop_length..].fill(0.0);

        self.window_sum_buffer.copy_within(self.hop_length.., 0);
        self.window_sum_buffer[self.n_fft - self.hop_length..].fill(0.0);

        // Remove padding samples if needed (for initial frames)
        let to_remove = self.padding_to_remove.min(output.len());
        if to_remove > 0 {
            self.padding_to_remove -= to_remove;
            output.drain(0..to_remove);
        }

        Ok(output)
    }

    /// Process a frame from magnitude/phase embeddings
    ///
    /// This is the format output by models like LFM2.5-Audio:
    /// - First half: log magnitude
    /// - Second half: phase
    ///
    /// # Arguments
    /// * `embeddings` - Embeddings from the model (length = n_embd)
    ///
    /// # Returns
    /// Audio samples for this frame
    pub fn process_embedding(&mut self, embeddings: &[f32]) -> Result<Vec<f32>> {
        let n_embd = embeddings.len();
        if !n_embd.is_multiple_of(2) {
            return Err(ForgeError::InvalidParameter(format!(
                "Embedding size must be even, got {}",
                n_embd
            )));
        }

        let half = n_embd / 2;
        if half != self.n_fft_bins {
            return Err(ForgeError::InvalidParameter(format!(
                "Expected {} frequency bins, got {}",
                self.n_fft_bins, half
            )));
        }

        // Convert magnitude/phase to complex spectrum
        let mut spectrum = Vec::with_capacity(self.n_fft_bins * 2);
        for k in 0..half {
            let log_mag = embeddings[k];
            let phase = embeddings[k + half];

            // exp(log_mag) with clamping to avoid overflow
            let mag = log_mag.exp().min(100.0);

            // Convert to real/imag
            let real = mag * phase.cos();
            let imag = mag * phase.sin();

            spectrum.push(real);
            spectrum.push(imag);
        }

        self.process_frame(&spectrum)
    }

    /// Flush remaining samples at end of stream
    ///
    /// Call this after processing all frames to get the final samples
    /// that are still in the overlap buffer.
    pub fn flush(&mut self) -> Vec<f32> {
        let mut output = Vec::new();
        let mut remaining = self.n_fft - self.hop_length;

        while remaining > 0 {
            let chunk_size = remaining.min(self.hop_length);

            for i in 0..chunk_size {
                let sample = if self.window_sum_buffer[i] > 1e-8 {
                    self.overlap_buffer[i] / self.window_sum_buffer[i]
                } else {
                    self.overlap_buffer[i]
                };
                output.push(sample);
            }

            // Shift buffers
            self.overlap_buffer.copy_within(chunk_size.., 0);
            self.overlap_buffer[self.n_fft - chunk_size..].fill(0.0);

            self.window_sum_buffer.copy_within(chunk_size.., 0);
            self.window_sum_buffer[self.n_fft - chunk_size..].fill(0.0);

            remaining -= chunk_size;
        }

        output
    }

    /// Get the expected embedding size per frame
    pub fn embedding_size(&self) -> usize {
        self.n_fft_bins * 2
    }

    /// Get the hop length
    pub fn hop_length(&self) -> usize {
        self.hop_length
    }

    /// Get the FFT size
    pub fn n_fft(&self) -> usize {
        self.n_fft
    }
}

/// Audio decoder that wraps StreamingIstft
///
/// Implements the Decoder trait for integration with the engine.
pub struct AudioDecoder {
    /// The underlying ISTFT vocoder
    istft: StreamingIstft,
    /// Sample rate in Hz
    sample_rate: u32,
}

impl AudioDecoder {
    /// Create a new audio decoder with default LFM2.5-Audio parameters
    pub fn new() -> Result<Self> {
        Ok(Self {
            istft: StreamingIstft::new_default(),
            sample_rate: DEFAULT_SAMPLE_RATE,
        })
    }

    /// Create with custom parameters
    pub fn with_params(n_fft: usize, hop_length: usize, sample_rate: u32) -> Result<Self> {
        Ok(Self {
            istft: StreamingIstft::new(n_fft, hop_length),
            sample_rate,
        })
    }

    /// Get the expected embedding size per frame
    pub fn embedding_size(&self) -> usize {
        self.istft.embedding_size()
    }

    /// Process embeddings and return audio
    pub fn process_embeddings(&mut self, embeddings: &[f32]) -> Result<AudioChunk> {
        let samples = self.istft.process_embedding(embeddings)?;
        Ok(AudioChunk::new(samples))
    }

    /// Flush remaining audio
    pub fn flush_audio(&mut self) -> AudioChunk {
        AudioChunk::new(self.istft.flush())
    }
}

impl Default for AudioDecoder {
    fn default() -> Self {
        Self::new().expect("Failed to create default AudioDecoder")
    }
}

impl Decoder for AudioDecoder {
    type Output = AudioChunk;

    fn audio_format(&self) -> Option<AudioFormat> {
        Some(AudioFormat {
            sample_rate: self.sample_rate,
            channels: 1,
        })
    }

    fn decode_step(&mut self, token_embedding: &[f32]) -> Result<Self::Output> {
        self.process_embeddings(token_embedding)
    }

    fn finalize(&mut self) -> Result<Option<Self::Output>> {
        let chunk = self.flush_audio();
        if chunk.is_empty() {
            Ok(None)
        } else {
            Ok(Some(chunk))
        }
    }

    fn reset(&mut self) {
        self.istft.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hann_window() {
        let istft = StreamingIstft::new(8, 2);

        // Hann window should be 0 at edges (periodic)
        assert!(istft.hann_window[0].abs() < 1e-6);

        // Should be 1.0 at center for even-length window
        let center = istft.n_fft / 2;
        assert!((istft.hann_window[center] - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_istft_creation() {
        let istft = StreamingIstft::new_default();
        assert_eq!(istft.n_fft, DEFAULT_N_FFT);
        assert_eq!(istft.hop_length, DEFAULT_HOP_LENGTH);
        assert_eq!(istft.n_fft_bins, DEFAULT_N_FFT / 2 + 1);
    }

    #[test]
    fn test_process_zero_frame() {
        let mut istft = StreamingIstft::new(8, 2);
        let n_bins = istft.n_fft_bins;

        // Zero spectrum should produce zero output
        let spectrum = vec![0.0f32; n_bins * 2];
        let result = istft.process_frame(&spectrum);
        assert!(result.is_ok());

        // Output should be mostly zeros (or very small due to initial padding removal)
        let output = result.unwrap();
        for sample in output {
            assert!(sample.abs() < 1e-6);
        }
    }

    #[test]
    fn test_decoder_trait() {
        let decoder = AudioDecoder::new().unwrap();
        let format = decoder.audio_format();
        assert!(format.is_some());
        assert_eq!(format.unwrap().sample_rate, DEFAULT_SAMPLE_RATE);
        assert_eq!(format.unwrap().channels, 1);
    }

    #[test]
    fn test_reset() {
        let mut istft = StreamingIstft::new(8, 2);

        // Process a frame to modify state
        let spectrum = vec![1.0f32; istft.n_fft_bins * 2];
        let _ = istft.process_frame(&spectrum);

        // Reset should clear state
        istft.reset();

        // Overlap buffer should be zero
        assert!(istft.overlap_buffer.iter().all(|&x| x == 0.0));
    }
}
