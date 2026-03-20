use std::future::Future;

use ame_netease::NeteaseClient;
use anyhow::{Context as _, Result};

pub fn block_on<F, T, E>(future: F) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build temporary tokio runtime")?;
    Ok(runtime.block_on(future)?)
}

pub fn netease_client(cookie: Option<&str>) -> NeteaseClient {
    cookie
        .filter(|it| !it.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie)
}
