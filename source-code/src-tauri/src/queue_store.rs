use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PersistedJob {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub payload: serde_json::Value,
}

fn queue_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    PathBuf::from(format!("{home}/.hackeros/store/queue.json"))
}

pub fn load() -> Vec<PersistedJob> {
    std::fs::read_to_string(queue_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(jobs: &[PersistedJob]) -> Result<(), String> {
    let path = queue_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(jobs).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
