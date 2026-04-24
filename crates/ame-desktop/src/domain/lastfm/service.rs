use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ame_core::storage::StateStorage;
use md5::{Digest, Md5};
use reqwest::StatusCode;
use serde_json::Value;
use tiny_http::{Header, Method, Response, Server, StatusCode as TinyStatusCode};

use crate::app::runtime::KEY_LASTFM_SCROBBLE_QUEUE;
use crate::domain::lastfm::{LastFmBuildConfig, LastFmScrobbleRecord, LastFmSession};
use crate::domain::runtime::block_on;

const LASTFM_API_ENDPOINT: &str = "https://ws.audioscrobbler.com/2.0/";
const LASTFM_AUTH_ENDPOINT: &str = "https://www.last.fm/api/auth/";
const LASTFM_CALLBACK_PATH: &str = "/callback/";
const LASTFM_CALLBACK_URL: &str = "http://localhost:26211/callback/";
const LASTFM_CALLBACK_BIND_ADDRESSES: [&str; 2] = ["127.0.0.1:26211", "[::1]:26211"];
const LASTFM_AUTH_TIMEOUT: Duration = Duration::from_secs(120);
const LASTFM_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LastFmError {
    MissingConfiguration(String),
    InvalidSession(String),
    Retryable(String),
    Fatal(String),
}

impl LastFmError {
    pub fn message(&self) -> &str {
        match self {
            Self::MissingConfiguration(message)
            | Self::InvalidSession(message)
            | Self::Retryable(message)
            | Self::Fatal(message) => message,
        }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }
}

impl std::fmt::Display for LastFmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for LastFmError {}

#[derive(Debug, Clone)]
pub struct LastFmNowPlayingPayload {
    pub artist: String,
    pub track: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

pub fn authenticate_via_browser(config: LastFmBuildConfig) -> Result<LastFmSession, LastFmError> {
    ensure_configured(config)?;

    let flow_id = generate_flow_id();
    let callback_url = format!("{LASTFM_CALLBACK_URL}?flow_id={flow_id}");
    let auth_url = format!(
        "{LASTFM_AUTH_ENDPOINT}?api_key={}&cb={}",
        config.api_key.expect("validated config"),
        percent_encode_component(&callback_url)
    );

    let mut servers = open_callback_servers()?;
    webbrowser::open(&auth_url).map_err(|err| {
        LastFmError::Fatal(format!("Failed to open default browser for Last.fm: {err}"))
    })?;

    let token = await_callback_token(&mut servers, &flow_id)?;
    let session_key = fetch_session_key(config, &token)?;
    let user_name = fetch_user_name(config, &session_key)?;
    Ok(LastFmSession {
        session_key,
        user_name: Some(user_name),
    })
}

pub fn fetch_user_name(
    config: LastFmBuildConfig,
    session_key: &str,
) -> Result<String, LastFmError> {
    ensure_configured(config)?;
    let body = signed_call(
        config,
        &[
            ("method", "user.getInfo".to_string()),
            ("sk", session_key.to_string()),
        ],
    )?;
    body.get("user")
        .and_then(|user| user.get("name"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| LastFmError::Fatal("Last.fm user.getInfo returned no username".to_string()))
}

pub fn update_now_playing(
    config: LastFmBuildConfig,
    session_key: &str,
    payload: &LastFmNowPlayingPayload,
) -> Result<(), LastFmError> {
    ensure_configured(config)?;
    let mut params = vec![
        ("method", "track.updateNowPlaying".to_string()),
        ("sk", session_key.to_string()),
        ("artist", payload.artist.clone()),
        ("track", payload.track.clone()),
    ];
    if let Some(album) = payload
        .album
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        params.push(("album", album.to_string()));
    }
    if let Some(duration_ms) = payload.duration_ms.filter(|value| *value > 0) {
        params.push(("duration", (duration_ms / 1000).to_string()));
    }
    let _ = signed_call(config, &params)?;
    Ok(())
}

pub fn scrobble(
    config: LastFmBuildConfig,
    session_key: &str,
    record: &LastFmScrobbleRecord,
) -> Result<(), LastFmError> {
    ensure_configured(config)?;
    let mut params = vec![
        ("method", "track.scrobble".to_string()),
        ("sk", session_key.to_string()),
        ("artist", record.artist.clone()),
        ("track", record.track.clone()),
        ("timestamp", record.started_at_unix_secs.to_string()),
    ];
    if let Some(album) = record
        .album
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        params.push(("album", album.to_string()));
    }
    if let Some(duration_ms) = record.duration_ms.filter(|value| *value > 0) {
        params.push(("duration", (duration_ms / 1000).to_string()));
    }

    let body = signed_call(config, &params)?;
    let accepted = body
        .get("scrobbles")
        .and_then(|value| value.get("@attr"))
        .and_then(|value| value.get("accepted"))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(1);
    if accepted == 0 {
        return Err(LastFmError::Fatal(
            "Last.fm ignored the submitted scrobble".to_string(),
        ));
    }

    Ok(())
}

pub fn load_scrobble_queue(
    state_store: &StateStorage,
) -> Result<Vec<LastFmScrobbleRecord>, String> {
    state_store
        .get(KEY_LASTFM_SCROBBLE_QUEUE)
        .map(|value| value.unwrap_or_default())
        .map_err(|err| format!("Failed to read Last.fm scrobble queue: {err}"))
}

pub fn store_scrobble_queue(
    state_store: &StateStorage,
    queue: &[LastFmScrobbleRecord],
) -> Result<(), String> {
    if queue.is_empty() {
        state_store
            .remove(KEY_LASTFM_SCROBBLE_QUEUE)
            .map_err(|err| format!("Failed to clear Last.fm scrobble queue: {err}"))
    } else {
        state_store
            .set(KEY_LASTFM_SCROBBLE_QUEUE, &queue)
            .map_err(|err| format!("Failed to write Last.fm scrobble queue: {err}"))
    }
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

pub fn retry_backoff_ms(retry_count: u32) -> u64 {
    let step = 5_000_u64.saturating_mul(1_u64 << retry_count.min(4));
    step.min(60_000)
}

fn fetch_session_key(config: LastFmBuildConfig, token: &str) -> Result<String, LastFmError> {
    let body = signed_call(
        config,
        &[
            ("method", "auth.getSession".to_string()),
            ("token", token.to_string()),
        ],
    )?;
    body.get("session")
        .and_then(|session| session.get("key"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            LastFmError::Fatal("Last.fm auth.getSession returned no session key".to_string())
        })
}

fn signed_call(config: LastFmBuildConfig, params: &[(&str, String)]) -> Result<Value, LastFmError> {
    ensure_configured(config)?;

    let mut request_params = BTreeMap::<String, String>::new();
    request_params.insert(
        "api_key".to_string(),
        config.api_key.expect("validated config").to_string(),
    );
    for (key, value) in params {
        request_params.insert((*key).to_string(), value.clone());
    }
    let api_sig = sign_params(
        &request_params,
        config.shared_secret.expect("validated config"),
    );

    let mut form = request_params.into_iter().collect::<Vec<_>>();
    form.push(("api_sig".to_string(), api_sig));
    form.push(("format".to_string(), "json".to_string()));

    let (status, body) = block_on(async move {
        let response = reqwest::Client::builder()
            .timeout(LASTFM_REQUEST_TIMEOUT)
            .user_agent(format!(
                "Ame/{} (+https://github.com/Yamrc/Ame)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?
            .post(LASTFM_API_ENDPOINT)
            .form(&form)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        Ok::<_, reqwest::Error>((status, body))
    })
    .map_err(|err| LastFmError::Retryable(format!("Last.fm request failed: {err}")))?;

    if !status.is_success() {
        return Err(classify_http_error(status, &body));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|err| LastFmError::Fatal(format!("Failed to parse Last.fm JSON: {err}")))?;

    if let Some(error_code) = json.get("error").and_then(Value::as_i64) {
        let message = json
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Last.fm request failed");
        return Err(classify_api_error(error_code, message));
    }

    Ok(json)
}

fn sign_params(params: &BTreeMap<String, String>, shared_secret: &str) -> String {
    let mut source = String::new();
    for (key, value) in params {
        if key == "format" || key == "callback" || key == "cb" {
            continue;
        }
        source.push_str(key);
        source.push_str(value);
    }
    source.push_str(shared_secret);
    Md5::digest(source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn ensure_configured(config: LastFmBuildConfig) -> Result<(), LastFmError> {
    if config.is_configured() {
        Ok(())
    } else {
        Err(LastFmError::MissingConfiguration(
            "Last.fm API credentials are not configured for this build".to_string(),
        ))
    }
}

fn classify_http_error(status: StatusCode, body: &str) -> LastFmError {
    let message = if body.trim().is_empty() {
        format!("Last.fm returned HTTP {}", status.as_u16())
    } else {
        format!("Last.fm returned HTTP {}: {}", status.as_u16(), body.trim())
    };

    if status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS {
        LastFmError::Retryable(message)
    } else {
        LastFmError::Fatal(message)
    }
}

fn classify_api_error(code: i64, message: &str) -> LastFmError {
    match code {
        8 | 11 | 16 | 29 => {
            LastFmError::Retryable(format!("Last.fm temporary error ({code}): {message}"))
        }
        9 => LastFmError::InvalidSession(format!("Last.fm session is invalid: {message}")),
        10 | 13 | 26 => LastFmError::MissingConfiguration(format!(
            "Last.fm credentials are invalid for this build ({code}): {message}"
        )),
        _ => LastFmError::Fatal(format!("Last.fm rejected the request ({code}): {message}")),
    }
}

fn generate_flow_id() -> String {
    format!(
        "{:016x}{:016x}",
        rand::random::<u64>(),
        rand::random::<u64>()
    )
}

fn open_callback_servers() -> Result<Vec<Server>, LastFmError> {
    let mut servers = Vec::new();
    let mut errors = Vec::new();
    for bind_address in LASTFM_CALLBACK_BIND_ADDRESSES {
        match Server::http(bind_address) {
            Ok(server) => servers.push(server),
            Err(err) => errors.push(format!("{bind_address}: {err}")),
        }
    }

    if servers.is_empty() {
        Err(LastFmError::Fatal(format!(
            "Failed to bind Last.fm callback server on port 26211: {}",
            errors.join("; ")
        )))
    } else {
        Ok(servers)
    }
}

fn await_callback_token(
    servers: &mut [Server],
    expected_flow_id: &str,
) -> Result<String, LastFmError> {
    let deadline = Instant::now() + LASTFM_AUTH_TIMEOUT;
    while Instant::now() < deadline {
        for server in &*servers {
            let request = server
                .recv_timeout(Duration::from_millis(200))
                .map_err(|err| {
                    LastFmError::Fatal(format!(
                        "Last.fm callback server failed while waiting: {err}"
                    ))
                })?;
            let Some(request) = request else {
                continue;
            };
            if let Some(token) = handle_callback_request(request, expected_flow_id)? {
                return Ok(token);
            }
        }
    }

    Err(LastFmError::Fatal(
        "Timed out waiting for Last.fm authorization callback".to_string(),
    ))
}

fn handle_callback_request(
    request: tiny_http::Request,
    expected_flow_id: &str,
) -> Result<Option<String>, LastFmError> {
    if request.method() != &Method::Get {
        respond_html(request, 405, "Method Not Allowed", "Only GET is supported.");
        return Ok(None);
    }

    let (path, query) = split_url(request.url());
    if path != LASTFM_CALLBACK_PATH {
        respond_html(request, 404, "Not Found", "Unknown callback path.");
        return Ok(None);
    }

    let params = parse_query(query.unwrap_or_default());
    let Some(flow_id) = params.get("flow_id") else {
        respond_html(
            request,
            400,
            "Missing Flow",
            "The callback did not include a flow identifier.",
        );
        return Ok(None);
    };
    if flow_id != expected_flow_id {
        respond_html(
            request,
            400,
            "Flow Mismatch",
            "This authorization flow is no longer active.",
        );
        return Ok(None);
    }

    let Some(token) = params.get("token").filter(|value| !value.trim().is_empty()) else {
        respond_html(
            request,
            400,
            "Missing Token",
            "Last.fm did not return an authorization token.",
        );
        return Ok(None);
    };

    respond_html(
        request,
        200,
        "Last.fm Connected",
        "Last.fm authorization completed. You can close this tab and return to Ame.",
    );
    Ok(Some(token.clone()))
}

fn respond_html(request: tiny_http::Request, status_code: u16, title: &str, body: &str) {
    let response = Response::from_string(format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title></head><body><h1>{title}</h1><p>{body}</p></body></html>"
    ))
    .with_status_code(TinyStatusCode(status_code))
    .with_header(
        Header::from_bytes(
            &b"Content-Type"[..],
            &b"text/html; charset=utf-8"[..],
        )
        .expect("valid html header"),
    );
    let _ = request.respond(response);
}

fn split_url(url: &str) -> (&str, Option<&str>) {
    match url.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (url, None),
    }
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    query
        .split('&')
        .filter(|segment| !segment.trim().is_empty())
        .filter_map(|segment| {
            let (key, value) = match segment.split_once('=') {
                Some((key, value)) => (key, value),
                None => (segment, ""),
            };
            let key = percent_decode_component(key)?;
            let value = percent_decode_component(value)?;
            Some((key, value))
        })
        .collect()
}

fn percent_encode_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn percent_decode_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                output.push((high << 4) | low);
                index += 3;
            }
            value => {
                output.push(value);
                index += 1;
            }
        }
    }
    String::from_utf8(output).ok()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{parse_query, percent_decode_component, percent_encode_component, sign_params};

    #[test]
    fn percent_codec_round_trips_callback_url() {
        let raw = "http://localhost:26211/callback/?flow_id=abc123";
        let encoded = percent_encode_component(raw);
        let decoded = percent_decode_component(&encoded).expect("decode");
        assert_eq!(decoded, raw);
    }

    #[test]
    fn query_parser_decodes_percent_encoded_values() {
        let params = parse_query("flow_id=abc&token=hello%20world");
        assert_eq!(params.get("token").map(String::as_str), Some("hello world"));
    }

    #[test]
    fn signature_ignores_format_and_callback_keys() {
        let mut params = BTreeMap::new();
        params.insert("api_key".to_string(), "key".to_string());
        params.insert("method".to_string(), "auth.getSession".to_string());
        params.insert("token".to_string(), "token".to_string());
        params.insert("format".to_string(), "json".to_string());
        params.insert("cb".to_string(), "http://localhost".to_string());

        let signature = sign_params(&params, "secret");
        params.remove("format");
        params.remove("cb");

        assert_eq!(signature, sign_params(&params, "secret"));
        assert_eq!(signature.len(), 32);
    }
}
