use bytes::Bytes;
use std::any::Any;
use std::borrow::Cow;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct HttpScope {
    pub http_version: http::Version,
    pub method: http::Method,
    pub headers: http::HeaderMap,
}

pub const PROTOCOL_HTTP: &str = "http";
pub const PROTOCOL_WEBSOCKET: &str = "websocket";

pub const EVENT_HTTP_REQUEST: &str = "http.request";
pub const EVENT_HTTP_RESPONSE_BODY: &str = "http.response_body";

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct HttpRequestEvent {
    pub body: Bytes,
    pub more_body: bool,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct HttpResponseEvent {
    pub body: Bytes,
    pub more_body: bool,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct WebsocketScope {
    pub http_version: http::Version,
    pub headers: http::HeaderMap,

    pub subprotocols: Vec<Cow<'static, str>>,
}

#[derive(Clone)]
pub struct Scope {
    pub protocol: Cow<'static, str>,
    pub scope: Arc<Box<dyn Any + Send>>,
}

impl Scope {
    pub fn protocol(&self) -> &str {
        self.protocol.as_ref()
    }
}

#[derive(Clone)]
pub struct Event {
    pub event_type: Cow<'static, str>,
    pub event: Arc<Box<dyn Any + Send>>,
}

pub struct WebSocketEvent {}

pub trait AsgiService<ServerStream: futures_core::Stream<Item = Event>> {
    type AppStream: futures_core::Stream<Item = Event>;
    type Error: std::error::Error;

    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error>;
}

pub trait AsgiService2<ServerStream: futures_core::Stream<Item = Event>, ScopeT: Any> {
    type AppStream: futures_core::Stream<Item = Event>;
    type Error: std::error::Error;

    fn call(
        &mut self,
        scope: ScopeT,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error>;
}

pub struct HelloStream {
    header_sent: bool,
}

impl futures_core::Stream for HelloStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if !self.header_sent {
        //     Ok(Some())
        // }
        Poll::Ready(Some(Event {
            event_type: EVENT_HTTP_RESPONSE_BODY.into(),
            event: Arc::new(Box::new(HttpResponseEvent {
                body: "Hello, world!".into(),
                more_body: false,
            })),
        }))
    }
}

pub struct Hello {}

impl<ServerStream> AsgiService<ServerStream> for Hello
where
    ServerStream: futures_core::Stream<Item = Event>,
{
    type AppStream = HelloStream;
    type Error = io::Error;

    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        if scope.protocol == PROTOCOL_HTTP {
            Ok(HelloStream { header_sent: false })
        } else {
            Err(io::ErrorKind::Unsupported.into())
        }
    }
}
