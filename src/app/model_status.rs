#[derive(Debug, Clone, Default)]
pub enum ModelStatus {
    #[default]
    NotDownloaded,
    Downloading(f32),
    Ready,
    Error(String),
}
