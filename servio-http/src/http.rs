use bytes::Bytes;
use std::net::SocketAddr;

pub const PROTOCOL_HTTP: &str = "http";
pub const EVENT_HTTP: &str = "http";

#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct HttpScope {
    pub method: http::Method,
    pub uri: http::Uri,
    pub version: http::Version,
    pub headers: http::HeaderMap,

    pub server: Option<SocketAddr>,
    pub client: Option<SocketAddr>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum HttpEvent {
    RequestChunk(RequestChunk),
    ResponseChunk(ResponseChunk),
    ResponseStart(ResponseStart),
    ResponseTrailer(ResponseTrailer),
    Disconnect(Disconnect),
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct RequestChunk {
    pub body: Bytes,
    pub more: bool,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct ResponseChunk {
    pub body: Bytes,
    pub more: bool,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct ResponseStart {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub trailers: bool,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct ResponseTrailer {
    pub headers: http::HeaderMap,
    pub more: bool,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct Disconnect {}
