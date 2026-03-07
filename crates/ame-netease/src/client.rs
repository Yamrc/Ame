use crate::api::request::ApiRequest;
use crate::crypto::{eapi, weapi};
use reqwest::{Client, header::SET_COOKIE};
use serde_json::Value;
use std::sync::Mutex;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

const EAPI_BASE: &str = "https://interface.music.163.com/eapi";
const WEAPI_BASE: &str = "https://music.163.com/weapi";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/3.1.28.205001";
const EAPI_USER_AGENT: &str = "NeteaseMusic 9.0.90/5038 (iPhone; iOS 16.2; zh_CN)";
const REFERER: &str = "https://music.163.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub struct NeteaseClient {
    client: Client,
    cookie: String,
    last_set_cookie: Mutex<Vec<String>>,
}

impl Default for NeteaseClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NeteaseClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cookie: String::new(),
            last_set_cookie: Mutex::new(Vec::new()),
        }
    }

    pub fn with_cookie(cookie: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            cookie: cookie.into(),
            last_set_cookie: Mutex::new(Vec::new()),
        }
    }

    pub fn take_last_set_cookie(&self) -> Vec<String> {
        match self.last_set_cookie.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(_) => Vec::new(),
        }
    }

    pub async fn eapi_request<R: ApiRequest>(&self, req: R) -> Result<R::Response, Error> {
        let endpoint = req.endpoint();
        let route = strip_api_prefix(endpoint);
        let mut params = req.payload();
        let url = format!("{}{}", EAPI_BASE, route);
        let cookie_pairs = normalize_cookie_pairs(&self.cookie);
        let eapi_header = build_eapi_header(&cookie_pairs);
        attach_eapi_header(&mut params, eapi_header.clone());
        let cookie_header = cookie_string_from_pairs(&eapi_header);
        let encrypted = eapi::encrypt(&eapi_encrypt_path(endpoint), &params.to_string());

        let resp: reqwest::Response = self
            .client
            .post(&url)
            .timeout(REQUEST_TIMEOUT)
            .header("User-Agent", EAPI_USER_AGENT)
            .header("Referer", REFERER)
            .header("Origin", REFERER)
            .header("Accept", "*/*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Cookie", cookie_header)
            .form(&[("params", encrypted)])
            .send()
            .await?;
        self.replace_last_set_cookie(extract_set_cookie_values(&resp));

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(Error::Http(status, text));
        }

        match eapi::decrypt(&text) {
            Ok(Some(decrypted)) => return Ok(serde_json::from_str(&decrypted)?),
            Ok(None) => {}
            Err(_) => {}
        }

        Ok(serde_json::from_str(&text)?)
    }

    pub async fn weapi_request<R: ApiRequest>(&self, req: R) -> Result<R::Response, Error> {
        let endpoint = req.endpoint();
        let route = strip_api_prefix(endpoint);
        let params = req.payload();
        let url = format!("{}{}", WEAPI_BASE, route);
        let payload = weapi::encrypt(&params.to_string());

        let resp: reqwest::Response = self
            .client
            .post(&url)
            .timeout(REQUEST_TIMEOUT)
            .header("User-Agent", USER_AGENT)
            .header("Referer", REFERER)
            .header("Origin", REFERER)
            .header("Accept", "*/*")
            .header("Accept-Language", "zh-CN,zh;q=0.9")
            .header("Cookie", &self.cookie)
            .form(&[
                ("params", payload.params),
                ("encSecKey", payload.enc_sec_key),
            ])
            .send()
            .await?;
        self.replace_last_set_cookie(extract_set_cookie_values(&resp));

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(Error::Http(status, text));
        }

        Ok(serde_json::from_str(&text)?)
    }

    fn replace_last_set_cookie(&self, cookies: Vec<String>) {
        if let Ok(mut guard) = self.last_set_cookie.lock() {
            *guard = cookies;
        }
    }
}

fn extract_set_cookie_values(resp: &reqwest::Response) -> Vec<String> {
    resp.headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .map(ToString::to_string)
        .collect()
}

fn strip_api_prefix(endpoint: &str) -> &str {
    endpoint.strip_prefix("/api").unwrap_or(endpoint)
}

fn eapi_encrypt_path(endpoint: &str) -> String {
    if endpoint.starts_with("/api/") {
        endpoint.to_string()
    } else {
        format!("/api{}", endpoint)
    }
}

fn normalize_cookie_pairs(raw_cookie: &str) -> Vec<(String, String)> {
    let mut pairs = parse_cookie_pairs(raw_cookie);
    upsert_cookie(&mut pairs, "os", "pc");
    upsert_cookie(&mut pairs, "appver", "3.1.28.205001");
    upsert_cookie(
        &mut pairs,
        "osver",
        "Microsoft-Windows-10-Professional-build-19045-64bit",
    );
    upsert_cookie(&mut pairs, "channel", "netease");
    upsert_cookie(&mut pairs, "WEVNSM", "1.0.0");
    pairs
}

fn parse_cookie_pairs(raw_cookie: &str) -> Vec<(String, String)> {
    raw_cookie
        .split(';')
        .filter_map(|part| {
            let trimmed = part.trim();
            let (k, v) = trimmed.split_once('=')?;
            let key = k.trim();
            let value = v.trim();
            if key.is_empty() || value.is_empty() {
                return None;
            }
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn upsert_cookie(pairs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if pairs
        .iter()
        .any(|(existing_key, existing_value)| existing_key == key && !existing_value.is_empty())
    {
        return;
    }
    pairs.push((key.to_string(), value.to_string()));
}

fn cookie_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find_map(|(existing_key, value)| (existing_key == key).then_some(value.as_str()))
}

fn cookie_string_from_pairs(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn attach_eapi_header(params: &mut Value, eapi_header: Vec<(String, String)>) {
    let Some(object) = params.as_object_mut() else {
        return;
    };
    let mut header = serde_json::Map::new();
    for (k, v) in eapi_header {
        header.insert(k, Value::String(v));
    }
    object.insert("header".to_string(), Value::Object(header));
}

fn build_eapi_header(cookie_pairs: &[(String, String)]) -> Vec<(String, String)> {
    let timestamp_ms = now_millis();
    let buildver = (timestamp_ms / 1000).to_string();
    let request_id = format!("{timestamp_ms}_0001");
    let mut header = vec![
        (
            "osver".to_string(),
            cookie_value(cookie_pairs, "osver")
                .unwrap_or_default()
                .to_string(),
        ),
        (
            "deviceId".to_string(),
            cookie_value(cookie_pairs, "deviceId")
                .unwrap_or_default()
                .to_string(),
        ),
        (
            "os".to_string(),
            cookie_value(cookie_pairs, "os").unwrap_or("pc").to_string(),
        ),
        (
            "appver".to_string(),
            cookie_value(cookie_pairs, "appver")
                .unwrap_or("3.1.28.205001")
                .to_string(),
        ),
        (
            "versioncode".to_string(),
            cookie_value(cookie_pairs, "versioncode")
                .unwrap_or("140")
                .to_string(),
        ),
        (
            "mobilename".to_string(),
            cookie_value(cookie_pairs, "mobilename")
                .unwrap_or_default()
                .to_string(),
        ),
        (
            "buildver".to_string(),
            cookie_value(cookie_pairs, "buildver")
                .map(ToString::to_string)
                .unwrap_or(buildver),
        ),
        (
            "resolution".to_string(),
            cookie_value(cookie_pairs, "resolution")
                .unwrap_or("1920x1080")
                .to_string(),
        ),
        (
            "__csrf".to_string(),
            cookie_value(cookie_pairs, "__csrf")
                .unwrap_or_default()
                .to_string(),
        ),
        (
            "channel".to_string(),
            cookie_value(cookie_pairs, "channel")
                .unwrap_or("netease")
                .to_string(),
        ),
        ("requestId".to_string(), request_id),
    ];
    if let Some(music_u) = cookie_value(cookie_pairs, "MUSIC_U") {
        header.push(("MUSIC_U".to_string(), music_u.to_string()));
    }
    if let Some(music_a) = cookie_value(cookie_pairs, "MUSIC_A") {
        header.push(("MUSIC_A".to_string(), music_a.to_string()));
    }
    header
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error {0}: {1}")]
    Http(reqwest::StatusCode, String),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::{build_eapi_header, normalize_cookie_pairs};

    #[test]
    fn normalize_cookie_adds_required_defaults() {
        let pairs = normalize_cookie_pairs("MUSIC_A=guest_token");
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "MUSIC_A" && v == "guest_token")
        );
        assert!(pairs.iter().any(|(k, v)| k == "os" && v == "pc"));
        assert!(pairs.iter().any(|(k, v)| k == "appver" && !v.is_empty()));
        assert!(pairs.iter().any(|(k, v)| k == "channel" && v == "netease"));
    }

    #[test]
    fn eapi_header_keeps_auth_cookie() {
        let pairs = normalize_cookie_pairs("MUSIC_U=user_token; __csrf=csrf_token");
        let header = build_eapi_header(&pairs);
        assert!(
            header
                .iter()
                .any(|(k, v)| k == "MUSIC_U" && v == "user_token")
        );
        assert!(
            header
                .iter()
                .any(|(k, v)| k == "__csrf" && v == "csrf_token")
        );
    }
}
