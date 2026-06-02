use crate::errors::{BackendError, Result};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use url::Url;

#[derive(Clone)]
pub enum ContextNode<'a> {
    Document(&'a Html),
    Element(ElementRef<'a>),
    Text(String),
}

#[derive(Clone, Copy)]
enum AttrKind {
    Text,
    Html,
    TextNodes,
    Attr,
}

#[derive(Clone)]
struct RulePart {
    selector: String,
    attr: AttrKind,
    attr_name: Option<String>,
    index: Option<IndexSpec>,
    exclude_indices: Vec<usize>,
}

#[derive(Clone)]
enum IndexSpec {
    Single(isize),
    Multi(Vec<isize>),
    Range(Option<isize>, Option<isize>),
}

pub fn resolve_url(base: &str, value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    if value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("data:")
        || value.starts_with("file:")
    {
        return value.to_string();
    }
    Url::parse(base)
        .and_then(|parsed| parsed.join(value))
        .map(|url| url.to_string())
        .unwrap_or_else(|_| value.to_string())
}

pub fn parse_document(html: &str) -> Html {
    Html::parse_document(html)
}

pub fn apply_rule_first(context: &ContextNode<'_>, rule: &str, base_url: &str) -> Result<String> {
    Ok(apply_rule(context, rule, base_url)?
        .into_iter()
        .next()
        .unwrap_or_default())
}

pub fn apply_rule(context: &ContextNode<'_>, rule: &str, base_url: &str) -> Result<Vec<String>> {
    let mut expr = rule.trim();
    if expr.is_empty() {
        return Ok(Vec::new());
    }

    let replacements = parse_replacements(expr)?;
    if let Some((head, _)) = expr.split_once("##") {
        expr = head.trim();
    }

    let parts = split_rule_parts(expr);
    let mut nodes = vec![context.clone()];
    for raw_part in parts {
        let part = parse_rule_part(&raw_part)?;
        nodes = eval_part(&nodes, &part)?;
    }

    let mut values = nodes
        .into_iter()
        .map(|node| match node {
            ContextNode::Text(text) => text,
            ContextNode::Element(element) => {
                normalize_space(&element.text().collect::<Vec<_>>().join(""))
            }
            ContextNode::Document(document) => {
                normalize_space(&document.root_element().text().collect::<Vec<_>>().join(""))
            }
        })
        .collect::<Vec<_>>();

    if matches!(last_attr_kind(expr), Some("href" | "src")) {
        values = values
            .into_iter()
            .map(|value| resolve_url(base_url, &value))
            .collect();
    }

    for (pattern, replacement) in replacements {
        let regex = Regex::new(&pattern)
            .map_err(|err| BackendError::msg(format!("规则正则错误 {pattern}: {err}")))?;
        values = values
            .into_iter()
            .map(|value| regex.replace_all(&value, replacement.as_str()).to_string())
            .collect();
    }

    Ok(values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect())
}

pub fn apply_rule_nodes<'a>(
    context: &'a ContextNode<'a>,
    rule: &str,
    _base_url: &str,
) -> Result<Vec<ContextNode<'a>>> {
    let expr = rule.split("##").next().unwrap_or(rule).trim();
    let parts = split_rule_parts(expr);
    let mut nodes = vec![context.clone()];
    for raw_part in parts {
        let part = parse_rule_part(&raw_part)?;
        nodes = eval_part(&nodes, &part)?;
    }
    Ok(nodes)
}

fn parse_replacements(expr: &str) -> Result<Vec<(String, String)>> {
    let mut parts = expr.split("##").skip(1);
    let mut replacements = Vec::new();
    while let Some(pattern) = parts.next() {
        let replacement = parts.next().unwrap_or("");
        replacements.push((pattern.to_string(), replacement.to_string()));
    }
    Ok(replacements)
}

fn split_rule_parts(expr: &str) -> Vec<String> {
    expr.split('@')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_rule_part(raw: &str) -> Result<RulePart> {
    if raw == "text" {
        return Ok(RulePart {
            selector: String::new(),
            attr: AttrKind::Text,
            attr_name: None,
            index: None,
            exclude_indices: Vec::new(),
        });
    }
    if raw == "html" {
        return Ok(RulePart {
            selector: String::new(),
            attr: AttrKind::Html,
            attr_name: None,
            index: None,
            exclude_indices: Vec::new(),
        });
    }
    if raw == "textNodes" {
        return Ok(RulePart {
            selector: String::new(),
            attr: AttrKind::TextNodes,
            attr_name: None,
            index: None,
            exclude_indices: Vec::new(),
        });
    }

    let mut selector = raw.to_string();
    let mut index = None;
    let mut exclude_indices = Vec::new();

    if let Some(pos) = selector.find('!') {
        let tail = selector[pos + 1..].to_string();
        selector.truncate(pos);
        for item in tail.split(',') {
            if let Ok(idx) = item.trim().parse::<usize>() {
                exclude_indices.push(idx);
            }
        }
    }

    if let Some(pos) = selector.rfind('.') {
        let maybe = selector[pos + 1..].to_string();
        if maybe.parse::<isize>().is_ok() {
            index = Some(IndexSpec::Single(maybe.parse().unwrap()));
            selector.truncate(pos);
        } else {
            let multi = maybe
                .split(':')
                .map(str::trim)
                .map(str::parse::<isize>)
                .collect::<std::result::Result<Vec<_>, _>>();
            if let Ok(indices) = multi {
                if !indices.is_empty() {
                    index = Some(IndexSpec::Multi(indices));
                    selector.truncate(pos);
                }
            }
        }
    }

    if index.is_none() && selector.find(':').is_some() {
        let pos = selector.find(':').unwrap();
        let range = selector[pos + 1..].to_string();
        if range
            .split(':')
            .all(|item| item.is_empty() || item.parse::<isize>().is_ok())
        {
            let mut it = range.split(':');
            let start = it.next().and_then(|value| value.parse::<isize>().ok());
            let end = it.next().and_then(|value| value.parse::<isize>().ok());
            index = Some(IndexSpec::Range(start, end));
            selector.truncate(pos);
        }
    }

    let (selector, attr, attr_name) = match selector.as_str() {
        "text" => (String::new(), AttrKind::Text, None),
        "html" => (String::new(), AttrKind::Html, None),
        "textNodes" => (String::new(), AttrKind::TextNodes, None),
        attr if is_simple_attr(attr) => (String::new(), AttrKind::Attr, Some(attr.to_string())),
        _ => (selector, AttrKind::Text, None),
    };

    Ok(RulePart {
        selector,
        attr,
        attr_name,
        index,
        exclude_indices,
    })
}

fn is_simple_attr(value: &str) -> bool {
    matches!(value, "href" | "src" | "alt" | "title" | "data-src")
}

fn normalize_selector(selector: &str) -> String {
    if let Some(id) = selector.strip_prefix("id.") {
        return format!("#{id}");
    }
    if let Some(class_name) = selector.strip_prefix("class.") {
        return format!(".{class_name}");
    }
    selector.to_string()
}

fn eval_part<'a>(nodes: &[ContextNode<'a>], part: &RulePart) -> Result<Vec<ContextNode<'a>>> {
    let mut out = Vec::new();
    for node in nodes {
        if part.selector.is_empty() {
            out.extend(extract_attr(node.clone(), part));
            continue;
        }
        if let Some(text_query) = part.selector.strip_prefix("text.") {
            out.extend(find_link_by_text(node, text_query, part));
            continue;
        }
        let css_selector = normalize_selector(&part.selector);
        let selector = Selector::parse(&css_selector)
            .map_err(|_| BackendError::msg(format!("CSS 规则无法解析: {}", part.selector)))?;
        let mut selected = match node {
            ContextNode::Document(document) => document.select(&selector).collect::<Vec<_>>(),
            ContextNode::Element(element) => element.select(&selector).collect::<Vec<_>>(),
            ContextNode::Text(_) => Vec::new(),
        };
        selected = apply_excludes(selected, &part.exclude_indices);
        selected = apply_index(selected, part.index.clone());
        if matches!(part.attr, AttrKind::Text) && part.attr_name.is_none() {
            out.extend(selected.into_iter().map(ContextNode::Element));
        } else {
            out.extend(
                selected
                    .into_iter()
                    .flat_map(|element| extract_attr(ContextNode::Element(element), part)),
            );
        }
    }
    Ok(out)
}

fn find_link_by_text<'a>(
    node: &ContextNode<'a>,
    query: &str,
    part: &RulePart,
) -> Vec<ContextNode<'a>> {
    let Ok(selector) = Selector::parse("a") else {
        return Vec::new();
    };
    let selected = match node {
        ContextNode::Document(document) => document.select(&selector).collect::<Vec<_>>(),
        ContextNode::Element(element) => element.select(&selector).collect::<Vec<_>>(),
        ContextNode::Text(_) => Vec::new(),
    };
    selected
        .into_iter()
        .filter(|element| {
            normalize_space(&element.text().collect::<Vec<_>>().join("")).contains(query)
        })
        .flat_map(|element| {
            if matches!(part.attr, AttrKind::Text) && part.attr_name.is_none() {
                vec![ContextNode::Element(element)]
            } else {
                extract_attr(ContextNode::Element(element), part)
            }
        })
        .collect()
}

fn extract_attr<'a>(node: ContextNode<'a>, part: &RulePart) -> Vec<ContextNode<'a>> {
    match (node, part.attr) {
        (ContextNode::Text(text), _) => vec![ContextNode::Text(text)],
        (ContextNode::Document(document), AttrKind::Text) => vec![ContextNode::Text(
            normalize_space(&document.root_element().text().collect::<Vec<_>>().join("")),
        )],
        (ContextNode::Document(document), AttrKind::Html) => {
            vec![ContextNode::Text(document.root_element().html())]
        }
        (ContextNode::Document(document), AttrKind::TextNodes) => vec![ContextNode::Text(
            normalize_space(&document.root_element().text().collect::<Vec<_>>().join("")),
        )],
        (ContextNode::Document(_), AttrKind::Attr) => Vec::new(),
        (ContextNode::Element(element), AttrKind::Text) => vec![ContextNode::Text(
            normalize_space(&element.text().collect::<Vec<_>>().join("")),
        )],
        (ContextNode::Element(element), AttrKind::Html) => {
            vec![ContextNode::Text(element.inner_html())]
        }
        (ContextNode::Element(element), AttrKind::TextNodes) => {
            vec![ContextNode::Text(normalize_space(
                &element.text().collect::<Vec<_>>().join(""),
            ))]
        }
        (ContextNode::Element(element), AttrKind::Attr) => part
            .attr_name
            .as_deref()
            .and_then(|name| element.value().attr(name))
            .map(|value| vec![ContextNode::Text(value.to_string())])
            .unwrap_or_default(),
    }
}

fn apply_excludes<'a>(items: Vec<ElementRef<'a>>, excludes: &[usize]) -> Vec<ElementRef<'a>> {
    if excludes.is_empty() {
        return items;
    }
    items
        .into_iter()
        .enumerate()
        .filter_map(|(idx, item)| (!excludes.contains(&idx)).then_some(item))
        .collect()
}

fn apply_index<'a>(items: Vec<ElementRef<'a>>, index: Option<IndexSpec>) -> Vec<ElementRef<'a>> {
    let Some(index) = index else {
        return items;
    };
    match index {
        IndexSpec::Single(idx) => {
            let len = items.len() as isize;
            let idx = if idx < 0 { len + idx } else { idx };
            if idx < 0 || idx >= len {
                Vec::new()
            } else {
                items.into_iter().nth(idx as usize).into_iter().collect()
            }
        }
        IndexSpec::Multi(indices) => {
            let len = items.len() as isize;
            let wanted = indices
                .into_iter()
                .map(|idx| if idx < 0 { len + idx } else { idx })
                .filter(|idx| *idx >= 0 && *idx < len)
                .collect::<Vec<_>>();
            items
                .into_iter()
                .enumerate()
                .filter_map(|(idx, item)| wanted.contains(&(idx as isize)).then_some(item))
                .collect()
        }
        IndexSpec::Range(start, end) => {
            let len = items.len() as isize;
            let start = start.unwrap_or(0);
            let end = end.unwrap_or(len);
            let start = if start < 0 { len + start } else { start }.clamp(0, len);
            let end = if end < 0 { len + end } else { end }.clamp(start, len);
            items
                .into_iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let idx = idx as isize;
                    (idx >= start && idx < end).then_some(item)
                })
                .collect()
        }
    }
}

pub fn normalize_space(value: &str) -> String {
    value
        .replace('\u{00a0}', " ")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn last_attr_kind(expr: &str) -> Option<&str> {
    expr.split('@').last().map(str::trim)
}
