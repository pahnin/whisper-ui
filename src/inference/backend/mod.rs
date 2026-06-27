use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

// ===== Trait definitions and types =====

#[derive(Debug, Clone)]
pub struct BackendParams {
    pub language: Option<String>,
    pub beam_size: u32,
    pub vad_enabled: bool,
}

impl Default for BackendParams {
    fn default() -> Self {
        Self {
            language: Some("en".to_string()),
            beam_size: 1,
            vad_enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptSegment {
    pub text: String,
    pub start: Duration,
    pub end: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("model not loaded")]
    ModelNotLoaded,
    #[error("invalid audio data: {0}")]
    InvalidAudio(String),
    #[error("transcription failed: {0}")]
    TranscriptionFailed(String),
    #[error("backend error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait TranscriptionBackend: Send + 'static {
    type Error: std::error::Error + Send + Sync;

    async fn new(model_path: PathBuf, params: BackendParams) -> Result<Self, Self::Error>
    where
        Self: Sized;

    async fn transcribe_segment(
        &mut self,
        audio_data: &[f32],
    ) -> Result<TranscriptSegment, Self::Error>;

    fn supported_languages(&self) -> Vec<String>;
    fn language_codes(&self) -> Vec<String>;
    fn is_ready(&self) -> bool;
}

// ===== Re-exports =====

pub mod whisper_backend;
pub mod model_manager;

pub use whisper_backend::WhisperBackend;
