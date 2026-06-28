use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::document::{Document, TranscriptLine};

const DOCUMENTS_DIR: &str = "documents";
const SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

pub struct Workspace {
    pub documents: BTreeMap<Uuid, Document>,
    pub active_id: Option<Uuid>,
    pub base_dir: std::path::PathBuf,
}

impl Workspace {
    pub fn load(base_dir: &Path) -> Self {
        let documents_dir = base_dir.join(DOCUMENTS_DIR);
        let mut documents = BTreeMap::new();

        if documents_dir.exists() {
            if let Ok(entries) = fs::read_dir(&documents_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let filename = path.file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let id = filename.split('-').next()
                                .and_then(|id| Uuid::parse_str(id).ok());

                            if let Some(doc_id) = id {
                                let title = if filename.contains('-') {
                                    filename.splitn(2, '-').nth(1)
                                        .unwrap_or("Untitled")
                                        .replace('-', " ")
                                } else {
                                    "Untitled".to_string()
                                };

                                let transcript_lines = TranscriptLine::parse_content(&content);
                                documents.insert(doc_id, Document {
                                    id: doc_id,
                                    title: title.trim().to_string(),
                                    content,
                                    transcript_lines,
                                    created_at: chrono::Utc::now().timestamp(),
                                    modified_at: chrono::Utc::now().timestamp(),
                                    last_save_at: std::time::Instant::now(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let active_id = documents.keys().next().copied();

        Self {
            documents,
            active_id,
            base_dir: base_dir.to_path_buf(),
        }
    }

    pub fn new_document(&mut self) -> Uuid {
        let title = format!("Document {}", self.documents.len() + 1);
        let doc = Document::new(title);
        let id = doc.id;
        self.documents.insert(id, doc);
        self.active_id = Some(id);
        id
    }

    pub fn delete_document(&mut self, id: Uuid) -> bool {
        if self.documents.remove(&id).is_some() {
            if self.active_id == Some(id) {
                self.active_id = self.documents.keys().next().copied();
            }
            true
        } else {
            false
        }
    }

    pub fn activate(&mut self, id: Uuid) {
        if self.documents.contains_key(&id) {
            self.active_id = Some(id);
        }
    }

    pub fn active_mut(&mut self) -> Option<&mut Document> {
        self.active_id.and_then(|id| self.documents.get_mut(&id))
    }

    pub fn active(&self) -> Option<&Document> {
        self.active_id.and_then(|id| self.documents.get(&id))
    }

    pub fn save(&self, id: Uuid) -> Result<(), std::io::Error> {
        if let Some(doc) = self.documents.get(&id) {
            let documents_dir = self.base_dir.join(DOCUMENTS_DIR);
            fs::create_dir_all(&documents_dir)?;
            let filename = doc.to_filename();
            let path = documents_dir.join(filename);
            fs::write(path, &doc.content)?;
        }
        Ok(())
    }

    pub fn save_if_needed(&mut self, id: Uuid) -> Result<(), std::io::Error> {
        let needs_save = self.documents.get(&id)
            .map(|doc| doc.last_save_at.elapsed() >= SAVE_INTERVAL)
            .unwrap_or(false);
        if needs_save {
            self.save(id)?;
            if let Some(doc) = self.documents.get_mut(&id) {
                doc.last_save_at = std::time::Instant::now();
            }
        }
        Ok(())
    }

    pub fn save_all_forced(&self) -> Result<(), std::io::Error> {
        self.save_all()
    }

    pub fn save_all(&self) -> Result<(), std::io::Error> {
        for id in self.documents.keys() {
            self.save(*id)?;
        }
        Ok(())
    }

    pub fn rename_document(&mut self, id: Uuid, new_title: String) -> bool {
        if let Some(doc) = self.documents.get_mut(&id) {
            doc.title = new_title;
            doc.modified_at = chrono::Utc::now().timestamp();
            true
        } else {
            false
        }
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            documents: BTreeMap::new(),
            active_id: None,
            base_dir: std::path::PathBuf::new(),
        }
    }
}
