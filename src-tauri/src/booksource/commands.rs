use super::legado_json::{parse_source_list, LegadoBookSource};
use super::parser_registry::{self, SourceData};
use super::types::*;
use super::{default_source_dir, BookSourceState, ParserType, SourceFile};
use crate::errors::{BackendError, Result};
use crate::fs_utils::{safe_file_name, stable_hash};
use crate::http;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSourceInfo {
    pub uuid: Option<String>,
    pub name: String,
    pub version: String,
    pub author: String,
    pub url: String,
    pub logo: String,
    pub description: String,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub file_name: String,
    pub download_url: String,
    pub file_size: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoManifest {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub updated_at: String,
    pub sources: Vec<RepoSourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteBookSourcePreview {
    pub download_url: String,
    pub meta: BookSourceMeta,
    pub has_explicit_uuid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoSourceSyncResult {
    pub file_name: String,
    pub uuid: String,
    pub is_consistent: bool,
    pub local_version: String,
    pub remote_version: String,
}

#[tauri::command]
pub async fn booksource_get_dir() -> Result<String> {
    let dir = default_source_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn booksource_get_dirs(state: tauri::State<'_, BookSourceState>) -> Result<Vec<String>> {
    let dirs = state.dirs().await?;
    for dir in &dirs {
        tokio::fs::create_dir_all(dir).await?;
    }
    Ok(dirs
        .into_iter()
        .map(|dir| dir.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub async fn booksource_add_dir(
    state: tauri::State<'_, BookSourceState>,
    dir_path: String,
) -> Result<()> {
    let dir = PathBuf::from(dir_path);
    tokio::fs::create_dir_all(&dir).await?;
    state.add_dir(dir).await;
    Ok(())
}

#[tauri::command]
pub async fn booksource_remove_dir(
    state: tauri::State<'_, BookSourceState>,
    dir_path: String,
) -> Result<()> {
    state.remove_dir(&PathBuf::from(dir_path)).await;
    Ok(())
}

#[tauri::command]
pub async fn booksource_pick_dir() -> Result<String> {
    Ok(String::new())
}

#[tauri::command]
pub async fn booksource_list(
    state: tauri::State<'_, BookSourceState>,
) -> Result<Vec<BookSourceMeta>> {
    list_sources(&state).await
}

#[tauri::command]
pub async fn booksource_list_streaming(
    app: AppHandle,
    state: tauri::State<'_, BookSourceState>,
    request_id: String,
) -> Result<()> {
    let items = list_sources(&state).await?;
    let payload = serde_json::json!({
        "requestId": request_id,
        "items": items,
        "done": true,
        "total": items.len(),
    });
    let _ = app.emit("booksource:batch", payload);
    Ok(())
}

#[tauri::command]
pub async fn booksource_read(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    source_dir: Option<String>,
) -> Result<String> {
    let path = resolve_source_path(&state, &file_name, source_dir.as_deref()).await?;
    Ok(tokio::fs::read_to_string(path).await?)
}

#[tauri::command]
pub async fn booksource_save(
    app: AppHandle,
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    content: String,
    source_dir: Option<String>,
) -> Result<()> {
    let dir = source_dir
        .map(PathBuf::from)
        .unwrap_or(default_source_dir()?);
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(safe_source_file_name(&file_name)?);
    tokio::fs::write(&path, content).await?;
    emit_source_changed(&app, &file_name, "save");
    let _ = list_sources(&state).await;
    Ok(())
}

#[tauri::command]
pub async fn booksource_delete(
    app: AppHandle,
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    source_dir: Option<String>,
) -> Result<()> {
    let path = resolve_source_path(&state, &file_name, source_dir.as_deref()).await?;
    tokio::fs::remove_file(path).await?;
    emit_source_changed(&app, &file_name, "delete");
    Ok(())
}

#[tauri::command]
pub async fn booksource_delete_batch(
    app: AppHandle,
    state: tauri::State<'_, BookSourceState>,
    items: Vec<BookSourceDeleteItem>,
) -> Result<BookSourceBatchDeleteResult> {
    let mut deleted = Vec::new();
    let mut errors = Vec::new();
    for item in items {
        match resolve_source_path(&state, &item.file_name, item.source_dir.as_deref()).await {
            Ok(path) => match tokio::fs::remove_file(&path).await {
                Ok(_) => deleted.push(item),
                Err(err) => errors.push(BookSourceDeleteError {
                    file_name: item.file_name,
                    source_dir: item.source_dir,
                    message: err.to_string(),
                }),
            },
            Err(err) => errors.push(BookSourceDeleteError {
                file_name: item.file_name,
                source_dir: item.source_dir,
                message: err.to_string(),
            }),
        }
    }
    emit_source_changed(&app, "", "delete-batch");
    Ok(BookSourceBatchDeleteResult { deleted, errors })
}

#[tauri::command]
pub async fn booksource_toggle(
    app: AppHandle,
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    enabled: bool,
    source_dir: Option<String>,
) -> Result<()> {
    let path = resolve_source_path(&state, &file_name, source_dir.as_deref()).await?;
    let content = tokio::fs::read_to_string(&path).await?;
    let next =
        if parser_registry::detect_parser_type(&file_name, &content) == ParserType::LegadoJsonCss {
            let mut value: Value = serde_json::from_str(&content)?;
            if let Value::Array(list) = &mut value {
                if let Some(Value::Object(obj)) = list.first_mut() {
                    obj.insert("enabled".into(), Value::Bool(enabled));
                }
            } else if let Value::Object(obj) = &mut value {
                obj.insert("enabled".into(), Value::Bool(enabled));
            }
            serde_json::to_string_pretty(&value)?
        } else {
            set_meta_enabled(&content, enabled)
        };
    tokio::fs::write(&path, next).await?;
    emit_source_changed(&app, &file_name, "toggle");
    Ok(())
}

#[tauri::command]
pub async fn booksource_resolve_path(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    source_dir: Option<String>,
) -> Result<String> {
    Ok(
        resolve_source_path(&state, &file_name, source_dir.as_deref())
            .await?
            .to_string_lossy()
            .to_string(),
    )
}

#[tauri::command]
pub async fn booksource_open_in_vscode(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    source_dir: Option<String>,
) -> Result<()> {
    let path = resolve_source_path(&state, &file_name, source_dir.as_deref()).await?;
    std::process::Command::new("code")
        .arg(path)
        .spawn()
        .map_err(|err| BackendError::msg(format!("无法调用 VS Code: {err}")))?;
    Ok(())
}

#[tauri::command]
pub async fn booksource_import_legacy_json_text(
    app: AppHandle,
    content: String,
    #[allow(unused_variables)] smart_explore_sub_categories: Option<bool>,
) -> Result<LegacyJsonImportResult> {
    import_legacy_sources(app, content).await
}

#[tauri::command]
pub async fn booksource_import_legacy_json_url(
    app: AppHandle,
    url: String,
    #[allow(unused_variables)] smart_explore_sub_categories: Option<bool>,
) -> Result<LegacyJsonImportResult> {
    let text = http::fetch_text(&url, None).await?;
    import_legacy_sources(app, text).await
}

#[tauri::command]
pub async fn booksource_search(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    keyword: String,
    page: Option<u32>,
    source_dir: Option<String>,
) -> Result<Vec<BookItem>> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    parser_registry::search(&source, &keyword, page.unwrap_or(1)).await
}

#[tauri::command]
pub async fn booksource_book_info(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    book_url: String,
    source_dir: Option<String>,
) -> Result<BookDetail> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    parser_registry::book_info(&source, &book_url).await
}

#[tauri::command]
pub async fn booksource_chapter_list(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    book_url: String,
    #[allow(unused_variables)] task_id: Option<String>,
    source_dir: Option<String>,
) -> Result<Vec<ChapterItem>> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    parser_registry::chapter_list(&source, &book_url).await
}

#[tauri::command]
pub async fn booksource_chapter_content(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    chapter_url: String,
    source_dir: Option<String>,
    #[allow(unused_variables)] category_params: Option<Value>,
) -> Result<String> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    parser_registry::chapter_content(&source, &chapter_url).await
}

#[tauri::command]
pub async fn booksource_purchase_chapter() -> Result<Value> {
    Ok(serde_json::json!({ "ok": true, "message": "免费章节无需购买" }))
}

#[tauri::command]
pub async fn booksource_call_fn() -> Result<Value> {
    Err(BackendError::msg("当前解析器不支持自定义函数调用"))
}

#[tauri::command]
pub async fn booksource_explore(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    category: String,
    page: Option<u32>,
    source_dir: Option<String>,
    #[allow(unused_variables)] no_cache: Option<bool>,
) -> Result<Value> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    parser_registry::explore(&source, &category, page.unwrap_or(1)).await
}

#[tauri::command]
pub async fn explore_clear_cache(#[allow(unused_variables)] file_name: Option<String>) -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn booksource_cancel() -> Result<()> {
    Ok(())
}

#[tauri::command]
pub async fn booksource_eval(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    source_dir: Option<String>,
    #[allow(unused_variables)] entry_code: Option<String>,
) -> Result<String> {
    let source = load_source(&state, &file_name, source_dir.as_deref()).await?;
    Ok(parser_registry::capabilities(&source).join(","))
}

#[tauri::command]
pub async fn js_eval(code: String) -> Result<String> {
    Ok(format!("JS 引擎尚未接入。收到代码 {} 字符。", code.len()))
}

#[tauri::command]
pub async fn script_repl_eval() -> Result<String> {
    Err(BackendError::msg("脚本 REPL 尚未接入"))
}

#[tauri::command]
pub async fn booksource_run_tests(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    #[allow(unused_variables)] timeout_secs: Option<u64>,
) -> Result<TestRunResult> {
    let source = load_source(&state, &file_name, None).await?;
    let mut steps = Vec::new();

    let start = Instant::now();
    let search = parser_registry::search(&source, "剑来", 1).await;
    let search_items = match search {
        Ok(items) => {
            let passed = !items.is_empty();
            steps.push(TestStepResult {
                step: "search".into(),
                passed,
                message: if passed {
                    format!("搜索返回 {} 条", items.len())
                } else {
                    "搜索结果为空".into()
                },
                duration_ms: start.elapsed().as_millis(),
            });
            items
        }
        Err(err) => {
            steps.push(TestStepResult {
                step: "search".into(),
                passed: false,
                message: err.to_string(),
                duration_ms: start.elapsed().as_millis(),
            });
            return Ok(finish_test(file_name, steps));
        }
    };

    let Some(first_book) = search_items.first() else {
        return Ok(finish_test(file_name, steps));
    };

    let start = Instant::now();
    let detail = parser_registry::book_info(&source, &first_book.book_url).await;
    let detail = match detail {
        Ok(detail) => {
            let passed = !detail.name.is_empty() && detail.toc_url.is_some();
            steps.push(TestStepResult {
                step: "bookInfo".into(),
                passed,
                message: format!("详情: {}", detail.name),
                duration_ms: start.elapsed().as_millis(),
            });
            detail
        }
        Err(err) => {
            steps.push(TestStepResult {
                step: "bookInfo".into(),
                passed: false,
                message: err.to_string(),
                duration_ms: start.elapsed().as_millis(),
            });
            return Ok(finish_test(file_name, steps));
        }
    };

    let start = Instant::now();
    let toc_url = detail.toc_url.clone().unwrap_or(detail.book_url.clone());
    let chapters = parser_registry::chapter_list(&source, &toc_url).await;
    let chapters = match chapters {
        Ok(chapters) => {
            let passed = !chapters.is_empty();
            steps.push(TestStepResult {
                step: "chapterList".into(),
                passed,
                message: format!("目录返回 {} 章", chapters.len()),
                duration_ms: start.elapsed().as_millis(),
            });
            chapters
        }
        Err(err) => {
            steps.push(TestStepResult {
                step: "chapterList".into(),
                passed: false,
                message: err.to_string(),
                duration_ms: start.elapsed().as_millis(),
            });
            return Ok(finish_test(file_name, steps));
        }
    };

    if let Some(first_chapter) = chapters.first() {
        let start = Instant::now();
        let content = parser_registry::chapter_content(&source, &first_chapter.url).await;
        match content {
            Ok(content) => steps.push(TestStepResult {
                step: "chapterContent".into(),
                passed: !content.trim().is_empty(),
                message: format!("正文 {} 字符", content.chars().count()),
                duration_ms: start.elapsed().as_millis(),
            }),
            Err(err) => steps.push(TestStepResult {
                step: "chapterContent".into(),
                passed: false,
                message: err.to_string(),
                duration_ms: start.elapsed().as_millis(),
            }),
        }
    }

    Ok(finish_test(file_name, steps))
}

#[tauri::command]
pub async fn booksource_check_update(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
) -> Result<UpdateCheckResult> {
    let source = load_source(&state, &file_name, None).await?;
    let meta = meta_from_source(&source, tokio::fs::metadata(&source.path).await?);
    Ok(UpdateCheckResult {
        file_name,
        uuid: meta.uuid,
        has_update: false,
        local_version: meta.version,
        remote_version: String::new(),
    })
}

#[tauri::command]
pub async fn booksource_apply_update() -> Result<()> {
    Err(BackendError::msg(
        "当前书源未配置更新地址或更新功能尚未接入",
    ))
}

#[tauri::command]
pub async fn repository_fetch(url: String) -> Result<RepoManifest> {
    let text = http::fetch_text(&url, None).await?;
    Ok(serde_json::from_str(&text)?)
}

#[tauri::command]
pub async fn repository_install(download_url: String, file_name: String) -> Result<()> {
    let text = http::fetch_text(&download_url, None).await?;
    let dir = default_source_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join(safe_source_file_name(&file_name)?), text).await?;
    Ok(())
}

#[tauri::command]
pub async fn repository_preview_source(download_url: String) -> Result<RemoteBookSourcePreview> {
    let text = http::fetch_text(&download_url, None).await?;
    let dir = default_source_dir()?;
    let file_name = safe_file_name(
        &UrlFileName::from_url(&download_url).unwrap_or_else(|| "remote.json".into()),
        "json",
    );
    let temp_path = dir.join(&file_name);
    let parser_type = parser_registry::detect_parser_type(&file_name, &text);
    let data = match parser_type {
        ParserType::LegadoJsonCss => {
            SourceData::LegadoJson(super::legado_json::parse_source_text(&text)?)
        }
        ParserType::TauriJs => SourceData::TauriJs(text.clone()),
    };
    let source = SourceFile {
        file_name: file_name.clone(),
        path: temp_path,
        source_dir: dir,
        parser_type,
        data,
    };
    let meta = meta_from_source(&source, fake_metadata());
    Ok(RemoteBookSourcePreview {
        download_url,
        meta,
        has_explicit_uuid: false,
    })
}

#[tauri::command]
pub async fn repository_check_source_sync(
    state: tauri::State<'_, BookSourceState>,
    file_name: String,
    #[allow(unused_variables)] download_url: String,
    #[allow(unused_variables)] expected_uuid: Option<String>,
) -> Result<RepoSourceSyncResult> {
    let source = load_source(&state, &file_name, None).await?;
    let meta = meta_from_source(&source, tokio::fs::metadata(&source.path).await?);
    Ok(RepoSourceSyncResult {
        file_name,
        uuid: meta.uuid,
        is_consistent: false,
        local_version: meta.version,
        remote_version: String::new(),
    })
}

#[tauri::command]
pub async fn booksource_save_draft(file_name: String, content: String) -> Result<()> {
    let dir = default_source_dir()?.join(".drafts");
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::write(dir.join(safe_source_file_name(&file_name)?), content).await?;
    Ok(())
}

#[tauri::command]
pub async fn booksource_delete_draft(file_name: String) -> Result<()> {
    let path = default_source_dir()?
        .join(".drafts")
        .join(safe_source_file_name(&file_name)?);
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

async fn list_sources(state: &BookSourceState) -> Result<Vec<BookSourceMeta>> {
    let dirs = state.dirs().await?;
    let mut out = Vec::new();
    for dir in dirs {
        tokio::fs::create_dir_all(&dir).await?;
        let mut entries = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ext != "js" && ext != "json" {
                continue;
            }
            match parser_registry::load_source_file(path.clone(), dir.clone()).await {
                Ok(source) => {
                    let metadata = tokio::fs::metadata(&path).await?;
                    out.push(meta_from_source(&source, metadata));
                }
                Err(err) => eprintln!("[booksource_list] skip {}: {err}", path.display()),
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

async fn resolve_source_path(
    state: &BookSourceState,
    file_name: &str,
    source_dir: Option<&str>,
) -> Result<PathBuf> {
    let safe = safe_source_file_name(file_name)?;
    if let Some(dir) = source_dir.filter(|value| !value.trim().is_empty()) {
        let path = PathBuf::from(dir).join(&safe);
        if path.exists() {
            return Ok(path);
        }
    }
    for dir in state.dirs().await? {
        let path = dir.join(&safe);
        if path.exists() {
            return Ok(path);
        }
    }
    Err(BackendError::msg(format!("书源不存在: {file_name}")))
}

async fn load_source(
    state: &BookSourceState,
    file_name: &str,
    source_dir: Option<&str>,
) -> Result<SourceFile> {
    let path = resolve_source_path(state, file_name, source_dir).await?;
    let source_dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(default_source_dir()?);
    parser_registry::load_source_file(path, source_dir).await
}

async fn import_legacy_sources(app: AppHandle, content: String) -> Result<LegacyJsonImportResult> {
    let sources = parse_source_list(&content)?;
    let dir = default_source_dir()?;
    tokio::fs::create_dir_all(&dir).await?;
    let mut result = LegacyJsonImportResult {
        imported: 0,
        skipped: 0,
        files: Vec::new(),
        errors: Vec::new(),
    };
    for source in sources {
        if source.book_source_name.trim().is_empty() || source.book_source_url.trim().is_empty() {
            result.skipped += 1;
            result
                .errors
                .push("缺少 bookSourceName 或 bookSourceUrl".into());
            continue;
        }
        let file_name = legacy_file_name(&source);
        let path = dir.join(&file_name);
        match serde_json::to_string_pretty(&source) {
            Ok(text) => {
                tokio::fs::write(&path, text).await?;
                result.imported += 1;
                result.files.push(file_name.clone());
                emit_source_changed(&app, &file_name, "import-legacy-json");
            }
            Err(err) => {
                result.skipped += 1;
                result
                    .errors
                    .push(format!("{}: {err}", source.book_source_name));
            }
        }
    }
    Ok(result)
}

fn meta_from_source(source: &SourceFile, metadata: std::fs::Metadata) -> BookSourceMeta {
    match &source.data {
        SourceData::LegadoJson(rule) => legado_meta(source, rule, metadata),
        SourceData::TauriJs(content) => js_meta(source, content, metadata),
    }
}

fn legado_meta(
    source: &SourceFile,
    rule: &LegadoBookSource,
    metadata: std::fs::Metadata,
) -> BookSourceMeta {
    let name = rule.book_source_name.trim().to_string();
    let url = rule.book_source_url.trim().to_string();
    let uuid = stable_hash(&format!("{name}|{url}|{}", source.file_name));
    BookSourceMeta {
        source_key: format!(
            "{}::{}",
            source.source_dir.to_string_lossy(),
            source.file_name
        ),
        uuid,
        file_name: source.file_name.clone(),
        name,
        url: url.clone(),
        urls: vec![url],
        homepage_url: None,
        author: None,
        logo: None,
        description: Some(format!(
            "Legado JSON 规则书源；parserType={}",
            ParserType::LegadoJsonCss.as_str()
        )),
        enabled: rule.enabled.unwrap_or(true),
        file_size: metadata.len(),
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default(),
        source_dir: source.source_dir.to_string_lossy().to_string(),
        source_type: source_type_from_legado(&rule.book_source_type),
        version: "legacy-json".into(),
        update_url: None,
        tags: split_tags(&rule.book_source_group),
        min_delay_ms: 0,
        require_urls: Vec::new(),
        has_explore: rule.enabled_explore,
        parser_type: Some(ParserType::LegadoJsonCss.as_str().into()),
    }
}

fn js_meta(source: &SourceFile, content: &str, metadata: std::fs::Metadata) -> BookSourceMeta {
    let name = read_meta(content, "@name").unwrap_or_else(|| source.file_name.clone());
    let url = read_meta(content, "@url").unwrap_or_default();
    let uuid = read_meta(content, "@uuid").unwrap_or_else(|| stable_hash(&format!("{name}|{url}")));
    BookSourceMeta {
        source_key: format!(
            "{}::{}",
            source.source_dir.to_string_lossy(),
            source.file_name
        ),
        uuid,
        file_name: source.file_name.clone(),
        name,
        url: url.clone(),
        urls: if url.is_empty() {
            Vec::new()
        } else {
            vec![url]
        },
        homepage_url: read_meta(content, "@homepage").or_else(|| read_meta(content, "@homeurl")),
        author: read_meta(content, "@author"),
        logo: read_meta(content, "@logo"),
        description: read_meta(content, "@description"),
        enabled: read_meta(content, "@enabled")
            .map(|value| value != "false")
            .unwrap_or(true),
        file_size: metadata.len(),
        modified_at: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default(),
        source_dir: source.source_dir.to_string_lossy().to_string(),
        source_type: read_meta(content, "@type").unwrap_or_else(|| "novel".into()),
        version: read_meta(content, "@version").unwrap_or_default(),
        update_url: read_meta(content, "@updateUrl"),
        tags: read_meta(content, "@tags")
            .map(|value| split_tags(&value))
            .unwrap_or_default(),
        min_delay_ms: 0,
        require_urls: Vec::new(),
        has_explore: Some(content.contains("function explore") || content.contains("explore =")),
        parser_type: Some(ParserType::TauriJs.as_str().into()),
    }
}

fn source_type_from_legado(value: &Value) -> String {
    match value {
        Value::Number(num) if num.as_i64() == Some(1) => "audio".into(),
        Value::Number(num) if num.as_i64() == Some(2) => "comic".into(),
        Value::Number(num) if num.as_i64() == Some(3) => "video".into(),
        Value::String(value) if value == "1" => "audio".into(),
        Value::String(value) if value == "2" => "comic".into(),
        Value::String(value) if value == "3" => "video".into(),
        _ => "novel".into(),
    }
}

fn split_tags(value: &str) -> Vec<String> {
    value
        .split([',', '，', ';', '；'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn read_meta(content: &str, key: &str) -> Option<String> {
    content.lines().take(80).find_map(|line| {
        let line = line.trim().trim_start_matches("//").trim();
        line.strip_prefix(key)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn set_meta_enabled(content: &str, enabled: bool) -> String {
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        if line.trim_start().starts_with("// @enabled") {
            lines.push(format!(
                "// @enabled     {}",
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
            format!("// @enabled     {}", if enabled { "true" } else { "false" }),
        );
    }
    lines.join("\n")
}

fn safe_source_file_name(file_name: &str) -> Result<String> {
    let safe = safe_file_name(file_name, "");
    if safe != file_name || safe.contains("..") {
        return Err(BackendError::msg("文件名不能包含路径或特殊字符"));
    }
    let lower = safe.to_ascii_lowercase();
    if !lower.ends_with(".js") && !lower.ends_with(".json") {
        return Err(BackendError::msg("书源文件必须是 .js 或 .json"));
    }
    Ok(safe)
}

fn legacy_file_name(source: &LegadoBookSource) -> String {
    let name = source.book_source_name.replace('🎉', "").trim().to_string();
    let hash = stable_hash(&format!(
        "{}|{}",
        source.book_source_name, source.book_source_url
    ));
    safe_file_name(&format!("{name}-{hash}.json"), "json")
}

fn emit_source_changed(app: &AppHandle, file_name: &str, reason: &str) {
    let _ = app.emit(
        "booksource:changed",
        serde_json::json!({
            "fileName": if file_name.is_empty() { Value::Null } else { Value::String(file_name.to_string()) },
            "reason": reason,
        }),
    );
}

fn finish_test(file_name: String, steps: Vec<TestStepResult>) -> TestRunResult {
    let all_passed = !steps.is_empty() && steps.iter().all(|step| step.passed);
    TestRunResult {
        file_name,
        steps,
        all_passed,
    }
}

struct UrlFileName;

impl UrlFileName {
    fn from_url(value: &str) -> Option<String> {
        url::Url::parse(value)
            .ok()
            .and_then(|url| {
                url.path_segments()
                    .and_then(|mut segments| segments.next_back().map(str::to_string))
            })
            .filter(|value| !value.is_empty())
    }
}

fn fake_metadata() -> std::fs::Metadata {
    std::fs::metadata(".").expect("current dir metadata")
}
