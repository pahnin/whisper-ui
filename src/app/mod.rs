use iced::widget::{Column, Container, Row};
use iced::{Element, Length, Task};

use crate::audio::capture::AudioCapture;
use crate::inference::backend::whisper_backend::WhisperBackend;
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
    ContentChangedTemp(String),
    CommitContent,
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
    InitBackend,
    PollResults,
}

pub struct AppState {
    pub workspace: Workspace,
    pub active_id: Option<uuid::Uuid>,
    pub audio_capture: Option<AudioCapture>,
    pub backend: Option<WhisperBackend>,
    pub temp_content: String,
    pub is_recording: bool,
    pub is_paused: bool,
    pub audio_level: f32,
    pub show_settings: bool,
    pub selected_model_idx: usize,
    pub models: Vec<String>,
    pub model_status: ModelStatus,
    pub language: String,
    pub result_rx: Option<std::sync::mpsc::Receiver<TranscriptionResult>>,
    pub level_rx: Option<std::sync::mpsc::Receiver<f32>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: Workspace::default(),
            active_id: None,
            audio_capture: None,
            backend: None,
            temp_content: String::new(),
            is_recording: false,
            is_paused: false,
            audio_level: 0.0,
            show_settings: false,
            selected_model_idx: 0,
            models: Vec::new(),
            model_status: ModelStatus::NotDownloaded,
            language: "en".to_string(),
            result_rx: None,
            level_rx: None,
        }
    }
}

impl AppState {
    pub fn load_models(&mut self) {
        let cache_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
            .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models"));
        let manager = crate::inference::backend::model_manager::ModelManager::new(cache_dir);
        for model in manager.available_models() {
            self.models.push(format!(
                "{} ({:.1}MB) {}",
                model.name,
                model.size_bytes as f64 / 1_000_000.0,
                if model.downloaded { "[✓]" } else { "" }
            ));
        }
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
        let backend = WhisperBackend::new(model.path.clone(), params)
            .map_err(|e| format!("Failed to load model: {}", e))?;
        self.backend = Some(backend);
        Ok(())
    }

    pub fn init_audio(&mut self) {
        let (level_tx, level_rx) = std::sync::mpsc::channel();
        match AudioCapture::new(level_tx) {
            Ok(audio) => {
                self.audio_capture = Some(audio);
                self.level_rx = Some(level_rx);
            }
            Err(e) => {
                eprintln!("Failed to initialize audio: {}", e);
            }
        }
    }

    pub fn is_backend_ready(&self) -> bool {
        self.backend.is_some()
    }

    pub fn handle_transcription_result(&mut self, text: String) {
        if !text.is_empty() {
            let now = chrono::Utc::now().format("%M:%S");
            self.temp_content.push_str(&format!("[{}] {}\n", now, text.trim()));
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
                        self.handle_transcription_result(format!("[Error] {}", err));
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
    }
}

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    match message {
        Message::NewDocument => {
            let id = state.workspace.new_document();
            state.active_id = Some(id);
            state.temp_content.clear();
        }
        Message::SelectDocument(id) => {
            state.workspace.activate(id);
            if let Some(doc) = state.workspace.active() {
                state.temp_content = doc.content.clone();
            }
        }
        Message::DeleteDocument(id) => {
            state.workspace.delete_document(id);
            if let Some(doc) = state.workspace.active() {
                state.temp_content = doc.content.clone();
            } else {
                state.temp_content.clear();
            }
        }
        Message::ContentChangedTemp(text) => {
            state.temp_content = text;
        }
        Message::CommitContent => {
            if let Some(doc) = state.workspace.active_mut() {
                doc.content = state.temp_content.clone();
            }
            if let Some(id) = state.active_id {
                let _ = state.workspace.save(id);
            }
        }

        Message::StartRecord => {
            state.is_recording = true;
            state.is_paused = false;
            if !state.is_backend_ready() {
                if let Err(e) = state.init_backend() {
                    state.temp_content.push_str(&format!("[Error] {}\n", e));
                    return Task::none();
                }
            }
            let Some(mut audio) = state.audio_capture.take() else {
                return Task::none();
            };
            let Some(backend) = state.backend.take() else {
                state.audio_capture = Some(audio);
                return Task::none();
            };

            let (result_tx, result_rx) = std::sync::mpsc::channel();
            state.result_rx = Some(result_rx);

            match audio.start() {
                Ok(()) => {
                    let ring_buffer = audio.get_ring_buffer();

                    std::thread::spawn(move || {
                        worker::run_worker(ring_buffer, backend, result_tx);
                    });

                    state.audio_capture = Some(audio);
                    // backend was moved into the thread, it will be recreated on next StartRecord if needed
                }
                Err(e) => {
                    state.audio_capture = Some(audio);
                    state.temp_content.push_str(&format!("[Error] {}\n", e));
                }
            }
        }

        Message::StopRecord => {
            state.is_recording = false;
            state.is_paused = false;
            if let Some(mut audio) = state.audio_capture.take() {
                audio.stop();
                state.audio_capture = Some(audio);
            }
            if state.backend.is_none() {
                if let Some(backend) = std::mem::take(&mut state.backend) {
                    state.backend = Some(backend);
                }
            }
        }

        Message::ResumeRecord => {
            state.is_paused = false;
        }

        Message::TranscriptionUpdate(text) => {
            state.handle_transcription_result(text);
        }

        Message::AudioLevelUpdate(level) => {
            state.audio_level = level;
        }

        Message::PollResults => {
            state.poll_results();
        }

        Message::ShowSettings => {
            state.show_settings = true;
        }
        Message::HideSettings => {
            state.show_settings = false;
        }
        Message::SaveSettings => {
            state.show_settings = false;
            if let Err(e) = state.init_backend() {
                state.model_status = ModelStatus::Error(e);
            } else {
                state.model_status = ModelStatus::Ready;
            }
        }

        Message::ModelDownloadProgress(pct) => {
            state.model_status = ModelStatus::Downloading(pct);
        }
        Message::ModelDownloadComplete => {
            state.model_status = ModelStatus::Ready;
            state.load_models();
        }
        Message::ModelDownloadError(err) => {
            state.model_status = ModelStatus::Error(err);
        }
        Message::ModelSelected(idx) => {
            state.selected_model_idx = idx;
            if let Some(model) = crate::inference::backend::model_manager::ModelManager::new(
                std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
                    .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models")),
            )
            .get_model_by_index(idx)
            {
                if model.downloaded {
                    state.model_status = ModelStatus::Ready;
                } else {
                    state.model_status = ModelStatus::NotDownloaded;
                }
            }
        }
        Message::LanguageChanged(lang) => {
            state.language = lang;
        }
        Message::DownloadModel(idx) => {
            let cache_dir = std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".cache/whisper-app/models"))
                .unwrap_or_else(|_| std::path::PathBuf::from(".cache/whisper-app/models"));
            let manager = crate::inference::backend::model_manager::ModelManager::new(cache_dir);
            let progress_tx = manager.clone_progress_sender();
            return Task::perform(
                async move {
                    match manager.download(idx, progress_tx).await {
                        Ok(_) => Message::ModelDownloadComplete,
                        Err(e) => Message::ModelDownloadError(e),
                    }
                },
                |msg| msg,
            );
        }
        Message::InitBackend => {
            if let Err(e) = state.init_backend() {
                return Task::perform(
                    async move { Message::ModelDownloadError(e) },
                    |msg| msg,
                );
            }
        }
    }
    Task::none()
}

pub fn view<'a>(
    state: &'a AppState,
) -> Element<'a, Message> {
    let sidebar = crate::ui::sidebar::view(&state.workspace, state.active_id);
    let active_doc = state.workspace.active();
    let editor = crate::ui::editor::view(active_doc, &state.temp_content);
    let controls = crate::ui::controls::view(
        state.is_recording,
        state.is_paused,
        state.audio_level,
        state.is_backend_ready(),
    );
    let settings = crate::ui::settings::view(
        state.show_settings,
        &state.models,
        state.selected_model_idx,
        &state.language,
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
        .height(Length::Fill);

    if let Some(settings_elem) = settings {
        Container::new(iced::widget::stack![main_content, settings_elem]).into()
    } else {
        main_content.into()
    }
}
