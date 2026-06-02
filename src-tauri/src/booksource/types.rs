use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSourceMeta {
    pub source_key: String,
    pub uuid: String,
    pub file_name: String,
    pub name: String,
    pub url: String,
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub enabled: bool,
    pub file_size: u64,
    pub modified_at: i64,
    pub source_dir: String,
    pub source_type: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_url: Option<String>,
    pub tags: Vec<String>,
    pub min_delay_ms: u64,
    pub require_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_explore: Option<bool>,
    /// 解析器类型由后端在加载书源时先判定，再分派到对应解析器。
    ///
    /// 维护提醒：新增书源格式时必须同步更新
    /// `booksource/parser_registry.rs` 的 `ParserType` 和分派逻辑，
    /// 不要在命令函数里直接判断字段临时解析。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookItem {
    pub name: String,
    pub author: String,
    pub book_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_chapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_chapter_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetail {
    pub name: String,
    pub author: String,
    pub book_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intro: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_chapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_chapter_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_count: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterItem {
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
#[serde(rename_all = "camelCase")]
pub struct LegacyJsonImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub files: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSourceDeleteItem {
    pub file_name: String,
    #[serde(default)]
    pub source_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSourceDeleteError {
    pub file_name: String,
    #[serde(default)]
    pub source_dir: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSourceBatchDeleteResult {
    pub deleted: Vec<BookSourceDeleteItem>,
    pub errors: Vec<BookSourceDeleteError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStepResult {
    pub step: String,
    pub passed: bool,
    pub message: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRunResult {
    pub file_name: String,
    pub steps: Vec<TestStepResult>,
    pub all_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub file_name: String,
    pub uuid: String,
    pub has_update: bool,
    pub local_version: String,
    pub remote_version: String,
}
