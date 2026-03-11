use std::{mem, sync::Arc, time::Duration};

use futures::future::BoxFuture;
use nekowg::http_client::{self, HttpClient, Url, http};
use reqwest::header::{HeaderMap, HeaderValue};

const MUSIC_163_CDN_SUFFIX: &str = ".music.126.net";
const MUSIC_163_REFERER: &str = "https://music.163.com/";

pub struct ReqwestGpuiClient {
    client: reqwest::Client,
    user_agent: Option<HeaderValue>,
    executor: TokioExecutor,
}

#[derive(Clone)]
struct TokioExecutor {
    handle: tokio::runtime::Handle,
    _runtime_guard: Option<Arc<tokio::runtime::Runtime>>,
}

impl TokioExecutor {
    fn create() -> anyhow::Result<Self> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            return Ok(Self {
                handle,
                _runtime_guard: None,
            });
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()?;
        let runtime = Arc::new(runtime);

        Ok(Self {
            handle: runtime.handle().clone(),
            _runtime_guard: Some(runtime),
        })
    }

    fn handle(&self) -> tokio::runtime::Handle {
        self.handle.clone()
    }
}

impl ReqwestGpuiClient {
    fn builder() -> reqwest::ClientBuilder {
        reqwest::Client::builder().connect_timeout(Duration::from_secs(10))
    }

    pub fn user_agent(agent: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        let user_agent = HeaderValue::from_str(agent)?;
        headers.insert(http::header::USER_AGENT, user_agent.clone());
        let client = Self::builder().default_headers(headers).build()?;

        Ok(Self {
            client,
            user_agent: Some(user_agent),
            executor: TokioExecutor::create()?,
        })
    }
}

impl HttpClient for ReqwestGpuiClient {
    fn user_agent(&self) -> Option<&HeaderValue> {
        self.user_agent.as_ref()
    }

    fn send(
        &self,
        req: http::Request<http_client::AsyncBody>,
    ) -> BoxFuture<'static, anyhow::Result<http::Response<http_client::AsyncBody>>> {
        let client = self.client.clone();
        let executor = self.executor.clone();

        Box::pin(async move {
            let (parts, body) = req.into_parts();
            let method = parts.method;
            let uri_string = parts.uri.to_string();
            let headers = parts.headers;
            let inject_referer = parts
                .uri
                .host()
                .is_some_and(|host| host.ends_with(MUSIC_163_CDN_SUFFIX));
            let request_body = to_reqwest_body(body)?;

            let (status, version, response_headers, bytes) = executor
                .handle()
                .spawn(async move {
                    let mut request = client.request(method, uri_string).headers(headers);
                    if inject_referer {
                        request = request.header(http::header::REFERER, MUSIC_163_REFERER);
                    }

                    let mut response = request.body(request_body).send().await?;
                    let status = response.status();
                    let version = response.version();
                    let headers = mem::take(response.headers_mut());
                    let bytes = response.bytes().await?;

                    Ok::<_, anyhow::Error>((status, version, headers, bytes))
                })
                .await??;

            let mut builder = http::Response::builder()
                .status(status.as_u16())
                .version(version);
            *builder
                .headers_mut()
                .expect("response headers should exist") = response_headers;

            builder
                .body(http_client::AsyncBody::from_bytes(bytes))
                .map_err(anyhow::Error::from)
        })
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }
}

pub fn build_http_client(user_agent: &str) -> anyhow::Result<Arc<dyn HttpClient>> {
    Ok(Arc::new(ReqwestGpuiClient::user_agent(user_agent)?))
}

fn to_reqwest_body(body: http_client::AsyncBody) -> anyhow::Result<reqwest::Body> {
    match body.0 {
        http_client::Inner::Empty => Ok(reqwest::Body::default()),
        http_client::Inner::Bytes(cursor) => Ok(cursor.into_inner().into()),
        http_client::Inner::AsyncReader(_) => Err(anyhow::anyhow!(
            "streaming request body is not supported by ReqwestGpuiClient"
        )),
    }
}
