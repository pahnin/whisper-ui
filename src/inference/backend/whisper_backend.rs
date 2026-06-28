use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

use super::{BackendError, BackendParams, TranscriptSegment, TranscriptionBackend};

static LOGGING_INITIALIZED: OnceLock<()> = OnceLock::new();

pub struct WhisperBackend {
    #[allow(dead_code)]
    ctx: WhisperContext,
    state: WhisperState,
    language: Option<String>,
    pub accelerator: String,
    /// Text accumulated across chunk boundaries for context preservation.
    accumulated_text: String,
}

impl WhisperBackend {
    pub fn new(model_path: PathBuf, params: BackendParams) -> Result<Self, BackendError> {
        LOGGING_INITIALIZED.get_or_init(|| whisper_rs::install_logging_hooks());

        let gpu_params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(&model_path, gpu_params);

        let (ctx, accelerator) = match ctx {
            Ok(ctx) => {
                eprintln!("[WHISPER] Loaded model with GPU acceleration");
                (ctx, "GPU".to_string())
            }
            Err(e) => {
                eprintln!("[WHISPER] GPU init failed ({}), falling back to CPU", e);
                let cpu_params = WhisperContextParameters {
                    use_gpu: false,
                    ..WhisperContextParameters::default()
                };
                let ctx = WhisperContext::new_with_params(&model_path, cpu_params)
                    .map_err(|e| BackendError::Internal(format!("Failed to load model (CPU): {}", e)))?;
                eprintln!("[WHISPER] Loaded model with CPU fallback");
                (ctx, "CPU".to_string())
            }
        };

        let state = ctx
            .create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to init state: {}", e)))?;

        Ok(Self {
            ctx,
            state,
            language: params.language,
            accelerator,
            accumulated_text: String::new(),
        })
    }

    pub fn new_cpu(model_path: PathBuf, params: BackendParams) -> Result<Self, BackendError> {
        LOGGING_INITIALIZED.get_or_init(|| whisper_rs::install_logging_hooks());
        let whisper_params = WhisperContextParameters {
            use_gpu: false,
            ..WhisperContextParameters::default()
        };
        let ctx = WhisperContext::new_with_params(&model_path, whisper_params)
            .map_err(|e| BackendError::Internal(format!("Failed to load model: {}", e)))?;
        let state = ctx
            .create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to init state: {}", e)))?;
        Ok(Self {
            ctx,
            state,
            language: params.language,
            accelerator: "CPU".to_string(),
            accumulated_text: String::new(),
        })
    }

    /// Synchronous transcription with Whisper's native context passing.
    /// `set_no_context(false)` retains the decoder's KV-cache across calls,
    /// allowing the model to carry forward attention state from previous
    /// chunks. A short prompt (last 150 chars) provides linguistic context.
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

        eprintln!("[WHISPER CHUNK] {} samples ({:.2}s)", audio_data.len(), audio_data.len() as f64 / 16000.0);

        let strategy = SamplingStrategy::Greedy { best_of: 1 };
        let mut params = FullParams::new(strategy);
        params.set_language(self.language.as_deref());
        params.set_n_threads(4);
        // Enable Whisper's native context retention — carries forward the
        // decoder's KV-cache so each chunk benefits from previous decoding state.
        params.set_no_context(false);

        // Use last 150 chars as initial prompt for linguistic context.
        if !self.accumulated_text.is_empty() {
            let prompt = if self.accumulated_text.len() > 150 {
                &self.accumulated_text[self.accumulated_text.len() - 150..]
            } else {
                &self.accumulated_text
            };
            eprintln!("[WHISPER PROMPT] '{}'", prompt);
            params.set_initial_prompt(prompt);
        }

        self.state
            .full(params, audio_data)
            .map_err(|e| BackendError::TranscriptionFailed(format!("full: {}", e)))?;

        let mut all_text = String::new();
        for segment in self.state.as_iter() {
            let text = segment.to_str_lossy().map_err(|e| BackendError::TranscriptionFailed(format!("segment to_str_lossy: {}", e)))?;
            eprintln!("[WHISPER RAW] '{}'", text.trim());
            if !text.is_empty() {
                let trimmed = text.trim().to_string();
                if !all_text.is_empty() {
                    all_text.push(' ');
                }
                all_text.push_str(&trimmed);
            }
        }

        // Update accumulated text — only append if this is new text
        if !all_text.is_empty() {
            let trimmed_new = all_text.trim();
            if self.accumulated_text.is_empty()
                || !self.accumulated_text.trim().ends_with(trimmed_new)
            {
                if !self.accumulated_text.is_empty() && !self.accumulated_text.trim().is_empty() {
                    self.accumulated_text.push(' ');
                }
                self.accumulated_text.push_str(trimmed_new);
                eprintln!("[WHISPER ACCUMULATED] '{}'", self.accumulated_text.trim());
            } else {
                eprintln!("[WHISPER DUPLICATE] Skipped (already in accumulated)");
            }
        }

        if !all_text.is_empty() {
            eprintln!("[WHISPER] {}", all_text);
        }

        let segment = TranscriptSegment {
            text: all_text,
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

    fn accelerator(&self) -> &str {
        &self.accelerator
    }
}
