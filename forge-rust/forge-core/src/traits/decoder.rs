//! Decoder trait for converting LLM outputs to media
//!
//! Decoders take token embeddings from the LLM and convert them
//! to media output (audio waveforms, etc.)

use crate::error::Result;

/// Audio format for output
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioFormat {
    /// Sample rate in Hz (e.g., 24000)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u8,
}

/// Trait for media decoders (vocoders)
///
/// Implement this trait to add support for new output modalities.
/// The decoder receives token embeddings and produces raw media data.
pub trait Decoder: Send + Sync {
    /// The type of output this decoder produces
    type Output;

    /// Returns the audio format for audio decoders
    fn audio_format(&self) -> Option<AudioFormat> {
        None
    }

    /// Decode a single token's embedding to output
    ///
    /// For audio decoders, this produces a small chunk of audio samples.
    /// Called repeatedly during streaming generation.
    fn decode_step(&mut self, token_embedding: &[f32]) -> Result<Self::Output>;

    /// Finalize decoding and return any remaining output
    ///
    /// Called when generation is complete to flush any buffered data.
    fn finalize(&mut self) -> Result<Option<Self::Output>>;

    /// Reset decoder state for a new generation session
    fn reset(&mut self);
}
