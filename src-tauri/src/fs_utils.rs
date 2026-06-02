use crate::errors::{BackendError, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn app_data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .ok_or_else(|| BackendError::msg("无法定位应用数据目录"))?;
    Ok(base.join("legado-tauri"))
}

pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    Ok(base.join("legado-tauri"))
}

pub fn safe_file_name(input: &str, default_ext: &str) -> String {
    let trimmed = input.trim();
    let mut name = trimmed
        .chars()
        .map(|ch| match ch {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect::<String>();
    name = name.trim_matches('.').trim().to_string();
    if name.is_empty() {
        name = "source".to_string();
    }
    if !name.contains('.') && !default_ext.is_empty() {
        name.push('.');
        name.push_str(default_ext.trim_start_matches('.'));
    }
    name
}

pub fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub async fn write_json_pretty(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let text = serde_json::to_string_pretty(value)?;
    tokio::fs::write(path, text).await?;
    Ok(())
}
