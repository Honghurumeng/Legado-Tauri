use crate::errors::{BackendError, Result};
use crate::fs_utils::{stable_hash, write_json_pretty};
use crate::storage::StorageState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfBook {
    pub id: String,
    pub name: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_referer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub book_url: String,
    pub file_name: String,
    pub source_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chapter: Option<String>,
    pub added_at: i64,
    pub last_read_at: i64,
    pub read_chapter_index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_chapter_url: Option<String>,
    pub total_chapters: usize,
    pub source_type: String,
    pub read_page_index: i32,
    pub read_scroll_ratio: f64,
    pub read_playback_time: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reader_settings: Option<String>,
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddBookPayload {
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub intro: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub book_url: String,
    #[serde(default)]
    pub last_chapter: Option<String>,
    #[serde(default)]
    pub source_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateShelfBookPayload {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub intro: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    pub book_url: String,
    pub file_name: String,
    pub source_name: String,
    #[serde(default)]
    pub last_chapter: Option<String>,
    pub total_chapters: usize,
    pub read_chapter_index: i32,
    #[serde(default)]
    pub read_chapter_url: Option<String>,
    pub source_type: String,
    #[serde(default)]
    pub added_at: Option<i64>,
    #[serde(default)]
    pub last_read_at: Option<i64>,
    #[serde(default)]
    pub read_page_index: Option<i32>,
    #[serde(default)]
    pub read_scroll_ratio: Option<f64>,
    #[serde(default)]
    pub read_playback_time: Option<f64>,
    #[serde(default)]
    pub reader_settings: Option<String>,
    #[serde(default)]
    pub is_private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedChapter {
    pub index: i32,
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vip: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeProgress {
    pub time: f64,
    pub duration: f64,
    #[serde(rename = "lastPlayedAt")]
    pub last_played_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSwitchRestoreResult {
    pub book: ShelfBook,
    pub chapters: Vec<CachedChapter>,
}

pub struct BookshelfState {
    lock: Mutex<()>,
}

impl BookshelfState {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
        }
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

async fn load_books() -> Result<Vec<ShelfBook>> {
    let path = StorageState::bookshelf_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&text)?)
}

async fn save_books(books: &[ShelfBook]) -> Result<()> {
    write_json_pretty(
        &StorageState::bookshelf_path()?,
        &serde_json::to_value(books)?,
    )
    .await
}

fn chapter_path(id: &str) -> Result<PathBuf> {
    Ok(StorageState::chapters_dir()?.join(format!("{id}.json")))
}

fn content_dir(id: &str) -> Result<PathBuf> {
    Ok(StorageState::contents_dir()?.join(id))
}

fn content_path(id: &str, chapter_index: i32) -> Result<PathBuf> {
    Ok(content_dir(id)?.join(format!("{chapter_index}.txt")))
}

fn episode_progress_path(id: &str) -> Result<PathBuf> {
    Ok(content_dir(id)?.join("episode-progress.json"))
}

fn book_from_update(payload: UpdateShelfBookPayload, current: Option<&ShelfBook>) -> ShelfBook {
    ShelfBook {
        id: payload.id,
        name: payload.name,
        author: payload.author.unwrap_or_default(),
        cover_url: payload.cover_url,
        cover_referer: current.and_then(|book| book.cover_referer.clone()),
        intro: payload.intro,
        kind: payload.kind,
        group_id: payload.group_id,
        book_url: payload.book_url,
        file_name: payload.file_name,
        source_name: payload.source_name,
        last_chapter: payload.last_chapter,
        added_at: payload
            .added_at
            .unwrap_or_else(|| current.map(|book| book.added_at).unwrap_or_else(now_ms)),
        last_read_at: payload
            .last_read_at
            .unwrap_or_else(|| current.map(|book| book.last_read_at).unwrap_or_else(now_ms)),
        read_chapter_index: payload.read_chapter_index,
        read_chapter_url: payload.read_chapter_url,
        total_chapters: payload.total_chapters,
        source_type: payload.source_type,
        read_page_index: payload
            .read_page_index
            .unwrap_or_else(|| current.map(|book| book.read_page_index).unwrap_or(-1)),
        read_scroll_ratio: payload
            .read_scroll_ratio
            .unwrap_or_else(|| current.map(|book| book.read_scroll_ratio).unwrap_or(-1.0)),
        read_playback_time: payload
            .read_playback_time
            .unwrap_or_else(|| current.map(|book| book.read_playback_time).unwrap_or(-1.0)),
        reader_settings: payload
            .reader_settings
            .or_else(|| current.and_then(|book| book.reader_settings.clone())),
        is_private: payload
            .is_private
            .unwrap_or_else(|| current.map(|book| book.is_private).unwrap_or(false)),
    }
}

#[tauri::command]
pub async fn bookshelf_list(state: tauri::State<'_, BookshelfState>) -> Result<Vec<ShelfBook>> {
    let _guard = state.lock.lock().await;
    load_books().await
}

#[tauri::command]
pub async fn bookshelf_add(
    app: AppHandle,
    state: tauri::State<'_, BookshelfState>,
    book: AddBookPayload,
    file_name: String,
    source_name: String,
) -> Result<ShelfBook> {
    let _guard = state.lock.lock().await;
    let mut books = load_books().await?;
    if let Some(existing) = books
        .iter()
        .find(|item| item.book_url == book.book_url && item.file_name == file_name)
    {
        return Ok(existing.clone());
    }
    let id = stable_hash(&format!(
        "{}|{}|{}",
        book.book_url,
        file_name,
        uuid::Uuid::new_v4()
    ));
    let now = now_ms();
    let shelf_book = ShelfBook {
        id,
        name: book.name,
        author: book.author.unwrap_or_default(),
        cover_url: book.cover_url,
        cover_referer: None,
        intro: book.intro,
        kind: book.kind,
        group_id: book.group_id,
        book_url: book.book_url,
        file_name,
        source_name,
        last_chapter: book.last_chapter,
        added_at: now,
        last_read_at: now,
        read_chapter_index: -1,
        read_chapter_url: None,
        total_chapters: 0,
        source_type: book.source_type.unwrap_or_else(|| "novel".into()),
        read_page_index: -1,
        read_scroll_ratio: -1.0,
        read_playback_time: -1.0,
        reader_settings: None,
        is_private: false,
    };
    books.push(shelf_book.clone());
    save_books(&books).await?;
    let _ = app.emit("bookshelf:changed", serde_json::json!({}));
    Ok(shelf_book)
}

#[tauri::command]
pub async fn bookshelf_remove(
    app: AppHandle,
    state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut books = load_books().await?;
    books.retain(|book| book.id != id);
    save_books(&books).await?;
    let _ = tokio::fs::remove_file(chapter_path(&id)?).await;
    let _ = tokio::fs::remove_dir_all(content_dir(&id)?).await;
    let _ = app.emit("bookshelf:changed", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_get(
    state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<ShelfBook> {
    let _guard = state.lock.lock().await;
    load_books()
        .await?
        .into_iter()
        .find(|book| book.id == id)
        .ok_or_else(|| BackendError::msg(format!("书籍不存在: {id}")))
}

#[tauri::command]
pub async fn bookshelf_update_book(
    app: AppHandle,
    state: tauri::State<'_, BookshelfState>,
    book: UpdateShelfBookPayload,
    chapters: Option<Vec<CachedChapter>>,
) -> Result<ShelfBook> {
    let _guard = state.lock.lock().await;
    let mut books = load_books().await?;
    let idx = books
        .iter()
        .position(|item| item.id == book.id)
        .ok_or_else(|| BackendError::msg(format!("书籍不存在: {}", book.id)))?;
    let next = book_from_update(book, books.get(idx));
    books[idx] = next.clone();
    save_books(&books).await?;
    if let Some(chapters) = chapters {
        write_json_pretty(&chapter_path(&next.id)?, &serde_json::to_value(chapters)?).await?;
    }
    let _ = app.emit("bookshelf:changed", serde_json::json!({}));
    Ok(next)
}

#[tauri::command]
pub async fn bookshelf_update_progress(
    app: AppHandle,
    state: tauri::State<'_, BookshelfState>,
    id: String,
    chapter_index: i32,
    chapter_url: String,
    page_index: Option<i32>,
    scroll_ratio: Option<f64>,
    playback_time: Option<f64>,
    reader_settings: Option<String>,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut books = load_books().await?;
    let book = books
        .iter_mut()
        .find(|book| book.id == id)
        .ok_or_else(|| BackendError::msg(format!("书籍不存在: {id}")))?;
    book.read_chapter_index = chapter_index;
    book.read_chapter_url = Some(chapter_url.clone());
    book.last_read_at = now_ms();
    if let Some(value) = page_index {
        book.read_page_index = value;
    }
    if let Some(value) = scroll_ratio {
        book.read_scroll_ratio = value;
    }
    if let Some(value) = playback_time {
        book.read_playback_time = value;
    }
    if let Some(value) = reader_settings {
        book.reader_settings = Some(value);
    }
    let payload = serde_json::json!({
        "id": id,
        "readChapterIndex": book.read_chapter_index,
        "readChapterUrl": chapter_url,
        "readPageIndex": book.read_page_index,
        "readScrollRatio": book.read_scroll_ratio,
        "readPlaybackTime": book.read_playback_time,
        "lastReadAt": book.last_read_at,
    });
    save_books(&books).await?;
    let _ = app.emit("bookshelf:progress-updated", payload);
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_set_private(
    app: AppHandle,
    state: tauri::State<'_, BookshelfState>,
    id: String,
    is_private: bool,
) -> Result<()> {
    let _guard = state.lock.lock().await;
    let mut books = load_books().await?;
    if let Some(book) = books.iter_mut().find(|book| book.id == id) {
        book.is_private = is_private;
    }
    save_books(&books).await?;
    let _ = app.emit("bookshelf:changed", serde_json::json!({}));
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_save_chapters(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapters: Vec<CachedChapter>,
) -> Result<()> {
    write_json_pretty(&chapter_path(&id)?, &serde_json::to_value(chapters)?).await
}

#[tauri::command]
pub async fn bookshelf_get_chapters(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<Vec<CachedChapter>> {
    let path = chapter_path(&id)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&text)?)
}

#[tauri::command]
pub async fn bookshelf_save_content(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapter_index: i32,
    content: String,
) -> Result<()> {
    tokio::fs::create_dir_all(content_dir(&id)?).await?;
    tokio::fs::write(content_path(&id, chapter_index)?, content).await?;
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_get_content(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapter_index: i32,
) -> Result<Option<String>> {
    let path = content_path(&id, chapter_index)?;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(tokio::fs::read_to_string(path).await?))
}

#[tauri::command]
pub async fn bookshelf_delete_content(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapter_index: i32,
) -> Result<()> {
    let path = content_path(&id, chapter_index)?;
    if path.exists() {
        tokio::fs::remove_file(path).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_get_cached_indices(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<Vec<i32>> {
    let dir = content_dir(&id)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut entries = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("txt") {
            continue;
        }
        if let Some(idx) = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse().ok())
        {
            out.push(idx);
        }
    }
    out.sort_unstable();
    Ok(out)
}

#[tauri::command]
pub async fn bookshelf_save_txt_chapters(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapters: Vec<Value>,
) -> Result<()> {
    tokio::fs::create_dir_all(content_dir(&id)?).await?;
    for chapter in chapters {
        let index = chapter
            .get("index")
            .and_then(Value::as_i64)
            .unwrap_or_default() as i32;
        let content = chapter
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default();
        tokio::fs::write(content_path(&id, index)?, content).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn bookshelf_get_episode_progress(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<std::collections::HashMap<String, EpisodeProgress>> {
    let path = episode_progress_path(&id)?;
    if !path.exists() {
        return Ok(std::collections::HashMap::new());
    }
    let text = tokio::fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&text)?)
}

#[tauri::command]
pub async fn bookshelf_save_episode_progress(
    _state: tauri::State<'_, BookshelfState>,
    id: String,
    chapter_url: String,
    time: f64,
    duration: f64,
) -> Result<()> {
    let mut map = bookshelf_get_episode_progress(_state, id.clone()).await?;
    map.insert(
        chapter_url,
        EpisodeProgress {
            time,
            duration,
            last_played_at: now_ms(),
        },
    );
    tokio::fs::create_dir_all(content_dir(&id)?).await?;
    write_json_pretty(&episode_progress_path(&id)?, &serde_json::to_value(map)?).await
}

#[tauri::command]
pub async fn bookshelf_restore_source_switch(
    state: tauri::State<'_, BookshelfState>,
    id: String,
) -> Result<SourceSwitchRestoreResult> {
    let book = bookshelf_get(state.clone(), id.clone()).await?;
    let chapters = bookshelf_get_chapters(state, id).await?;
    Ok(SourceSwitchRestoreResult { book, chapters })
}
