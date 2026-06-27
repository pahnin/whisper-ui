pub mod app;
pub mod document;
pub mod workspace;
pub mod inference;
pub mod audio;
pub mod ui;

fn main() {
    let _base_dir = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".local/share/whisper-app"))
        .unwrap_or_else(|_| std::path::PathBuf::from("whisper-app-data"));

    let _result = iced::run::<crate::app::AppState, _, _, _>(
        crate::app::update,
        crate::app::view,
    );
}
