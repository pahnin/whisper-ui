use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use iced::widget::{button, Column, Container, Row, Text};
use iced::{Element, Length, Task};

use crate::audio::capture::AudioCapture;

static TOKIO_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_tokio_runtime() -> &'static tokio::runtime::Runtime {
    TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}
use crate::inference::backend::whisper_backend::WhisperBackend;
use crate::inference::backend::model_manager::ModelInfo;
use crate::workspace::Workspace;

pub mod model_status;
pub mod transcription_result;
pub mod worker;

pub use model_status::ModelStatus;
pub use transcription_result::TranscriptionResult;

#[derive(Debug, Clone)]
pub enum Message {
    NewDocument,
    SelectDocument(uuid::Uuid),
    DeleteDocument(uuid::Uuid),
    StartRecord,
    StopRecord,
    ResumeRecord,
    ShowSettings,
    HideSettings,
    SaveSettings,
    TranscriptionUpdate(String),
    AudioLevelUpdate(f32),
    ModelDownloadProgress(f32),
    ModelDownloadComplete,
    ModelDownloadError(String),
    ModelSelected(usize),
    LanguageChanged(String),
    DownloadModel(usize),
    LoadModel(usize),
    InitBackend,
    BackendInitError(String),
    PollResults,
    PollTrigger,
    RenameDocument(uuid::Uuid),
    RenameDocumentConfirm(String),
    ClearError,
    HideLanding,
}

pub struct AppState {
    pub workspace: Workspace,
    pub active_id: Option<uuid::Uuid>,
    pub last_active_doc: Option<uuid::Uuid>,
    pub audio_capture: Option<AudioCapture>,
    pub backend: Option<WhisperBackend>,
    pub worker_handle: Option<std::thread::JoinHandle<()>>,
    pub is_recording: bool,
    pub is_paused: bool,
    pub audio_level: f32,
    pub show_settings: bool,
    pub selected_model_idx: usize,
    pub models: Vec<ModelInfo>,
    pub model_status: ModelStatus,
    pub model_loaded: bool,
    pub language: String,
    pub language_options: Vec<(String, String)>,
    pub result_rx: Option<std::sync::mpsc::Receiver<TranscriptionResult>>,
    pub level_rx: Option<std::sync::mpsc::Receiver<f32>>,
    pub progress_rx: Option<std::sync::mpsc::Receiver<f32>>,
    pub poll_tx: Option<std::sync::mpsc::Sender<()>>,
    pub poll_rx: Option<std::sync::mpsc::Receiver<()>>,
    pub download_done: std::sync::Arc<AtomicBool>,
    pub downloading_model: Option<usize>,
    pub error_message: Option<String>,
    pub rename_doc: Option<uuid::Uuid>,
    pub rename_input: String,
    pub show_landing: bool,
    pub accelerator: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            active_id: None,
            last_active_doc: None,
            audio_capture: None,
            backend: None,
            worker_handle: None,
            is_recording: false,
            is_paused: false,
            audio_level: 0.0,
            show_settings: false,
            selected_model_idx: 1,
            models: Vec::new(),
            model_status: ModelStatus::NotDownloaded,
            model_loaded: false,
            language: "en".to_string(),
            result_rx: None,
            level_rx: None,
            progress_rx: None,
           poll_tx: None,
            poll_rx: None,
            download_done: std::sync::Arc::new(AtomicBool::new(false)),
            downloading_model: None,
            error_message: None,
            rename_doc: None,
            rename_input: String::new(),
            show_landing: false,
            language_options: Vec::new(),
            accelerator: None,
        }
    }
}

impl AppState {
    pub fn load_models(&mut self) {
        let cache_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models"));
        let manager = crate::inference::backend::model_manager::ModelManager::new(cache_dir);
        self.models = manager.available_models().to_vec();
    }

    pub fn init_backend(&mut self) -> Result<(), String> {
        let cache_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models"));
        let manager = crate::inference::backend::model_manager::ModelManager::new(cache_dir);
        let model = manager
            .get_model_by_index(self.selected_model_idx)
            .ok_or_else(|| "No model selected".to_string())?;
        if !model.downloaded {
            return Err(format!(
                "Model '{}' not downloaded. Download it in Settings.",
                model.name
            ));
        }
        let params = crate::inference::backend::BackendParams {
            language: Some(self.language.clone()),
            beam_size: 1,
            vad_enabled: false,
        };
        match WhisperBackend::new(model.path.clone(), params) {
            Ok(backend) => {
                self.accelerator = Some(backend.accelerator.clone());
                self.backend = Some(backend);
                self.model_loaded = true;
                Ok(())
            }
            Err(e) => {
                eprintln!("[APP] Backend init failed, falling back to CPU: {}", e);
                let cpu_params = crate::inference::backend::BackendParams {
                    language: Some(self.language.clone()),
                    beam_size: 1,
                    vad_enabled: false,
                };
                let backend = WhisperBackend::new_cpu(model.path.clone(), cpu_params)
                    .map_err(|e| format!("Failed to load model (CPU fallback): {}", e))?;
                self.accelerator = Some("CPU".to_string());
                self.backend = Some(backend);
                self.model_loaded = true;
                Ok(())
            }
        }
    }

    pub fn init_audio(&mut self) -> Result<(), String> {
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        let audio = AudioCapture::new(level_tx)
            .map_err(|e| format!("Failed to create audio capture: {}", e))?;
        self.audio_capture = Some(audio);
        self.level_rx = Some(level_rx);
        Ok(())
    }

    pub fn is_backend_ready(&self) -> bool {
        self.backend.is_some()
    }

    pub fn handle_transcription_result(&mut self, text: String) {
        if !text.is_empty() {
            let now = chrono::Utc::now().format("%M:%S");
            let formatted = format!("[{}] {}\n", now, text.trim());
            if let Some(doc) = self.workspace.active_mut() {
                doc.content.push_str(&formatted);
                doc.modified_at = chrono::Utc::now().timestamp();
                let id = doc.id;
                let _ = self.workspace.save(id);
            }
        }
    }

    pub fn poll_results(&mut self) {
        if let Some(rx) = self.result_rx.take() {
            while let Ok(result) = rx.try_recv() {
                match result {
                    TranscriptionResult::Segment(text) => {
                        self.handle_transcription_result(text);
                    }
                    TranscriptionResult::Error(err) => {
                        self.error_message = Some(err);
                    }
                }
            }
            self.result_rx = Some(rx);
        }
        if let Some(rx) = self.level_rx.take() {
            while let Ok(level) = rx.try_recv() {
                self.audio_level = level;
            }
            self.level_rx = Some(rx);
        }
        if self.download_done.load(Ordering::Relaxed) {
            return;
        }
        if let Some(rx) = self.progress_rx.take() {
            while let Ok(pct) = rx.try_recv() {
                if pct >= 0.0 {
                    // Don't overwrite error state
                    if !matches!(self.model_status, ModelStatus::Error(_)) {
                        self.model_status = ModelStatus::Downloading(pct);
                    }
                }
            }
            self.progress_rx = Some(rx);
        }
        if let Some(rx) = self.poll_rx.take() {
            while let Ok(()) = rx.try_recv() {
                // Just draining
            }
            self.poll_rx = Some(rx);
        }
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if let Some(ref mut audio) = self.audio_capture {
            audio.stop();
        }
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    state.poll_results();

    match message {
        Message::NewDocument => {
            let id = state.workspace.new_document();
            state.active_id = Some(id);
            let _ = save_app_state(state);
        }
        Message::SelectDocument(id) => {
            state.last_active_doc = state.active_id;
            if let Some(current_id) = state.active_id {
                let _ = state.workspace.save(current_id);
            }
            state.workspace.activate(id);
            state.active_id = Some(id);
            let _ = save_app_state(state);
        }
        Message::DeleteDocument(id) => {
            state.workspace.delete_document(id);
        }
        Message::StartRecord => {
            if state.is_recording {
                return Task::none();
            }
            if state.downloading_model.is_some() {
                state.error_message = Some("Cannot start recording while a model is downloading".to_string());
                return Task::none();
            }
            if state.workspace.documents.is_empty() {
                let id = state.workspace.new_document();
                state.active_id = Some(id);
            }
            state.is_recording = true;
            state.is_paused = false;
            if !state.is_backend_ready() {
                if let Err(e) = state.init_backend() {
                    eprintln!("[APP] Failed to load model: {}", e);
                    state.error_message = Some(e);
                    state.is_recording = false;
                    return Task::none();
                }
            }
            if state.audio_capture.is_none() {
                if let Err(e) = state.init_audio() {
                    eprintln!("[APP] Failed to init audio: {}", e);
                    state.error_message = Some(e);
                    state.is_recording = false;
                    return Task::none();
                }
            }
            let Some(mut audio) = state.audio_capture.take() else {
                state.error_message = Some("Audio capture not initialized".to_string());
                state.is_recording = false;
                return Task::none();
            };
            let Some(backend) = state.backend.take() else {
                state.audio_capture = Some(audio);
                state.error_message = Some("Backend not initialized".to_string());
                state.is_recording = false;
                return Task::none();
            };

            let (result_tx, result_rx) = std::sync::mpsc::channel();
            state.result_rx = Some(result_rx);

            match audio.start() {
                Ok(()) => {
                    let ring_buffer = audio.get_ring_buffer();
                    let running = audio.get_running();
                    let handle = worker::run_worker(ring_buffer, backend, result_tx, running);
                    state.worker_handle = Some(handle);
                    state.audio_capture = Some(audio);
                }
                Err(e) => {
                    state.audio_capture = Some(audio);
                    state.error_message = Some(format!("Failed to start recording: {}", e));
                    state.is_recording = false;
                }
            }
        }

        Message::StopRecord => {
            state.is_recording = false;
            state.is_paused = false;
            if let Some(id) = state.active_id {
                let _ = state.workspace.save(id);
            }
            if let Some(mut audio) = state.audio_capture.take() {
                audio.stop();
                state.audio_capture = Some(audio);
            }
            state.worker_handle = None;
            state.poll_results();
            if state.selected_model_idx < state.models.len()
                && state.models[state.selected_model_idx].downloaded {
                let _ = state.init_backend();
            }
        }

        Message::ResumeRecord => {
            state.is_paused = false;
        }

        Message::TranscriptionUpdate(text) => {
            if text.starts_with("[Error]") {
                state.error_message = Some(text.trim_start_matches("[Error]").trim().to_string());
            } else {
                state.handle_transcription_result(text);
            }
        }

        Message::AudioLevelUpdate(level) => {
            state.audio_level = level;
        }

        Message::PollResults => {
            state.poll_results();
        }
        Message::PollTrigger => {
            state.poll_results();
        }

        Message::ShowSettings => {
            state.show_settings = true;
        }
       Message::HideSettings => {
            state.downloading_model = None;
            state.progress_rx = None;
            state.poll_tx = None;
            state.poll_rx = None;
            state.download_done.store(false, Ordering::Relaxed);
            state.show_settings = false;
        }
       Message::SaveSettings => {
            state.downloading_model = None;
            state.progress_rx = None;
            state.poll_tx = None;
            state.poll_rx = None;
            state.download_done.store(false, Ordering::Relaxed);
            if state.backend.is_none() {
                if let Err(e) = state.init_backend() {
                    state.model_status = ModelStatus::Error(e);
                } else {
                    state.model_status = ModelStatus::Ready;
                }
            }
            state.show_settings = false;
            let _ = save_app_state(state);
        }

        Message::ModelDownloadProgress(pct) => {
            state.model_status = ModelStatus::Downloading(pct);
        }
       Message::ModelDownloadComplete => {
            state.model_status = ModelStatus::Ready;
            state.downloading_model = None;
            state.progress_rx = None;
            state.poll_tx = None;
            state.poll_rx = None;
            state.download_done.store(true, Ordering::Relaxed);
            state.load_models();
        }
     Message::ModelDownloadError(err) => {
             state.downloading_model = None;
             state.progress_rx = None;
             state.poll_tx = None;
             state.poll_rx = None;
             state.download_done.store(true, Ordering::Relaxed);
             state.model_status = ModelStatus::Error(err);
         }
        Message::BackendInitError(err) => {
             state.error_message = Some(err);
         }
  Message::ModelSelected(idx) => {
             state.selected_model_idx = idx;
             if idx < state.models.len() {
                 if state.models[idx].downloaded {
                     state.model_status = ModelStatus::Ready;
                     if state.is_backend_ready() {
                         state.model_loaded = true;
                     }
                 } else {
                     state.model_status = ModelStatus::NotDownloaded;
                 }
             }
             let _ = save_app_state(state);
         }
        Message::LanguageChanged(lang) => {
            state.language = lang;
            let _ = save_app_state(state);
        }
    Message::DownloadModel(idx) => {
            if state.downloading_model.is_some() {
                return Task::none();
            }
            state.model_status = ModelStatus::Downloading(0.0);
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (poll_tx, poll_rx) = std::sync::mpsc::channel();
            let ticker_poll_tx = poll_tx.clone();
            let download_done = std::sync::Arc::new(AtomicBool::new(false));
            state.downloading_model = Some(idx);
            state.progress_rx = Some(progress_rx);
            state.poll_tx = Some(poll_tx.clone());
            state.poll_rx = Some(poll_rx);
            state.download_done = download_done.clone();
            let progress = crate::inference::backend::model_manager::ProgressSender::new(progress_tx);
            let cache_dir = std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
                .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models"));
            let manager = crate::inference::backend::model_manager::ModelManager::new(cache_dir);
            let done_clone = download_done.clone();
            // Background download thread
            std::thread::spawn(move || {
                let rt = get_tokio_runtime();
                let result = rt.block_on(manager.download(idx, &progress));
                if result.is_err() {
                    let _ = progress.send(0.0);
                    let _ = poll_tx.send(());
                    let _ = poll_tx.send(());
                }
                done_clone.store(true, Ordering::Relaxed);
                drop(progress);
            });
            // Ticker thread that periodically triggers poll_results
            std::thread::spawn(move || {
                use std::time::Duration;
                loop {
                    std::thread::sleep(Duration::from_millis(100));
                    if download_done.load(Ordering::Relaxed) {
                        break;
                    }
                    if ticker_poll_tx.send(()).is_err() {
                        break;
                    }
                }
            });
            return Task::none();
        }
        Message::LoadModel(idx) => {
            state.selected_model_idx = idx;
            if let Err(e) = state.init_backend() {
                state.model_status = ModelStatus::Error(e);
            } else {
                state.model_status = ModelStatus::Ready;
            }
        }
        Message::InitBackend => {
            if let Err(e) = state.init_backend() {
                return Task::perform(
                    async move { Message::BackendInitError(e) },
                    |msg| msg,
                );
            }
        }

        Message::RenameDocument(id) => {
            state.rename_doc = Some(id);
            if let Some(doc) = state.workspace.documents.get(&id) {
                state.rename_input = doc.title.clone();
            }
        }
        Message::RenameDocumentConfirm(new_title) => {
            if let Some(id) = state.rename_doc.take() {
                let sanitized = crate::document::Document::sanitize_title(&new_title);
                if !sanitized.is_empty() {
                    state.workspace.rename_document(id, sanitized);
                }
                if let Some(doc) = state.workspace.documents.get(&id) {
                    state.rename_input = doc.title.clone();
                }
            }
        }
        Message::ClearError => {
            state.error_message = None;
        }
        Message::HideLanding => {
            state.show_landing = false;
        }
    }
    Task::none()
}

pub fn save_app_state(state: &AppState) -> Result<(), String> {
    let config_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
        .unwrap_or_else(|_| std::path::PathBuf::from(".local/share/whisper-app"));
    let config_path = config_dir.join("app_state.json");

    let last_active = state.last_active_doc.map(|id| id.to_string());

    #[derive(serde::Serialize)]
    struct AppConfig {
        selected_model_idx: usize,
        language: String,
        last_active_doc: Option<String>,
    }

    let config = AppConfig {
        selected_model_idx: state.selected_model_idx,
        language: state.language.clone(),
        last_active_doc: last_active,
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize app state: {}", e))?;
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;
    fs::write(config_path, json)
        .map_err(|e| format!("Failed to save app state: {}", e))?;

    Ok(())
}

pub fn load_app_state() -> (usize, String, Option<uuid::Uuid>) {
    let config_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
        .unwrap_or_else(|_| std::path::PathBuf::from(".local/share/whisper-app"));
    let config_path = config_dir.join("app_state.json");

    if !config_path.exists() {
        return (0, "en".to_string(), None);
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return (0, "en".to_string(), None),
    };

    #[derive(serde::Deserialize, Default)]
    struct AppConfig {
        #[serde(default)]
        selected_model_idx: usize,
        #[serde(default = "default_language")]
        language: String,
        #[serde(default)]
        last_active_doc: Option<String>,
    }

    fn default_language() -> String {
        "en".to_string()
    }

    let config: AppConfig = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return (0, "en".to_string(), None),
    };

    let last_active = config
        .last_active_doc
        .and_then(|s| uuid::Uuid::parse_str(&s).ok());

    (config.selected_model_idx, config.language, last_active)
}

pub fn view<'a>(
    state: &'a AppState,
) -> Element<'a, Message> {
    let sidebar = crate::ui::sidebar::view(
        &state.workspace,
        state.active_id,
        state.rename_doc,
        &state.rename_input,
    );
    let active_doc = state.workspace.active();
    let editor = crate::ui::editor::view(active_doc, "");
    let controls = crate::ui::controls::view(
        state.is_recording,
        state.is_paused,
        state.audio_level,
        state.model_loaded,
        state.accelerator.as_deref(),
    );

    let error_bar: Option<Element<'a, Message>> = if let Some(ref err) = state.error_message {
        let content: Element<Message> = Row::new()
            .push(Text::new(err).size(12))
            .push(iced::widget::space())
            .push(
                button(Text::new("✕"))
                    .on_press(Message::ClearError),
            )
            .spacing(8)
            .padding(8)
            .into();
        Some(Container::new(content)
            .width(Length::Fill)
            .height(40)
            .into())
    } else {
        None
    };

    let landing_overlay: Option<Element<'a, Message>> = if state.show_landing {
        let landing_content: Element<Message> = Column::new()
            .spacing(16)
            .align_x(iced::Alignment::Center)
            .push(Text::new("Whisper Voice Transcription").size(24))
            .push(Text::new("Download a model to get started").size(14))
            .push(
                button(Text::new("Open Settings"))
                    .on_press(Message::ShowSettings),
            )
            .padding(40)
            .into();
        Some(Container::new(landing_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into())
    } else {
        None
    };

    let settings = crate::ui::settings::view(
        state.show_settings,
        &state.models,
        state.selected_model_idx,
        &state.language,
        &state.model_status,
        state.is_backend_ready(),
        state.downloading_model,
        state.error_message.as_deref(),
        &state.language_options,
    );

    let main_content = Column::new()
        .push(
            Row::new()
                .push(sidebar)
                .push(editor)
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .push(controls)
        .height(Length::Fill)
        .width(Length::Fill);

    let with_landing = if let Some(overlay) = landing_overlay {
        Container::new(iced::widget::stack![main_content, overlay]).into()
    } else {
        main_content.into()
    };

    let with_error = if let Some(error_elem) = error_bar {
        Column::new()
            .push(error_elem)
            .push(with_landing)
            .into()
    } else {
        with_landing
    };

    if let Some(settings_elem) = settings {
        Column::new()
            .push(with_error)
            .push(settings_elem)
            .into()
    } else {
        with_error
    }
}
