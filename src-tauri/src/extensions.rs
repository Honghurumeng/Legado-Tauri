use crate::errors::{BackendError, Result};
use crate::fs_utils::{safe_file_name, stable_hash};
use crate::storage::StorageState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionMeta {
    pub file_name: String,
    pub name: String,
    pub namespace: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub match_patterns: Vec<String>,
    pub grants: Vec<String>,
    pub run_at: String,
    pub category: String,
    pub enabled: bool,
    pub file_size: u64,
    pub modified_at: i64,
}

#[tauri::command]
pub async fn extension_get_dir() -> Result<String> {
    let dir = StorageState::extensions_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn extension_list() -> Result<Vec<ExtensionMeta>> {
    let dir = StorageState::extensions_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    let mut out = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("js") {
            continue;
        }
        let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let meta = tokio::fs::metadata(&path).await?;
        out.push(parse_meta(&path, &content, meta));
    }
    Ok(out)
}

#[tauri::command]
pub async fn extension_read(file_name: String) -> Result<String> {
    Ok(tokio::fs::read_to_string(extension_path(&file_name)?).await?)
}

#[tauri::command]
pub async fn extension_save(file_name: String, content: String) -> Result<()> {
    let path = extension_path(&file_name)?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, content).await?;
    Ok(())
}

#[tauri::command]
pub async fn extension_delete(file_name: String) -> Result<()> {
    let path = extension_path(&file_name)?;
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn extension_toggle(file_name: String, enabled: bool) -> Result<()> {
    let path = extension_path(&file_name)?;
    let content = tokio::fs::read_to_string(&path).await?;
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim_start().starts_with("// @enabled") {
            lines.push(format!(
                "// @enabled      {}",
                if enabled { "true" } else { "false" }
            ));
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        lines.insert(
            0,
            format!(
                "// @enabled      {}",
                if enabled { "true" } else { "false" }
            ),
        );
    }
    tokio::fs::write(path, lines.join("\n")).await?;
    Ok(())
}

#[tauri::command]
pub async fn extension_open_in_vscode(file_name: String) -> Result<()> {
    std::process::Command::new("code")
        .arg(extension_path(&file_name)?)
        .spawn()
        .map_err(|err| BackendError::msg(format!("无法调用 VS Code: {err}")))?;
    Ok(())
}

fn extension_path(file_name: &str) -> Result<PathBuf> {
    let safe = safe_file_name(file_name, "");
    if safe != file_name || !safe.ends_with(".js") || safe.contains("..") {
        return Err(BackendError::msg("扩展文件名必须是 .js，且不能包含路径"));
    }
    Ok(StorageState::extensions_dir()?.join(safe))
}

fn parse_meta(path: &std::path::Path, content: &str, meta: std::fs::Metadata) -> ExtensionMeta {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    ExtensionMeta {
        file_name: file_name.clone(),
        name: read_meta(content, "name").unwrap_or_else(|| file_name.clone()),
        namespace: read_meta(content, "namespace").unwrap_or_else(|| stable_hash(&file_name)),
        version: read_meta(content, "version").unwrap_or_default(),
        description: read_meta(content, "description").unwrap_or_default(),
        author: read_meta(content, "author").unwrap_or_default(),
        match_patterns: read_meta_multi(content, "match"),
        grants: read_meta_multi(content, "grant"),
        run_at: read_meta(content, "run-at").unwrap_or_else(|| "document-idle".into()),
        category: read_meta(content, "category").unwrap_or_default(),
        enabled: read_meta(content, "enabled")
            .map(|value| value != "false")
            .unwrap_or(true),
        file_size: meta.len(),
        modified_at: meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default(),
    }
}

fn read_meta(content: &str, key: &str) -> Option<String> {
    let prefix = format!("// @{key}");
    content.lines().take(120).find_map(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn read_meta_multi(content: &str, key: &str) -> Vec<String> {
    let prefix = format!("// @{key}");
    content
        .lines()
        .take(120)
        .filter_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .collect()
}
