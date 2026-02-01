use crate::crypto::{eapi, weapi};
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Value;

const EAPI_BASE: &str = "https://interface.music.163.com/eapi";
const WEAPI_BASE: &str = "https://music.163.com/weapi";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/3.0.18.203152";
const REFERER: &str = "https://music.163.com";

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

    pub async fn eapi_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: Value,
    ) -> Result<T, Error> {
        let url = format!("{}{}", EAPI_BASE, endpoint);
        let encrypted = eapi::encrypt(endpoint, &params.to_string());

        let resp: reqwest::Response = self
            .client
            .post(&url)
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

        if let Some(decrypted) = eapi::decrypt(&text) {
            return Ok(serde_json::from_str(&decrypted)?);
        }

        Ok(serde_json::from_str(&text)?)
    }

    pub async fn weapi_request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        params: Value,
    ) -> Result<T, Error> {
        let url = format!("{}{}", WEAPI_BASE, endpoint);
        let payload = weapi::encrypt(&params.to_string());

        let resp: reqwest::Response = self
            .client
            .post(&url)
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

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error {0}: {1}")]
    Http(reqwest::StatusCode, String),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
