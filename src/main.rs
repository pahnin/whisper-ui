pub mod app;
pub mod document;
pub mod workspace;
pub mod inference;
pub mod audio;
pub mod ui;

fn main() {
    let language_options = {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            crate::inference::backend::model_manager::ModelManager::language_options()
        })
    };

    let boot = move || {
        let base_dir = std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
            .unwrap_or_else(|_| std::path::PathBuf::from("whisper-app-data"));

        let workspace = crate::workspace::Workspace::load(&base_dir);

        let mut app_state = crate::app::AppState::default();
        app_state.workspace = workspace;

        let (model_idx, language, last_active) = crate::app::load_app_state();
        app_state.selected_model_idx = model_idx;
        app_state.language = language;
        app_state.last_active_doc = last_active;

        app_state.load_models();
        app_state.language_options = language_options.clone();

        let has_downloaded = app_state.models.iter().any(|m| m.downloaded);
        app_state.show_landing = !has_downloaded;

        app_state
    };

    let _ = iced::application(boot, crate::app::update, crate::app::view).run();
}
