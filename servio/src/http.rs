use bytes::Bytes;
use std::fmt::Debug;
use std::net::SocketAddr;

pub const PROTOCOL_HTTP: &str = "http";

#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct HttpScope {
    pub version: ::http::Version,
    pub method: ::http::Method,
    pub headers: ::http::HeaderMap,
    pub uri: ::http::Uri,

    pub client: Option<SocketAddr>,
}

pub const EVENT_HTTP_REQUEST_BODY: &str = "http.request_body";
pub const EVENT_HTTP_RESPONSE_BODY: &str = "http.response_body";

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct HttpRequestEvent {
    pub body: Bytes,
    pub end: bool,
}

impl Default for HttpRequestEvent {
    fn default() -> Self {
        Self {
            body: Bytes::default(),
            end: true,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct HttpResponseEvent {
    pub body: Bytes,
    pub end: bool,
}

impl Default for HttpResponseEvent {
    fn default() -> Self {
        Self {
            body: Bytes::default(),
            end: true,
        }
    }
}
