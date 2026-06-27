#[derive(Debug, Clone)]
pub enum TranscriptionResult {
    Segment(String),
    Error(String),
}
