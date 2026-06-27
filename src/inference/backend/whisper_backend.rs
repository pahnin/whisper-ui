use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperState};

use super::{BackendError, BackendParams, TranscriptSegment, TranscriptionBackend};

pub struct WhisperBackend {
    ctx: WhisperContext,
    state: WhisperState,
    language: Option<String>,
}

impl WhisperBackend {
    pub fn new(model_path: PathBuf, params: BackendParams) -> Result<Self, BackendError> {
        let whisper_params = whisper_rs::WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(&model_path, whisper_params)
            .map_err(|e| BackendError::Internal(format!("Failed to load model: {}", e)))?;

        let state = ctx
            .create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to init state: {}", e)))?;

        Ok(Self {
            ctx,
            state,
            language: params.language,
        })
    }
}

#[async_trait]
impl TranscriptionBackend for WhisperBackend {
    type Error = BackendError;

    async fn new(model_path: PathBuf, params: BackendParams) -> Result<Self, Self::Error> {
        Self::new(model_path, params)
    }

    async fn transcribe_segment(
        &mut self,
        audio_data: &[f32],
    ) -> Result<TranscriptSegment, Self::Error> {
        if audio_data.is_empty() {
            return Ok(TranscriptSegment {
                text: String::new(),
                start: Duration::ZERO,
                end: Duration::ZERO,
            });
        }

        let strategy = SamplingStrategy::Greedy { best_of: 1 };
        let mut params = FullParams::new(strategy);
        params.set_language(self.language.as_deref());
        params.set_n_threads(4);

        self.state
            .pcm_to_mel(audio_data, 4)
            .map_err(|e| BackendError::TranscriptionFailed(format!("pcm_to_mel: {}", e)))?;

        self.state
            .encode(0, 4)
            .map_err(|e| BackendError::TranscriptionFailed(format!("encode: {}", e)))?;

        self.state
            .decode(&[], 0, 4)
            .map_err(|e| BackendError::TranscriptionFailed(format!("decode: {}", e)))?;

        let mut text_parts = Vec::new();
        for segment in self.state.as_iter() {
            text_parts.push(segment.to_str_lossy().unwrap_or_default().to_string());
        }

        let full_text = text_parts.join("\n");

        let segment = TranscriptSegment {
            text: full_text,
            start: Duration::ZERO,
            end: Duration::ZERO,
        };

        let new_state = self.ctx
            .create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to reset state: {}", e)))?;
        self.state = new_state;

        Ok(segment)
    }

    fn supported_languages(&self) -> Vec<String> {
        let mut langs = Vec::new();
        for i in 0..=whisper_rs::get_lang_max_id() {
            if let Some(lang) = whisper_rs::get_lang_str_full(i) {
                langs.push(lang.to_string());
            }
        }
        langs
    }

    fn is_ready(&self) -> bool {
        true
    }
}
