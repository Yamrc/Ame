use ame_netease::NeteaseClient;
use ame_netease::api::user::login_qr_check::{LoginQrCheckRequest, LoginQrCheckResponse};
use ame_netease::api::user::login_qr_key::{LoginQrKeyRequest, LoginQrKeyResponse};
use ame_netease::api::user::login_refresh::{LoginRefreshRequest, LoginRefreshResponse};
use ame_netease::api::user::register_anonymous::{
    RegisterAnonymousRequest, RegisterAnonymousResponse,
};
use ame_netease::api::user::status::{LoginStatusRequest, LoginStatusResponse};
use anyhow::Result;

use crate::domain::runtime::{block_on, netease_client};

#[derive(Debug, Clone)]
pub struct ResponseWithCookies<T> {
    pub body: T,
    pub set_cookie: Vec<String>,
}

pub fn fetch_login_qr_key_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginQrKeyResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginQrKeyRequest))?;
    Ok(response_with_cookies(&client, body, None))
}

pub fn check_login_qr_blocking(
    key: &str,
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginQrCheckResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginQrCheckRequest::new(key)))?;
    let cookie = body.cookie.clone();
    Ok(response_with_cookies(&client, body, cookie.as_deref()))
}

pub fn register_anonymous_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<RegisterAnonymousResponse>> {
    let cookie = cookie
        .filter(|it| !it.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0".to_string()
        });
    let client = NeteaseClient::with_cookie(cookie);

    let body = block_on(client.weapi_request(RegisterAnonymousRequest::new()))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(body.cookie.as_deref()));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn fetch_login_status_blocking(cookie: Option<&str>) -> Result<LoginStatusResponse> {
    let client = netease_client(cookie);
    block_on(client.weapi_request(LoginStatusRequest))
}

pub fn refresh_login_token_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginRefreshResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginRefreshRequest))?;
    let cookie = body.cookie.clone();
    Ok(response_with_cookies(&client, body, cookie.as_deref()))
}

fn extract_cookie_from_body(cookie: Option<&str>) -> Vec<String> {
    cookie
        .unwrap_or_default()
        .split(";;")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn response_with_cookies<T>(
    client: &NeteaseClient,
    body: T,
    cookie: Option<&str>,
) -> ResponseWithCookies<T> {
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(cookie));
    ResponseWithCookies { body, set_cookie }
}
