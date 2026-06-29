use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use log;

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
    pub fallback_to_cpu: bool,
    /// Text accumulated across chunk boundaries for context preservation.
    accumulated_text: String,
    chunks_processed: usize,
}

impl WhisperBackend {
    pub fn new(model_path: PathBuf, params: BackendParams) -> Result<Self, BackendError> {
        if !model_path.exists() {
            return Err(BackendError::Internal(format!(
                "Model file not found: {}",
                model_path.display()
            )));
        }
        let metadata = std::fs::metadata(&model_path).map_err(|e| {
            BackendError::Internal(format!("Cannot read model file: {}", e))
        })?;
        if !metadata.is_file() {
            return Err(BackendError::Internal(format!(
                "Model path is not a file: {}",
                model_path.display()
            )));
        }
        if metadata.len() < 1_000_000 {
            return Err(BackendError::Internal(format!(
                "Model file too small ({} bytes), may be corrupted: {}",
                metadata.len(),
                model_path.display()
            )));
        }
        LOGGING_INITIALIZED.get_or_init(|| whisper_rs::install_logging_hooks());

        let gpu_params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(&model_path, gpu_params);

        let (ctx, accelerator) = match ctx {
            Ok(ctx) => {
                let system_info = whisper_rs::print_system_info();
                let acc_name = if system_info.contains("Metal") {
                    eprintln!("[WHISPER] Loaded model with Metal GPU acceleration");
                    "Metal".to_string()
                } else if system_info.contains("CUDA") {
                    eprintln!("[WHISPER] Loaded model with CUDA GPU acceleration");
                    "CUDA".to_string()
                } else if system_info.contains("Vulkan") {
                    eprintln!("[WHISPER] Loaded model with Vulkan GPU acceleration");
                    "Vulkan".to_string()
                } else {
                    eprintln!("[WHISPER] Loaded model with CPU backend");
                    "CPU".to_string()
                };
                (ctx, acc_name)
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
            fallback_to_cpu: false,
            accumulated_text: String::new(),
            chunks_processed: 0,
        })
    }

    pub fn new_cpu(model_path: PathBuf, params: BackendParams) -> Result<Self, BackendError> {
        if !model_path.exists() {
            return Err(BackendError::Internal(format!(
                "Model file not found: {}",
                model_path.display()
            )));
        }
        let metadata = std::fs::metadata(&model_path).map_err(|e| {
            BackendError::Internal(format!("Cannot read model file: {}", e))
        })?;
        if !metadata.is_file() {
            return Err(BackendError::Internal(format!(
                "Model path is not a file: {}",
                model_path.display()
            )));
        }
        if metadata.len() < 1_000_000 {
            return Err(BackendError::Internal(format!(
                "Model file too small ({} bytes), may be corrupted: {}",
                metadata.len(),
                model_path.display()
            )));
        }
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
            fallback_to_cpu: true,
            accumulated_text: String::new(),
            chunks_processed: 0,
        })
    }

    /// Synchronous transcription with Whisper's native context passing.
    /// `set_no_context(false)` retains the decoder's KV-cache across calls,
    /// allowing the model to carry forward attention state from previous
    /// chunks. A short prompt (last 150 chars) provides linguistic context.
    pub fn reset_state(&mut self) -> Result<(), BackendError> {
        self.state = self.ctx.create_state()
            .map_err(|e| BackendError::Internal(format!("Failed to reset state: {}", e)))?;
        Ok(())
    }

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

        log::debug!("[WHISPER CHUNK] {} samples ({:.2}s)", audio_data.len(), audio_data.len() as f64 / 16000.0);

        let strategy = SamplingStrategy::Greedy { best_of: 1 };
        let mut params = FullParams::new(strategy);
        params.set_language(self.language.as_deref());
        let n_threads = std::thread::available_parallelism()
            .map(|n| n.get().min(8).max(2))
            .unwrap_or(4) as i32;
        params.set_n_threads(n_threads);
        params.set_no_context(false);

        if !self.accumulated_text.is_empty() {
            let prompt = if self.accumulated_text.len() > 300 {
                &self.accumulated_text[self.accumulated_text.len() - 300..]
            } else {
                &self.accumulated_text
            };
            log::debug!("[WHISPER PROMPT] '{}'", prompt);
            params.set_initial_prompt(prompt);
        }

        let mut full_result = self.state.full(params.clone(), audio_data);
        if full_result.is_err() && !self.fallback_to_cpu {
            eprintln!("[WHISPER] GPU transcription failed, falling back to CPU");
            self.fallback_to_cpu = true;
            self.state = self.ctx.create_state()
                .map_err(|e| BackendError::Internal(format!("Failed to create CPU state: {}", e)))?;
            full_result = self.state.full(params, audio_data);
        }
        full_result.map_err(|e| BackendError::TranscriptionFailed(format!("full: {}", e)))?;

        let mut all_text = String::new();
        for segment in self.state.as_iter() {
            let text = segment.to_str_lossy().map_err(|e| BackendError::TranscriptionFailed(format!("segment to_str_lossy: {}", e)))?;
            log::debug!("[WHISPER RAW] '{}'", text.trim());
            if !text.is_empty() {
                let trimmed = text.trim().to_string();
                if !all_text.is_empty() {
                    all_text.push(' ');
                }
                all_text.push_str(&trimmed);
            }
        }

        if !all_text.is_empty() {
            let trimmed_new = all_text.trim();
            let acc_trimmed = self.accumulated_text.trim();
            
            if acc_trimmed.is_empty() {
                self.accumulated_text = trimmed_new.to_string();
            } else if trimmed_new.starts_with(acc_trimmed) {
                let rest = trimmed_new[acc_trimmed.len()..].trim();
                if !rest.is_empty() {
                    self.accumulated_text.push_str(rest);
                }
            } else if acc_trimmed.starts_with(trimmed_new) {
            } else {
                self.accumulated_text.push(' ');
                self.accumulated_text.push_str(trimmed_new);
            }
            
            log::debug!("[WHISPER ACCUMULATED] '{}'", self.accumulated_text.trim());
        }

        if self.accumulated_text.len() > 2048 {
            self.accumulated_text = self.accumulated_text[self.accumulated_text.len() - 2048..].to_string();
        }

        self.chunks_processed += 1;
        if self.chunks_processed % 180 == 0 {
            self.state = self.ctx.create_state()
                .map_err(|e| BackendError::Internal(format!("Failed to reset state: {}", e)))?;
            self.chunks_processed = 0;
        }

        if !all_text.is_empty() {
            log::debug!("[WHISPER RESULT] {}", all_text);
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
