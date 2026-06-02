use crate::errors::{BackendError, Result};
use crate::fs_utils::{app_data_dir, write_json_pretty};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::PathBuf;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendStorageEntry {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendStorageNamespaceSummary {
    pub namespace: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDebugDump {
    pub frontend: Value,
    pub script_json: Value,
    pub script_bytes: Value,
    pub client_states: Value,
    pub app_state_path: Option<String>,
    pub bookshelf_path: Option<String>,
}

pub struct StorageState {
    lock: Mutex<()>,
}

impl StorageState {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }

    pub fn base_dir() -> Result<PathBuf> {
        app_data_dir()
    }

    fn frontend_path() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("frontend-storage.json"))
    }

    fn script_json_path() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("script-config.json"))
    }

    fn script_bytes_path() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("script-bytes.json"))
    }

    pub fn bookshelf_path() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("bookshelf.json"))
    }

    pub fn chapters_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("chapters"))
    }

    pub fn contents_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("contents"))
    }

    pub fn extensions_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("extensions"))
    }

    pub fn user_fonts_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("fonts"))
    }

    pub fn cover_cache_dir() -> Result<PathBuf> {
        Ok(Self::base_dir()?.join("cover-cache"))
    }

    pub fn ensure_layout(&self) -> Result<()> {
        std::fs::create_dir_all(Self::base_dir()?)?;
        std::fs::create_dir_all(Self::chapters_dir()?)?;
        std::fs::create_dir_all(Self::contents_dir()?)?;
        std::fs::create_dir_all(Self::extensions_dir()?)?;
        std::fs::create_dir_all(Self::user_fonts_dir()?)?;
        std::fs::create_dir_all(Self::cover_cache_dir()?)?;
        Ok(())
    }
}

async fn load_object(path: PathBuf) -> Result<Map<String, Value>> {
    if !path.exists() {
        return Ok(Map::new());
    }
    let text = tokio::fs::read_to_string(path).await?;
    let value: Value = serde_json::from_str(&text)?;
    Ok(value.as_object().cloned().unwrap_or_default())
}

async fn save_object(path: PathBuf, obj: Map<String, Value>) -> Result<()> {
    write_json_pretty(&path, &Value::Object(obj)).await
}

fn namespace_object<'a>(
    root: &'a mut Map<String, Value>,
    namespace: &str,
) -> &'a mut Map<String, Value> {
    root.entry(namespace.to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("namespace value must be object")
}

#[tauri::command]
pub async fn frontend_storage_list(
    state: tauri::State<'_, StorageState>,
    namespace: String,
) -> Result<Vec<FrontendStorageEntry>> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::frontend_path()?).await?;
    Ok(root
        .get(&namespace)
        .and_then(Value::as_object)
        .map(|obj| {
            obj.iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| FrontendStorageEntry {
                        key: key.clone(),
                        value: value.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

#[tauri::command]
pub async fn frontend_storage_set(
    state: tauri::State<'_, StorageState>,
    namespace: String,
    key: String,
    value: String,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::frontend_path()?).await?;
    namespace_object(&mut root, &namespace).insert(key, Value::String(value));
    save_object(StorageState::frontend_path()?, root).await
}

#[tauri::command]
pub async fn frontend_storage_remove(
    state: tauri::State<'_, StorageState>,
    namespace: String,
    key: String,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::frontend_path()?).await?;
    if let Some(obj) = root.get_mut(&namespace).and_then(Value::as_object_mut) {
        obj.remove(&key);
    }
    save_object(StorageState::frontend_path()?, root).await
}

#[tauri::command]
pub async fn frontend_storage_list_namespaces(
    state: tauri::State<'_, StorageState>,
) -> Result<Vec<FrontendStorageNamespaceSummary>> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::frontend_path()?).await?;
    Ok(root
        .into_iter()
        .filter_map(|(namespace, value)| {
            value
                .as_object()
                .map(|obj| FrontendStorageNamespaceSummary {
                    namespace,
                    count: obj.len(),
                })
        })
        .collect())
}

#[tauri::command]
pub async fn storage_debug_dump(state: tauri::State<'_, StorageState>) -> Result<StorageDebugDump> {
    let _guard = state.lock.lock().await;
    Ok(StorageDebugDump {
        frontend: Value::Object(load_object(StorageState::frontend_path()?).await?),
        script_json: Value::Object(load_object(StorageState::script_json_path()?).await?),
        script_bytes: Value::Object(load_object(StorageState::script_bytes_path()?).await?),
        client_states: Value::Object(Map::new()),
        app_state_path: Some(StorageState::base_dir()?.to_string_lossy().to_string()),
        bookshelf_path: Some(
            StorageState::bookshelf_path()?
                .to_string_lossy()
                .to_string(),
        ),
    })
}

#[tauri::command]
pub async fn config_read(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
) -> Result<String> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::script_json_path()?).await?;
    Ok(root
        .get(&scope)
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&key))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

#[tauri::command]
pub async fn config_write(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
    value: String,
) -> Result<()> {
    config_write_json(state, scope, key, Value::String(value)).await
}

#[tauri::command]
pub async fn config_read_json(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
) -> Result<Option<Value>> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::script_json_path()?).await?;
    Ok(root
        .get(&scope)
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&key))
        .cloned())
}

#[tauri::command]
pub async fn config_write_json(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
    value: Value,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::script_json_path()?).await?;
    namespace_object(&mut root, &scope).insert(key, value);
    save_object(StorageState::script_json_path()?, root).await
}

#[tauri::command]
pub async fn config_delete_key(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::script_json_path()?).await?;
    if let Some(obj) = root.get_mut(&scope).and_then(Value::as_object_mut) {
        obj.remove(&key);
    }
    save_object(StorageState::script_json_path()?, root).await
}

#[tauri::command]
pub async fn config_clear(state: tauri::State<'_, StorageState>, scope: String) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::script_json_path()?).await?;
    root.remove(&scope);
    save_object(StorageState::script_json_path()?, root).await
}

#[tauri::command]
pub async fn config_read_all(
    state: tauri::State<'_, StorageState>,
    scope: String,
) -> Result<String> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::script_json_path()?).await?;
    let value = root
        .get(&scope)
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    Ok(serde_json::to_string(&value)?)
}

#[tauri::command]
pub async fn config_read_bytes(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
) -> Result<Vec<u8>> {
    let _guard = state.lock.lock().await;
    let root = load_object(StorageState::script_bytes_path()?).await?;
    let Some(encoded) = root
        .get(&scope)
        .and_then(Value::as_object)
        .and_then(|obj| obj.get(&key))
        .and_then(Value::as_str)
    else {
        return Ok(Vec::new());
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|err| BackendError::msg(format!("字节配置解码失败: {err}")))
}

#[tauri::command]
pub async fn config_write_bytes(
    state: tauri::State<'_, StorageState>,
    scope: String,
    key: String,
    value: Vec<u8>,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut root = load_object(StorageState::script_bytes_path()?).await?;
    namespace_object(&mut root, &scope).insert(
        key,
        Value::String(base64::engine::general_purpose::STANDARD.encode(value)),
    );
    save_object(StorageState::script_bytes_path()?, root).await
}
