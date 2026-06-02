use crate::errors::{BackendError, Result};
use crate::fs_utils::{app_data_dir, write_json_pretty};
use crate::http::BUILTIN_USER_AGENT;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tauri::Emitter;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub http_user_agent: String,
    pub http_follow_redirects: bool,
    pub http_connect_timeout_secs: u64,
    pub http_ignore_tls_errors: bool,
    pub http_doh_server: String,
    pub proxy_mode: String,
    pub proxy_type: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_username: String,
    pub proxy_password: String,
    pub engine_timeout_secs: u64,
    pub booksource_watcher_enabled: bool,
    pub browser_probe_enabled: bool,
    pub browser_probe_user_agent: String,
    pub browser_probe_timeout_secs: u64,
    pub browser_probe_visible_by_default: bool,
    pub browser_probe_force_visible: bool,
    pub browser_probe_persist_profile: bool,
    pub comic_cache_enabled: bool,
    pub ui_layout_mode: String,
    pub ui_theme: String,
    pub ui_enable_aplus_tracking: bool,
    pub power_keep_awake_on_tts: bool,
    pub power_reader_awake_mode: String,
    pub power_reader_awake_timeout_secs: u64,
    pub windows_main_window_width: u32,
    pub windows_main_window_height: u32,
    pub video_player_type: String,
    pub video_default_rate: f64,
    pub video_auto_next: bool,
    pub video_quality_prefer: String,
    pub video_remember_progress: bool,
    pub video_seek_step_secs: u64,
    pub video_vjs_preload: String,
    pub video_vjs_pip: bool,
    pub video_xg_download: bool,
    pub video_dp_danmaku: bool,
    pub video_dp_theme: String,
    pub video_autoplay: bool,
    pub web_server_enabled: bool,
    pub web_server_port: u16,
    pub web_server_dist_path: String,
    pub web_remote_debug_enabled: bool,
    pub web_remote_debug_host: String,
    pub web_remote_debug_port: u16,
    pub request_min_delay_ms: u64,
    pub cache_prefetch_count: i32,
    pub cache_prefetch_concurrency: u32,
    pub export_prefetch_concurrency: u32,
    pub sync_enabled: bool,
    pub sync_provider: String,
    pub sync_profile_id: String,
    pub sync_webdav_url: String,
    pub sync_webdav_username: String,
    pub sync_webdav_root_dir: String,
    pub sync_webdav_allow_http: bool,
    pub sync_trigger_enabled: bool,
    pub sync_timer_enabled: bool,
    pub sync_timer_interval_secs: u64,
    pub sync_trigger_on_startup: bool,
    pub sync_trigger_on_resume: bool,
    pub sync_trigger_on_unlock_resume: bool,
    pub sync_trigger_on_bookshelf_change: bool,
    pub sync_trigger_on_booksource_change: bool,
    pub sync_trigger_on_settings_change: bool,
    pub sync_scope_bookshelf: bool,
    pub sync_scope_reading_progress: bool,
    pub sync_scope_booksources: bool,
    pub sync_scope_reader_settings: bool,
    pub sync_scope_app_settings: bool,
    pub sync_scope_source_flags: bool,
    pub sync_scope_extensions: bool,
    pub sync_scope_script_config: bool,
    pub sync_mobile_foreground_only: bool,
    pub sync_mobile_screen_on_only: bool,
    pub sync_mobile_wifi_only: bool,
    pub sync_mobile_pause_on_low_battery: bool,
    pub sync_mobile_startup_delay_ms: u64,
    pub sync_mobile_resume_delay_ms: u64,
    pub sync_baidu_app_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            http_user_agent: BUILTIN_USER_AGENT.to_string(),
            http_follow_redirects: true,
            http_connect_timeout_secs: 10,
            http_ignore_tls_errors: true,
            http_doh_server: "none".into(),
            proxy_mode: "system".into(),
            proxy_type: "http".into(),
            proxy_host: String::new(),
            proxy_port: 0,
            proxy_username: String::new(),
            proxy_password: String::new(),
            engine_timeout_secs: 30,
            booksource_watcher_enabled: false,
            browser_probe_enabled: true,
            browser_probe_user_agent: String::new(),
            browser_probe_timeout_secs: 0,
            browser_probe_visible_by_default: false,
            browser_probe_force_visible: false,
            browser_probe_persist_profile: true,
            comic_cache_enabled: true,
            ui_layout_mode: "auto".into(),
            ui_theme: "auto".into(),
            ui_enable_aplus_tracking: true,
            power_keep_awake_on_tts: false,
            power_reader_awake_mode: "off".into(),
            power_reader_awake_timeout_secs: 600,
            windows_main_window_width: 0,
            windows_main_window_height: 0,
            video_player_type: "videojs".into(),
            video_default_rate: 1.0,
            video_auto_next: true,
            video_quality_prefer: "auto".into(),
            video_remember_progress: true,
            video_seek_step_secs: 10,
            video_vjs_preload: "auto".into(),
            video_vjs_pip: true,
            video_xg_download: false,
            video_dp_danmaku: false,
            video_dp_theme: "#00b1ff".into(),
            video_autoplay: false,
            web_server_enabled: false,
            web_server_port: 7688,
            web_server_dist_path: String::new(),
            web_remote_debug_enabled: false,
            web_remote_debug_host: String::new(),
            web_remote_debug_port: 8080,
            request_min_delay_ms: 300,
            cache_prefetch_count: 3,
            cache_prefetch_concurrency: 2,
            export_prefetch_concurrency: 3,
            sync_enabled: false,
            sync_provider: "webdav".into(),
            sync_profile_id: "default".into(),
            sync_webdav_url: String::new(),
            sync_webdav_username: String::new(),
            sync_webdav_root_dir: "legado-sync".into(),
            sync_webdav_allow_http: false,
            sync_trigger_enabled: true,
            sync_timer_enabled: false,
            sync_timer_interval_secs: 900,
            sync_trigger_on_startup: true,
            sync_trigger_on_resume: true,
            sync_trigger_on_unlock_resume: true,
            sync_trigger_on_bookshelf_change: false,
            sync_trigger_on_booksource_change: false,
            sync_trigger_on_settings_change: false,
            sync_scope_bookshelf: true,
            sync_scope_reading_progress: true,
            sync_scope_booksources: true,
            sync_scope_reader_settings: true,
            sync_scope_app_settings: true,
            sync_scope_source_flags: false,
            sync_scope_extensions: false,
            sync_scope_script_config: false,
            sync_mobile_foreground_only: true,
            sync_mobile_screen_on_only: true,
            sync_mobile_wifi_only: true,
            sync_mobile_pause_on_low_battery: true,
            sync_mobile_startup_delay_ms: 5000,
            sync_mobile_resume_delay_ms: 1500,
            sync_baidu_app_name: "legado-tauri".into(),
        }
    }
}

pub struct AppConfigState {
    lock: Mutex<()>,
}

impl AppConfigState {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }

    fn path() -> Result<PathBuf> {
        Ok(app_data_dir()?.join("app-config.json"))
    }

    async fn load_inner() -> Result<AppConfig> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(AppConfig::default());
        }
        let text = tokio::fs::read_to_string(path).await?;
        let mut value: Value = serde_json::from_str(&text)?;
        let defaults = serde_json::to_value(AppConfig::default())?;
        merge_defaults(&mut value, &defaults);
        Ok(serde_json::from_value(value)?)
    }

    async fn save_inner(config: &AppConfig) -> Result<()> {
        write_json_pretty(&Self::path()?, &serde_json::to_value(config)?).await
    }

    pub async fn load(&self) -> Result<AppConfig> {
        let _guard = self.lock.lock().await;
        Self::load_inner().await
    }

    pub async fn save(&self, config: &AppConfig) -> Result<()> {
        let _guard = self.lock.lock().await;
        Self::save_inner(config).await
    }
}

fn merge_defaults(value: &mut Value, defaults: &Value) {
    let (Value::Object(value), Value::Object(defaults)) = (value, defaults) else {
        return;
    };
    for (key, default_value) in defaults {
        if !value.contains_key(key) {
            value.insert(key.clone(), default_value.clone());
        }
    }
}

#[tauri::command]
pub async fn app_config_get_all(state: tauri::State<'_, AppConfigState>) -> Result<AppConfig> {
    state.load().await
}

#[tauri::command]
pub async fn app_config_set(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppConfigState>,
    key: String,
    value: String,
) -> Result<()> {
    let mut config = state.load().await?;
    let mut json = serde_json::to_value(&config)?;
    let obj = json
        .as_object_mut()
        .ok_or_else(|| BackendError::msg("配置序列化失败"))?;
    let current = obj
        .get(&key)
        .cloned()
        .ok_or_else(|| BackendError::msg(format!("未知配置项: {key}")))?;
    obj.insert(key, parse_config_value(value, &current)?);
    config = serde_json::from_value(json)?;
    state.save(&config).await?;
    let _ = app.emit("app_config:changed", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn app_config_reset(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppConfigState>,
    key: String,
) -> Result<()> {
    let mut config = state.load().await?;
    let mut json = serde_json::to_value(&config)?;
    let default_json = serde_json::to_value(AppConfig::default())?;
    let default_value = default_json
        .get(&key)
        .cloned()
        .ok_or_else(|| BackendError::msg(format!("未知配置项: {key}")))?;
    json.as_object_mut()
        .ok_or_else(|| BackendError::msg("配置序列化失败"))?
        .insert(key, default_value);
    config = serde_json::from_value(json)?;
    state.save(&config).await?;
    let _ = app.emit("app_config:changed", serde_json::json!({}));
    Ok(())
}

fn parse_config_value(value: String, current: &Value) -> Result<Value> {
    match current {
        Value::Bool(_) => Ok(Value::Bool(value == "true")),
        Value::Number(num) if num.is_i64() || num.is_u64() => {
            let parsed = value
                .parse::<i64>()
                .map_err(|_| BackendError::msg(format!("配置值不是整数: {value}")))?;
            Ok(Value::Number(parsed.into()))
        }
        Value::Number(num) if num.is_f64() => {
            let parsed = value
                .parse::<f64>()
                .map_err(|_| BackendError::msg(format!("配置值不是数字: {value}")))?;
            serde_json::Number::from_f64(parsed)
                .map(Value::Number)
                .ok_or_else(|| BackendError::msg("配置值不是有效数字"))
        }
        _ => Ok(Value::String(value)),
    }
}
