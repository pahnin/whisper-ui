use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Uuid,
    pub title: String,
    pub content: String,
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

    pub fn to_filename(&self) -> String {
        let sanitized = Self::sanitize_title(&self.title);
        if sanitized.is_empty() {
            format!("{}.md", self.id)
        } else {
            format!("{}-{}.md", self.id, sanitized)
        }
    }
}
