pub mod app;
pub mod document;
pub mod workspace;
pub mod inference;
pub mod audio;
pub mod ui;

fn main() {
    let base_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
        .unwrap_or_else(|_| std::path::PathBuf::from("whisper-app-data"));

    let workspace = crate::workspace::Workspace::load(&base_dir);

    let mut app_state = crate::app::AppState::default();
    app_state.workspace = workspace;

    let (model_idx, language, last_active) = app::load_app_state();
    app_state.selected_model_idx = model_idx;
    app_state.language = language;
    app_state.last_active_doc = last_active;

    app_state.load_models();

    let has_downloaded = app_state.models.iter().any(|m| m.downloaded);
    app_state.show_landing = !has_downloaded;

    let _result = iced::run::<crate::app::AppState, _, _, _>(
        crate::app::update,
        crate::app::view,
    );

    let _ = crate::app::save_app_state(&app_state);
}
