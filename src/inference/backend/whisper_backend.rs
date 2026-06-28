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
        eprintln!("[WHISPER] Model loaded: {:?}", model_path);
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

    /// Synchronous transcription using WhisperState::full() convenience method.
    /// The same WhisperState instance is reused across calls — no reset needed.
    /// pcm_to_mel() on a fresh params call clears internal buffers automatically.
    pub fn transcribe_segment_sync(
        &mut self,
        audio_data: &[f32],
    ) -> Result<TranscriptSegment, BackendError> {
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
            .full(params, audio_data)
            .map_err(|e| BackendError::TranscriptionFailed(format!("full: {}", e)))?;

        let mut full_text = String::new();
        for segment in self.state.as_iter() {
            let text = segment.to_str_lossy().unwrap_or_default();
            if !text.is_empty() {
                let start_ms = segment.start_timestamp();
                let end_ms = segment.end_timestamp();
                let start = format!("{:}:{:02}", start_ms / 60000, (start_ms % 60000) / 1000);
                let end = format!("{:}:{:02}", end_ms / 60000, (end_ms % 60000) / 1000);
                eprintln!("[WHISPER] [{}-{}] {}", start, end, text.trim());
                if !full_text.is_empty() {
                    full_text.push('\n');
                }
                full_text.push_str(&text);
            }
        }

        let segment = TranscriptSegment {
            text: full_text,
            start: Duration::ZERO,
            end: Duration::ZERO,
        };

        Ok(segment)
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
        self.transcribe_segment_sync(audio_data)
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

    fn language_codes(&self) -> Vec<String> {
        let mut codes = Vec::new();
        for i in 0..=whisper_rs::get_lang_max_id() {
            if let Some(code) = whisper_rs::get_lang_str(i) {
                codes.push(code.to_string());
            }
        }
        codes
    }

    fn is_ready(&self) -> bool {
        true
    }
}
