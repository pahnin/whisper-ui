use iced::widget::{Column, Container, Row};
use iced::{Element, Length};

pub fn view<'a>(
    state: &'a crate::app::AppState,
) -> Element<'a, Message> {
    let sidebar = crate::ui::sidebar::view(&state.workspace, state.active_id);
    let active_doc = state.workspace.active();
    let editor = crate::ui::editor::view(active_doc, &state.temp_content);
    let controls = crate::ui::controls::view(state.is_recording, state.is_paused, state.audio_level);
    let settings = crate::ui::settings::view(state.show_settings, &state.models, state.selected_model_idx);

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
        Container::new(iced::widget::stack![main_content, settings_elem])
            .into()
    } else {
        main_content.into()
    }
}

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
}

pub fn update(state: &mut AppState, message: Message) {
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
        }
        Message::StartRecord => {
            state.is_recording = true;
            state.is_paused = false;
        }
        Message::StopRecord => {
            state.is_recording = false;
            state.is_paused = false;
        }
        Message::ResumeRecord => {
            state.is_paused = false;
        }
        Message::ShowSettings => {
            state.show_settings = true;
        }
        Message::HideSettings => {
            state.show_settings = false;
        }
        Message::SaveSettings => {
            state.show_settings = false;
        }
    }
}

pub struct AppState {
    pub workspace: crate::workspace::Workspace,
    pub active_id: Option<uuid::Uuid>,
    pub is_recording: bool,
    pub is_paused: bool,
    pub audio_level: f32,
    pub show_settings: bool,
    pub selected_model_idx: usize,
    pub temp_content: String,
    pub models: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace: crate::workspace::Workspace::default(),
            active_id: None,
            is_recording: false,
            is_paused: false,
            audio_level: 0.0,
            show_settings: false,
            selected_model_idx: 0,
            temp_content: String::new(),
            models: Vec::new(),
        }
    }
}
