use legado_tauri_backend::booksource::legado_json;
use legado_tauri_backend::booksource::rules::{apply_rule_nodes, parse_document, ContextNode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let full_toc = args.iter().any(|arg| arg == "--full-toc");
    let positional = args
        .iter()
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>();
    let path = positional
        .first()
        .cloned()
        .unwrap_or_else(|| "elang.json".to_string());
    let keyword = positional
        .get(1)
        .cloned()
        .unwrap_or_else(|| "剑来".to_string());
    let text = tokio::fs::read_to_string(&path).await?;
    let source = legado_json::parse_source_text(&text)?;

    println!(
        "parserType=legado-json-css source={} url={}",
        source.book_source_name, source.book_source_url
    );

    let results = legado_json::search(&source, &keyword, 1).await?;
    println!("search keyword={keyword} count={}", results.len());
    let Some(first) = results.first() else {
        return Err("search returned no results".into());
    };
    println!(
        "firstBook name={} author={} url={}",
        first.name, first.author, first.book_url
    );

    let detail = legado_json::book_info(&source, &first.book_url).await?;
    println!(
        "detail name={} author={} tocUrl={}",
        detail.name,
        detail.author,
        detail.toc_url.as_deref().unwrap_or("")
    );

    let toc_url = detail.toc_url.as_deref().unwrap_or(&detail.book_url);
    let chapters = if full_toc {
        legado_json::chapter_list(&source, toc_url).await?
    } else {
        let html =
            legado_tauri_backend::http::fetch_text(toc_url, Some(&source.book_source_url)).await?;
        let doc = parse_document(&html);
        let ctx = ContextNode::Document(&doc);
        apply_rule_nodes(&ctx, &source.rule_toc.chapter_list, toc_url)?
            .into_iter()
            .filter_map(|node| {
                let name = legado_tauri_backend::booksource::rules::apply_rule_first(
                    &node,
                    &source.rule_toc.chapter_name,
                    toc_url,
                )
                .ok()?;
                let url = legado_tauri_backend::booksource::rules::apply_rule_first(
                    &node,
                    &source.rule_toc.chapter_url,
                    toc_url,
                )
                .ok()?;
                (!name.is_empty() && !url.is_empty()).then(|| {
                    let url = legado_tauri_backend::booksource::rules::resolve_url(toc_url, &url);
                    legado_tauri_backend::booksource::types::ChapterItem {
                        name,
                        url,
                        group: None,
                        vip: None,
                        price: None,
                        currency: None,
                    }
                })
            })
            .collect()
    };
    println!(
        "chapters count={}{}",
        chapters.len(),
        if full_toc {
            " fullToc=true"
        } else {
            " quick=true"
        }
    );
    let Some(first_chapter) = chapters.first() else {
        return Err("chapter list returned no chapters".into());
    };
    println!(
        "firstChapter name={} url={}",
        first_chapter.name, first_chapter.url
    );

    let content = legado_json::chapter_content(&source, &first_chapter.url).await?;
    println!(
        "content chars={} preview={}",
        content.chars().count(),
        content
            .chars()
            .take(80)
            .collect::<String>()
            .replace('\n', " ")
    );
    if content.trim().is_empty() {
        return Err("chapter content returned empty text".into());
    }
    Ok(())
}
