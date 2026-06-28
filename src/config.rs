use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = home_dir() {
            home.join("Library/Application Support/whisper-app")
        } else {
            PathBuf::from("whisper-app-data")
        }
    } else if cfg!(target_os = "windows") {
        if let Some(path) = std::env::var_os("APPDATA") {
            PathBuf::from(&path).join("whisper-app")
        } else {
            home_dir()
                .map(|h| h.join("whisper-app"))
                .unwrap_or_else(|| PathBuf::from("whisper-app-data"))
        }
    } else {
        if let Some(home) = home_dir() {
            home.join(".local/share/whisper-app")
        } else {
            PathBuf::from("whisper-app-data")
        }
    }
}

pub fn cache_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        if let Some(home) = home_dir() {
            home.join("Library/Caches/whisper-app")
        } else {
            PathBuf::from(".cache/whisper-app")
        }
    } else if cfg!(target_os = "windows") {
        if let Some(path) = std::env::var_os("LOCALAPPDATA") {
            PathBuf::from(&path).join("whisper-app/Cache")
        } else {
            home_dir()
                .map(|h| h.join(".cache/whisper-app"))
                .unwrap_or_else(|| PathBuf::from(".cache/whisper-app"))
        }
    } else {
        if let Some(home) = home_dir() {
            home.join(".cache/whisper-app")
        } else {
            PathBuf::from(".cache/whisper-app")
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
