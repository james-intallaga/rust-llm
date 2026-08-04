//! Encoder trait for converting media to embeddings
//!
//! Encoders take raw media data (images, audio, video) and convert them
//! to embeddings that can be processed by the LLM.

/// Marker string for media in prompts (e.g., "<|image_start|>")
pub type MediaMarker = String;

/// Capability flags for encoders
#[derive(Debug, Clone, Copy, Default)]
pub struct EncoderCapabilities {
    /// Supports vision/image input
    pub vision: bool,
    /// Supports audio input
    pub audio: bool,
    /// Supports audio output (speech-to-speech)
    pub audio_output: bool,
}

/// Trait for querying encoder capabilities
///
/// This is a simpler trait that encoders can implement to advertise
/// what media types they support. The actual processing is done
/// through the encoder's own methods since different media types
/// have different processing patterns.
pub trait MediaEncoder: Send {
    /// Returns the media marker string used in prompts
    fn media_marker(&self) -> MediaMarker;

    /// Returns the capabilities of this encoder
    fn capabilities(&self) -> EncoderCapabilities;
}
