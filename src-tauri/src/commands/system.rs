use reader_core::CommandError;
use tauri::Emitter;

#[tauri::command]
pub fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "frontend", "{message}"),
        "warning" => tracing::warn!(target: "frontend", "{message}"),
        "success" | "info" => tracing::info!(target: "frontend", "{message}"),
        _ => tracing::debug!(target: "frontend", level = %level, "{message}"),
    }
}

#[tauri::command]
pub fn get_platform() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "android")]
    {
        "android"
    }
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        "unknown"
    }
}

#[tauri::command]
pub async fn open_dir_in_explorer(path: String) -> Result<(), CommandError> {
    tauri_plugin_opener::open_path(path, None::<&str>).map_err(|err| CommandError {
        code: "IO_ERROR".to_string(),
        message: err.to_string(),
        detail: Some(format!("{err:?}")),
        retryable: false,
    })
}

#[tauri::command]
pub async fn script_dialog_result(
    app: tauri::AppHandle,
    id: String,
    value: serde_json::Value,
) -> Result<(), CommandError> {
    let _ = app.emit(
        "script:dialog:result",
        serde_json::json!({
            "id": id,
            "value": value
        }),
    );
    Ok(())
}
