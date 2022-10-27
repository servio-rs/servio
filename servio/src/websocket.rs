use std::borrow::Cow;

pub const PROTOCOL_WEBSOCKET: &str = "websocket";

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct WebsocketScope {
    pub http_version: ::http::Version,
    pub headers: ::http::HeaderMap,

    pub subprotocols: Vec<Cow<'static, str>>,
}

pub struct WebSocketEvent {}
