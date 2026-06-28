use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use futures_lite::StreamExt;
use reqwest::Client;
use whisper_rs;

#[derive(Clone)]
pub struct ProgressSender {
    tx: Sender<f32>,
}

impl ProgressSender {
    pub fn new(tx: Sender<f32>) -> Self {
        Self { tx }
    }

    pub fn send(&self, value: f32) {
        let _ = self.tx.send(value);
    }
}

const MODEL_NAMES: &[&str] = &[
    "ggml-tiny.bin",
    "ggml-base.bin",
    "ggml-small.bin",
    "ggml-medium.bin",
    "ggml-large-v3.bin",
];

const MODEL_DESCRIPTIONS: &[&str] = &[
    "Tiny",
    "Base",
    "Small",
    "Medium",
    "Large v3",
];

const MODEL_SIZES_BYTES: &[usize] = &[
    75_000_000,
    142_000_000,
    466_000_000,
    1_500_000_000,
    2_900_000_000,
];

pub type ProgressCallback = Box<dyn Fn(f32) + Send + 'static>;

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: usize,
    pub downloaded: bool,
}

pub struct ModelManager {
    cache_dir: PathBuf,
    models: Vec<ModelInfo>,
}

const DOWNLOAD_BUFFER_SIZE: usize = 8 * 1024 * 1024;

impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> Self {
        let models = MODEL_NAMES
            .iter()
            .enumerate()
            .map(|(i, filename)| {
                let path = cache_dir.join(filename);
                let downloaded = path.exists();
                ModelInfo {
                    name: MODEL_DESCRIPTIONS[i].to_string(),
                    filename: filename.to_string(),
                    path,
                    size_bytes: MODEL_SIZES_BYTES[i],
                    downloaded,
                }
            })
            .collect();

        Self { cache_dir, models }
    }

    pub fn available_models(&self) -> &[ModelInfo] {
        &self.models
    }

    pub fn downloaded_models(&self) -> Vec<&ModelInfo> {
        self.models.iter().filter(|m| m.downloaded).collect()
    }

    pub fn get_model_by_index(&self, idx: usize) -> Option<&ModelInfo> {
        self.models.get(idx)
    }

    pub async fn download(
        &self,
        idx: usize,
        progress: &ProgressSender,
    ) -> Result<PathBuf, String> {
        let model = self
            .get_model_by_index(idx)
            .ok_or_else(|| format!("Unknown model index: {}", idx))?;

        let filename = &model.filename;
        let url = format!(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}?download=true",
            filename
        );

        fs::create_dir_all(&self.cache_dir).map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let client = Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        let total_size = response.content_length().unwrap_or(model.size_bytes as u64);
        let temp_path = self.cache_dir.join(format!(".{}.tmp", model.filename));
        let file = File::create(&temp_path).map_err(|e| format!("Failed to create temp file: {}", e))?;
        let mut writer = BufWriter::with_capacity(DOWNLOAD_BUFFER_SIZE, file);
        let mut downloaded: u64 = 0;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| format!("Stream error: {}", e))?;
            downloaded += bytes.len() as u64;
            writer.write_all(&bytes).map_err(|e| format!("Failed to write chunk: {}", e))?;

            let progress_pct = if total_size > 0 {
                (downloaded as f32 / total_size as f32) * 100.0
            } else {
                (downloaded as f32 / model.size_bytes as f32) * 100.0
            };

            progress.send(progress_pct);
        }

        writer.flush().map_err(|e| format!("Failed to flush writer: {}", e))?;
        drop(writer);
        let output_path = self.cache_dir.join(&model.filename);
        fs::rename(&temp_path, &output_path).map_err(|e| format!("Failed to finalize model: {}", e))?;

        Ok(output_path)
    }

    pub fn clone_progress_sender(&self) -> ProgressSender {
        let (tx, _rx) = std::sync::mpsc::channel();
        ProgressSender::new(tx)
    }

    pub fn language_options() -> Vec<(String, String)> {
        let mut langs = Vec::new();
        for i in 0..=whisper_rs::get_lang_max_id() {
            if let Some(code) = whisper_rs::get_lang_str(i) {
                if let Some(full) = whisper_rs::get_lang_str_full(i) {
                    langs.push((code.to_string(), full.to_string()));
                }
            }
        }
        langs
    }

    pub fn validate(&self, idx: usize) -> Result<PathBuf, String> {
        let model = self
            .get_model_by_index(idx)
            .ok_or_else(|| format!("Unknown model index: {}", idx))?;

        if !model.path.exists() {
            return Err(format!("Model not found: {}", model.path.display()));
        }

        let metadata = fs::metadata(&model.path).map_err(|e| {
            format!("Failed to read model metadata: {}", e)
        })?;

        let actual_size = metadata.len() as usize;
        let expected_size = model.size_bytes;

        let tolerance = 0.05;
        let diff = if actual_size > expected_size {
            actual_size - expected_size
        } else {
            expected_size - actual_size
        };

        if diff as f64 > expected_size as f64 * tolerance {
            return Err(format!(
                "Model size mismatch: expected ~{}, got {}",
                expected_size, actual_size
            ));
        }

        Ok(model.path.clone())
    }
}
