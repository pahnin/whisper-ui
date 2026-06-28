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
        eprintln!("[WHISPER] new() START - model_path={:?}", model_path);
        let whisper_params = whisper_rs::WhisperContextParameters::default();
        eprintln!("[WHISPER] new() calling WhisperContext::new_with_params()");
        let ctx = WhisperContext::new_with_params(&model_path, whisper_params)
            .map_err(|e| BackendError::Internal(format!("Failed to load model: {}", e)))?;
        eprintln!("[WHISPER] new() ctx created successfully");
        eprintln!("[WHISPER] new() calling ctx.create_state()");
        let state = ctx
            .create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to init state: {}", e)))?;
        eprintln!("[WHISPER] new() state created successfully, returning WhisperBackend");
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
        eprintln!("[WHISPER] transcribe_segment_sync() START - audio_data.len()={}", audio_data.len());
        if audio_data.is_empty() {
            eprintln!("[WHISPER] transcribe_segment_sync() returning early (empty audio)");
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

        eprintln!("[WHISPER] transcribe_segment_sync() calling state.full()");
        self.state
            .full(params, audio_data)
            .map_err(|e| BackendError::TranscriptionFailed(format!("full: {}", e)))?;
        eprintln!("[WHISPER] transcribe_segment_sync() state.full() done");

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

        eprintln!("[WHISPER] transcribe_segment_sync() returning Ok(segment)");
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
