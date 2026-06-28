pub mod app;
pub mod audio;
pub mod config;
pub mod document;
pub mod inference;
pub mod ui;
pub mod workspace;

fn subscription(state: &crate::app::AppState) -> iced::Subscription<crate::app::Message> {
    let poll_recording = if state.is_recording {
        iced::time::every(std::time::Duration::from_millis(1000))
            .map(|_| crate::app::Message::PollTrigger)
    } else {
        iced::Subscription::none()
    };
    let poll_download = if state.downloading_model.is_some() {
        iced::time::every(std::time::Duration::from_millis(1000))
            .map(|_| crate::app::Message::DownloadPoll)
    } else {
        iced::Subscription::none()
    };
    iced::Subscription::batch([poll_recording, poll_download])
}

fn main() {
    let language_options = crate::inference::backend::model_manager::ModelManager::language_options();

    let boot = move || {
        let base_dir = crate::config::config_dir();

        let mut workspace = crate::workspace::Workspace::load(&base_dir);

        for doc in workspace.documents.values_mut() {
            doc.parse_lines_from_content();
        }

        let mut app_state = crate::app::AppState::default();
        app_state.workspace = workspace;

        let (model_idx, language, last_active) = crate::app::load_app_state();
        app_state.selected_model_idx = model_idx;
        app_state.language = language;
        app_state.last_active_doc = last_active;

        app_state.load_models();
        app_state.language_options = language_options.clone();

        if app_state.selected_model_idx < app_state.models.len() {
            if app_state.models[app_state.selected_model_idx].downloaded {
                app_state.model_status = app::ModelStatus::Ready;
                let _ = app_state.init_backend();
            }
        }

        let has_downloaded = app_state.models.iter().any(|m| m.downloaded);
        app_state.show_landing = !has_downloaded;

        app_state
    };

    let _ = iced::application(boot, crate::app::update, crate::app::view)
        .subscription(subscription)
        .run();
}
