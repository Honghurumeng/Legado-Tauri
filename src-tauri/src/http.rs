use crate::errors::{BackendError, Result};
use encoding_rs::{Encoding, UTF_8};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const BUILTIN_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProxyResponse {
    pub status: u16,
    pub url: String,
    pub headers: serde_json::Value,
    pub body: String,
}

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent(BUILTIN_USER_AGENT)
        .timeout(Duration::from_secs(35))
        .connect_timeout(Duration::from_secs(12))
        .danger_accept_invalid_certs(true)
        .redirect(reqwest::redirect::Policy::limited(10))
        .cookie_store(true)
        .build()?)
}

pub fn build_headers(
    headers: Option<serde_json::Value>,
    referer: Option<&str>,
) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    map.insert(USER_AGENT, HeaderValue::from_static(BUILTIN_USER_AGENT));
    if let Some(value) = referer.filter(|value| !value.trim().is_empty()) {
        if let Ok(header) = HeaderValue::from_str(value) {
            map.insert(reqwest::header::REFERER, header);
        }
    }

    let Some(serde_json::Value::Object(obj)) = headers else {
        return Ok(map);
    };
    for (key, value) in obj {
        let Some(value) = value.as_str() else {
            continue;
        };
        let name = HeaderName::from_bytes(key.as_bytes())
            .map_err(|_| BackendError::msg(format!("非法请求头名称: {key}")))?;
        let header_value = HeaderValue::from_str(value)
            .map_err(|_| BackendError::msg(format!("非法请求头值: {key}")))?;
        map.insert(name, header_value);
    }
    Ok(map)
}

pub async fn fetch_text(url: &str, referer: Option<&str>) -> Result<String> {
    let resp = client()?
        .get(url)
        .headers(build_headers(None, referer)?)
        .send()
        .await?
        .error_for_status()?;
    decode_response_text(resp).await
}

pub async fn request_text(
    method: &str,
    url: &str,
    body: Option<String>,
    headers: Option<serde_json::Value>,
    referer: Option<&str>,
) -> Result<HttpProxyResponse> {
    let method = Method::from_bytes(method.as_bytes()).unwrap_or(Method::GET);
    let mut req = client()?
        .request(method, url)
        .headers(build_headers(headers, referer)?);
    if let Some(body) = body {
        req = req.body(body);
    }
    let resp = req.send().await?;
    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();
    let headers_json = serde_json::Value::Object(
        resp.headers()
            .iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    serde_json::Value::String(value.to_str().unwrap_or_default().to_string()),
                )
            })
            .collect(),
    );
    let body = decode_response_text(resp).await?;
    Ok(HttpProxyResponse {
        status,
        url: final_url,
        headers: headers_json,
        body,
    })
}

pub async fn decode_response_text(resp: reqwest::Response) -> Result<String> {
    let charset = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_charset)
        .unwrap_or("utf-8");
    let encoding = Encoding::for_label(charset.as_bytes()).unwrap_or(UTF_8);
    let bytes = resp.bytes().await?;
    let (cow, _, _) = encoding.decode(&bytes);
    Ok(cow.into_owned())
}

fn parse_charset(content_type: &str) -> Option<&str> {
    content_type
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("charset=").map(str::trim))
        .map(|value| value.trim_matches('"'))
}
