//! Types: Common data structures for multimodal pipelines
//!
//! This module contains shared type definitions used across
//! encoders, decoders, and engines.

/// Raw image data for vision models
#[derive(Debug, Clone)]
pub struct ImageInput {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Raw RGBA pixel data (4 bytes per pixel)
    pub rgba_data: Vec<u8>,
}

impl ImageInput {
    /// Create a new image input from RGBA data
    pub fn new(width: u32, height: u32, rgba_data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba_data,
        }
    }

    /// Validate the image data size
    pub fn is_valid(&self) -> bool {
        let expected = (self.width * self.height * 4) as usize;
        self.rgba_data.len() == expected
    }
}

/// Raw audio data for speech models
#[derive(Debug, Clone)]
pub struct AudioInput {
    /// Audio sample rate in Hz (typically 16000 for speech)
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u8,
    /// Raw PCM samples (f32, normalized -1.0 to 1.0)
    pub samples: Vec<f32>,
}

impl AudioInput {
    /// Create a new audio input
    pub fn new(sample_rate: u32, channels: u8, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels,
            samples,
        }
    }

    /// Duration in seconds
    pub fn duration_secs(&self) -> f32 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32)
    }
}
