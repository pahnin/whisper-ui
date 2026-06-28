use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptLine {
    pub timestamp: String,
    pub text: String,
}

impl TranscriptLine {
    pub fn parse_content(content: &str) -> Vec<Self> {
        content
            .lines()
            .filter_map(|line| {
                let line = line.trim_end();
                if line.is_empty() {
                    return None;
                }
                if let Some(rest) = line.strip_prefix('[') {
                    if let Some(close_idx) = rest.find(']') {
                        let ts = &rest[..close_idx];
                        if ts.matches(':').count() == 1 || ts.matches(':').count() == 2 {
                            let text = rest[close_idx + 1..].trim().to_string();
                            return Some(TranscriptLine {
                                timestamp: format!("[{}]", ts),
                                text,
                            });
                        }
                    }
                }
                None
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub transcript_lines: Vec<TranscriptLine>,
    pub created_at: i64,
    pub modified_at: i64,
}

impl Document {
    pub fn new(title: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            title,
            content: String::new(),
            transcript_lines: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }

    pub fn sanitize_title(title: &str) -> String {
        title
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
            .take(50)
            .collect::<String>()
            .trim_matches('-')
            .to_string()
            .replace("--", "-")
    }

    pub fn parse_lines_from_content(&mut self) {
        self.transcript_lines = TranscriptLine::parse_content(&self.content);
    }

    pub fn to_filename(&self) -> String {
        let sanitized = Self::sanitize_title(&self.title);
        if sanitized.is_empty() {
            format!("{}.md", self.id)
        } else {
            format!("{}-{}.md", self.id, sanitized)
        }
    }
}
