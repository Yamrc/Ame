use crate::crypto::{eapi, weapi};
use crate::api::request::ApiRequest;
use reqwest::Client;
use std::time::Duration;

const EAPI_BASE: &str = "https://interface.music.163.com/eapi";
const WEAPI_BASE: &str = "https://music.163.com/weapi";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/3.1.28.205001";
const REFERER: &str = "https://music.163.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

pub struct NeteaseClient {
    client: Client,
    cookie: String,
}

impl NeteaseClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cookie: String::new(),
        }
    }

    pub fn with_cookie(cookie: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            cookie: cookie.into(),
        }
    }

    pub async fn eapi_request<R: ApiRequest>(&self, req: R) -> Result<R::Response, Error> {
        let endpoint = req.endpoint();
        let route = strip_api_prefix(endpoint);
        let params = req.payload();
        let url = format!("{}{}", EAPI_BASE, route);
        let encrypted = eapi::encrypt(&eapi_encrypt_path(endpoint), &params.to_string());

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
            .form(&[("params", encrypted)])
            .send()
            .await?;

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

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            return Err(Error::Http(status, text));
        }

        Ok(serde_json::from_str(&text)?)
    }
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error {0}: {1}")]
    Http(reqwest::StatusCode, String),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
