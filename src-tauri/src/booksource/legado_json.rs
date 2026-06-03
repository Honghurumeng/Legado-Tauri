use super::rules::{apply_rule_first, apply_rule_nodes, parse_document, resolve_url, ContextNode};
use super::types::{BookDetail, BookItem, ChapterItem};
use crate::errors::{BackendError, Result};
use crate::http;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegadoBookSource {
    #[serde(default)]
    pub book_source_group: String,
    #[serde(default)]
    pub book_source_name: String,
    #[serde(default)]
    pub book_source_type: Value,
    #[serde(default)]
    pub book_source_url: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub enabled_explore: Option<bool>,
    #[serde(default)]
    pub rule_search: LegadoSearchRule,
    #[serde(default)]
    pub rule_book_info: LegadoBookInfoRule,
    #[serde(default)]
    pub rule_toc: LegadoTocRule,
    #[serde(default)]
    pub rule_content: LegadoContentRule,
    #[serde(default)]
    pub rule_explore: Value,
    #[serde(default)]
    pub search_url: String,
    #[serde(default)]
    pub weight: Option<i64>,
    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegadoSearchRule {
    #[serde(default)]
    pub book_list: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub book_url: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub last_chapter: String,
    #[serde(default)]
    pub word_count: String,
    #[serde(default)]
    pub update_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegadoBookInfoRule {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub last_chapter: String,
    #[serde(default)]
    pub toc_url: String,
    #[serde(default)]
    pub word_count: String,
    #[serde(default)]
    pub update_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegadoTocRule {
    #[serde(default)]
    pub chapter_list: String,
    #[serde(default)]
    pub chapter_name: String,
    #[serde(default)]
    pub chapter_url: String,
    #[serde(default)]
    pub next_toc_url: String,
    #[serde(default)]
    pub vip: String,
    #[serde(default)]
    pub is_vip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LegadoContentRule {
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub next_content_url: String,
}

pub fn parse_source_text(text: &str) -> Result<LegadoBookSource> {
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with('[') {
        let mut list: Vec<LegadoBookSource> = serde_json::from_str(trimmed)?;
        list.pop()
            .ok_or_else(|| BackendError::msg("开源阅读 JSON 数组为空"))
    } else {
        Ok(serde_json::from_str(trimmed)?)
    }
}

pub fn parse_source_list(text: &str) -> Result<Vec<LegadoBookSource>> {
    let trimmed = text.trim_start_matches('\u{feff}').trim();
    if trimmed.starts_with('[') {
        Ok(serde_json::from_str(trimmed)?)
    } else {
        Ok(vec![serde_json::from_str(trimmed)?])
    }
}

pub async fn search(source: &LegadoBookSource, keyword: &str, page: u32) -> Result<Vec<BookItem>> {
    let url = build_search_url(source, keyword, page)?;
    // 书源搜索页可能先跳到新域名，后面的 bookUrl/coverUrl 都要基于最终地址解析。
    let (final_url, html) = http::fetch_text_with_final_url(&url, Some(&source.book_source_url)).await?;
    let doc = parse_document(&html);
    let ctx = ContextNode::Document(&doc);
    let nodes = apply_rule_nodes(&ctx, &source.rule_search.book_list, &final_url)?;
    let mut items = Vec::new();
    for node in nodes {
        let name = first(&node, &source.rule_search.name, &final_url)?;
        let book_url = first(&node, &source.rule_search.book_url, &final_url)?;
        if name.is_empty() || book_url.is_empty() {
            continue;
        }
        items.push(BookItem {
            name,
            author: first(&node, &source.rule_search.author, &final_url)?,
            book_url: resolve_url(&final_url, &book_url),
            cover_url: opt(first(&node, &source.rule_search.cover_url, &final_url)?)
                .map(|value| resolve_url(&final_url, &value)),
            last_chapter: opt(first(&node, &source.rule_search.last_chapter, &final_url)?),
            latest_chapter: None,
            latest_chapter_url: None,
            word_count: opt(first(&node, &source.rule_search.word_count, &final_url)?),
            chapter_count: None,
            update_time: opt(first(&node, &source.rule_search.update_time, &final_url)?),
            status: None,
            kind: opt(first(&node, &source.rule_search.kind, &final_url)?),
            intro: opt(first(&node, &source.rule_search.intro, &final_url)?),
        });
    }
    Ok(items)
}

pub async fn book_info(source: &LegadoBookSource, book_url: &str) -> Result<BookDetail> {
    let url = resolve_url(&source.book_source_url, book_url);
    // 这里要把最终详情页 URL 回填给前端。
    // 真实案例里 m.elkoparts.com 详情页会 301 到 m.elkoparts.net；
    // 如果仍然把旧 bookUrl 继续传给目录、书架和阅读器，后续请求会再次命中失效域名。
    let (final_url, html) = http::fetch_text_with_final_url(&url, Some(&source.book_source_url)).await?;
    let doc = parse_document(&html);
    let ctx = ContextNode::Document(&doc);
    let toc_url = first(&ctx, &source.rule_book_info.toc_url, &final_url)?;
    let name = first(&ctx, &source.rule_book_info.name, &final_url)?;
    Ok(BookDetail {
        name,
        author: first(&ctx, &source.rule_book_info.author, &final_url)?,
        book_url: final_url.clone(),
        cover_url: opt(first(&ctx, &source.rule_book_info.cover_url, &final_url)?)
            .map(|value| resolve_url(&final_url, &value)),
        intro: opt(first(&ctx, &source.rule_book_info.intro, &final_url)?),
        kind: opt(first(&ctx, &source.rule_book_info.kind, &final_url)?),
        last_chapter: opt(first(&ctx, &source.rule_book_info.last_chapter, &final_url)?),
        latest_chapter: None,
        latest_chapter_url: None,
        word_count: opt(first(&ctx, &source.rule_book_info.word_count, &final_url)?),
        chapter_count: None,
        update_time: opt(first(&ctx, &source.rule_book_info.update_time, &final_url)?),
        status: None,
        toc_url: Some(if toc_url.is_empty() {
            final_url
        } else {
            resolve_url(&final_url, &toc_url)
        }),
    })
}

pub async fn chapter_list(source: &LegadoBookSource, book_url: &str) -> Result<Vec<ChapterItem>> {
    let mut url = resolve_url(&source.book_source_url, book_url);
    let mut chapters = Vec::new();
    let mut seen_pages = std::collections::HashSet::new();

    loop {
        if !seen_pages.insert(url.clone()) {
            break;
        }
        // next_toc_url 也必须以当前页最终落地地址为基准，避免翻页时跳回旧域名。
        let (final_url, html) =
            http::fetch_text_with_final_url(&url, Some(&source.book_source_url)).await?;
        let doc = parse_document(&html);
        let ctx = ContextNode::Document(&doc);
        let nodes = apply_rule_nodes(&ctx, &source.rule_toc.chapter_list, &final_url)?;
        for node in nodes {
            let name = first(&node, &source.rule_toc.chapter_name, &final_url)?;
            let chapter_url = first(&node, &source.rule_toc.chapter_url, &final_url)?;
            if name.is_empty() || chapter_url.is_empty() {
                continue;
            }
            let vip = if !source.rule_toc.vip.is_empty() {
                Some(!first(&node, &source.rule_toc.vip, &final_url)?.is_empty())
            } else if !source.rule_toc.is_vip.is_empty() {
                Some(!first(&node, &source.rule_toc.is_vip, &final_url)?.is_empty())
            } else {
                None
            };
            chapters.push(ChapterItem {
                name,
                url: resolve_url(&final_url, &chapter_url),
                group: None,
                vip,
                price: None,
                currency: None,
            });
        }
        let next = first(&ctx, &source.rule_toc.next_toc_url, &final_url)?;
        if next.is_empty() {
            break;
        }
        let next_url = resolve_url(&final_url, &next);
        if next_url == final_url {
            break;
        }
        url = next_url;
    }

    Ok(chapters)
}

pub async fn chapter_content(source: &LegadoBookSource, chapter_url: &str) -> Result<String> {
    let mut url = resolve_url(&source.book_source_url, chapter_url);
    let mut pieces = Vec::new();
    let mut seen_pages = std::collections::HashSet::new();

    loop {
        if !seen_pages.insert(url.clone()) {
            break;
        }
        // 正文分页和“下一页”链接同样依赖最终 URL，和目录页的换域问题一致。
        let (final_url, html) =
            http::fetch_text_with_final_url(&url, Some(&source.book_source_url)).await?;
        let doc = parse_document(&html);
        let ctx = ContextNode::Document(&doc);
        let content = first(&ctx, &source.rule_content.content, &final_url)?;
        if !content.is_empty() {
            pieces.push(clean_content_html(&content));
        }
        let next = first(&ctx, &source.rule_content.next_content_url, &final_url)?;
        if next.is_empty() {
            break;
        }
        let next_url = resolve_url(&final_url, &next);
        if next_url == final_url || !is_same_chapter_page(&final_url, &next_url) {
            break;
        }
        url = next_url;
    }

    Ok(pieces.join("\n\n").trim().to_string())
}

pub async fn explore(_source: &LegadoBookSource, category: &str, _page: u32) -> Result<Value> {
    if category == "GETALL" {
        Ok(Value::Array(Vec::new()))
    } else {
        Ok(Value::Array(Vec::new()))
    }
}

fn build_search_url(source: &LegadoBookSource, keyword: &str, page: u32) -> Result<String> {
    if source.search_url.trim().is_empty() {
        return Err(BackendError::msg("书源缺少 searchUrl"));
    }
    let encoded = utf8_percent_encode(keyword, NON_ALPHANUMERIC).to_string();
    let mut path = source
        .search_url
        .replace("{{key}}", &encoded)
        .replace("{{keyword}}", &encoded)
        .replace("{{page}}", &page.to_string());
    if !path.contains("{{") && !path.starts_with("http://") && !path.starts_with("https://") {
        let base = Url::parse(&source.book_source_url)?;
        path = base.join(&path)?.to_string();
    }
    Ok(path)
}

fn first(context: &ContextNode<'_>, rule: &str, base_url: &str) -> Result<String> {
    if rule.trim().is_empty() {
        return Ok(String::new());
    }
    apply_rule_first(context, rule, base_url)
}

fn opt(value: String) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}

fn clean_content_html(value: &str) -> String {
    let with_breaks = value
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    let fragment = scraper::Html::parse_fragment(&with_breaks);
    let text = fragment.root_element().text().collect::<Vec<_>>().join("");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_same_chapter_page(current: &str, next: &str) -> bool {
    let (Ok(current), Ok(next)) = (Url::parse(current), Url::parse(next)) else {
        return false;
    };
    if current.host_str() != next.host_str() {
        return false;
    }
    let current_path = current.path();
    let next_path = next.path();
    if next_path.ends_with("/index.html") || next_path.ends_with('/') {
        return false;
    }
    let Some(root) = current_path
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".html").or(Some(file)))
        .map(|stem| stem.split('_').next().unwrap_or(stem))
        .filter(|stem| !stem.is_empty())
    else {
        return false;
    };
    next_path != current_path && next_path.contains(root)
}
