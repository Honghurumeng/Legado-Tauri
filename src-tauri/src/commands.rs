use crate::errors::{BackendError, Result};
use crate::fs_utils::cache_dir;
use crate::http::{self, HttpProxyResponse};
use crate::storage::StorageState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::UdpSocket;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebServerStatus {
    pub running: bool,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SysFont {
    pub family: String,
    pub postscript_name: String,
    pub path: String,
}

#[tauri::command]
pub async fn get_platform() -> Result<String> {
    Ok(std::env::consts::OS.to_string())
}

#[tauri::command]
pub async fn get_local_ips() -> Result<Vec<String>> {
    let mut ips = Vec::new();
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                ips.push(addr.ip().to_string());
            }
        }
    }
    ips.sort();
    ips.dedup();
    Ok(ips)
}

#[tauri::command]
pub async fn open_dir_in_explorer(path: String) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("explorer");
        command.arg(path);
        command
    };
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(path);
        command
    };
    command
        .spawn()
        .map_err(|err| BackendError::msg(format!("无法打开目录: {err}")))?;
    Ok(())
}

#[tauri::command]
pub async fn list_system_fonts() -> Result<Vec<SysFont>> {
    Ok(Vec::new())
}

#[tauri::command]
pub async fn cover_cache_size() -> Result<u64> {
    dir_size(StorageState::cover_cache_dir()?).await
}

#[tauri::command]
pub async fn cover_cache_clear() -> Result<u64> {
    let dir = StorageState::cover_cache_dir()?;
    let size = dir_size(dir.clone()).await?;
    if dir.exists() {
        tokio::fs::remove_dir_all(&dir).await?;
    }
    tokio::fs::create_dir_all(dir).await?;
    Ok(size)
}

#[tauri::command]
pub async fn booksource_http_proxy(
    url: String,
    method: Option<String>,
    body: Option<String>,
    headers: Option<Value>,
    referer: Option<String>,
) -> Result<HttpProxyResponse> {
    http::request_text(
        method.as_deref().unwrap_or("GET"),
        &url,
        body,
        headers,
        referer.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn web_server_status() -> Result<WebServerStatus> {
    Ok(WebServerStatus {
        running: false,
        port: 7688,
    })
}

#[tauri::command]
pub async fn web_server_start() -> Result<u16> {
    Err(BackendError::msg(
        "Web/B-S 服务器尚未接入；Tauri IPC 已可用",
    ))
}

#[tauri::command]
pub async fn web_server_stop() -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn web_server_pick_dist_dir() -> Result<String> {
    Ok(String::new())
}

#[tauri::command]
pub async fn frontend_log(level: Option<String>, message: String) -> Result<()> {
    eprintln!(
        "[frontend:{}] {}",
        level.unwrap_or_else(|| "info".into()),
        message
    );
    Ok(())
}

#[tauri::command]
pub async fn not_implemented_command(cmd: Option<String>) -> Result<Value> {
    Err(BackendError::msg(format!(
        "未实现命令{}",
        cmd.map(|value| format!(": {value}")).unwrap_or_default()
    )))
}

async fn dir_size(path: std::path::PathBuf) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut total = 0u64;
    let mut stack = vec![path];
    while let Some(dir) = stack.pop() {
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[allow(dead_code)]
pub fn temp_cache_dir() -> Result<std::path::PathBuf> {
    cache_dir()
}
