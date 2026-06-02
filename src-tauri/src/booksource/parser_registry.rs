//! 书源解析器注册表。
//!
//! 维护提醒：
//! 1. 所有书源运行命令必须先调用 `load_source_file` 并读取 `parser_type`。
//! 2. 新增解析格式时，在 `ParserType` 中增加枚举值，并在本文件的分派函数里接入。
//! 3. 不要在 `commands.rs` 里用字段名临时判断解析类型，否则后续测试会很难按格式隔离。

use super::legado_json::{self, LegadoBookSource};
use super::types::{BookDetail, BookItem, ChapterItem};
use crate::errors::{BackendError, Result};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserType {
    TauriJs,
    LegadoJsonCss,
}

impl ParserType {
    pub fn as_str(self) -> &'static str {
        match self {
            ParserType::TauriJs => "tauri-js",
            ParserType::LegadoJsonCss => "legado-json-css",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SourceData {
    TauriJs(String),
    LegadoJson(LegadoBookSource),
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub file_name: String,
    pub path: PathBuf,
    pub source_dir: PathBuf,
    pub parser_type: ParserType,
    pub data: SourceData,
}

pub fn detect_parser_type(file_name: &str, content: &str) -> ParserType {
    let trimmed = content.trim_start_matches('\u{feff}').trim_start();
    if file_name.to_ascii_lowercase().ends_with(".json")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
    {
        ParserType::LegadoJsonCss
    } else {
        ParserType::TauriJs
    }
}

pub async fn load_source_file(path: PathBuf, source_dir: PathBuf) -> Result<SourceFile> {
    let content = tokio::fs::read_to_string(&path).await?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let parser_type = detect_parser_type(&file_name, &content);
    let data = match parser_type {
        ParserType::LegadoJsonCss => {
            SourceData::LegadoJson(legado_json::parse_source_text(&content)?)
        }
        ParserType::TauriJs => SourceData::TauriJs(content.clone()),
    };
    Ok(SourceFile {
        file_name,
        path,
        source_dir,
        parser_type,
        data,
    })
}

pub async fn search(source: &SourceFile, keyword: &str, page: u32) -> Result<Vec<BookItem>> {
    match &source.data {
        SourceData::LegadoJson(source) => legado_json::search(source, keyword, page).await,
        SourceData::TauriJs(_) => Err(BackendError::msg(
            "JS 书源运行时尚未接入；当前已支持 Legado JSON CSS 规则书源",
        )),
    }
}

pub async fn book_info(source: &SourceFile, book_url: &str) -> Result<BookDetail> {
    match &source.data {
        SourceData::LegadoJson(source) => legado_json::book_info(source, book_url).await,
        SourceData::TauriJs(_) => Err(BackendError::msg(
            "JS 书源运行时尚未接入；当前已支持 Legado JSON CSS 规则书源",
        )),
    }
}

pub async fn chapter_list(source: &SourceFile, book_url: &str) -> Result<Vec<ChapterItem>> {
    match &source.data {
        SourceData::LegadoJson(source) => legado_json::chapter_list(source, book_url).await,
        SourceData::TauriJs(_) => Err(BackendError::msg(
            "JS 书源运行时尚未接入；当前已支持 Legado JSON CSS 规则书源",
        )),
    }
}

pub async fn chapter_content(source: &SourceFile, chapter_url: &str) -> Result<String> {
    match &source.data {
        SourceData::LegadoJson(source) => legado_json::chapter_content(source, chapter_url).await,
        SourceData::TauriJs(_) => Err(BackendError::msg(
            "JS 书源运行时尚未接入；当前已支持 Legado JSON CSS 规则书源",
        )),
    }
}

pub async fn explore(source: &SourceFile, category: &str, page: u32) -> Result<Value> {
    match &source.data {
        SourceData::LegadoJson(source) => legado_json::explore(source, category, page).await,
        SourceData::TauriJs(_) => Err(BackendError::msg(
            "JS 书源运行时尚未接入；当前已支持 Legado JSON CSS 规则书源",
        )),
    }
}

pub fn capabilities(source: &SourceFile) -> Vec<&'static str> {
    match source.parser_type {
        ParserType::LegadoJsonCss => vec!["search", "bookInfo", "toc", "content"],
        ParserType::TauriJs => vec![],
    }
}
