use bytes::Bytes;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderMap, StatusCode};
use servio_http::http::{HttpEvent, ResponseChunk, ResponseStart, EVENT_HTTP};
use servio_service::{Event, Scope, Service};
use std::borrow::Cow;
use std::io;

#[derive(Default, Clone)]
pub struct StaticResponse {
    content: Bytes,
    status_code: StatusCode,
    headers: HeaderMap,
    media_type: Cow<'static, str>,
}

impl StaticResponse {
    pub fn new(status_code: StatusCode, content: Bytes, media_type: Cow<'static, str>) -> Self {
        Self {
            content,
            status_code,
            media_type,
            ..Default::default()
        }
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }
}

impl<ServerStream> Service<ServerStream> for StaticResponse
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;

    fn call(
        &mut self,
        _scope: Scope,
        _server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        let mut resp = ResponseChunk::default();
        resp.body = self.content.clone();

        let mut response_start = ResponseStart::default();
        response_start.status = self.status_code;
        response_start.headers = self.headers.clone();

        let headers = &mut response_start.headers;
        headers.insert(CONTENT_LENGTH, resp.body.len().into());
        headers.insert(CONTENT_TYPE, self.media_type.as_ref().parse().unwrap());

        let events = [
            Event::new(EVENT_HTTP.into(), HttpEvent::ResponseStart(response_start)),
            Event::new(EVENT_HTTP.into(), HttpEvent::ResponseChunk(resp)),
        ];

        Ok(Box::pin(futures_util::stream::iter(events)))
    }
}

macro_rules! simple_pass {
    ($srv:ident) => {
        #[derive(Default, Clone)]
        pub struct $srv {
            inner: StaticResponse,
        }

        impl $srv {
            pub fn with_headers(mut self, headers: HeaderMap) -> Self {
                self.inner = self.inner.with_headers(headers);
                self
            }
        }

        impl<ServerStream> Service<ServerStream> for $srv
        where
            ServerStream: Stream<Item = Event>,
        {
            type AppStream = BoxStream<'static, Event>;
            type Error = io::Error;

            fn call(
                &mut self,
                scope: Scope,
                server_events: ServerStream,
            ) -> Result<Self::AppStream, Self::Error> {
                self.inner.call(scope, server_events)
            }
        }
    };
}

simple_pass!(HtmlResponse);
impl HtmlResponse {
    pub fn new(status_code: StatusCode, content: Bytes) -> Self {
        Self {
            inner: StaticResponse::new(status_code, content, "text/html".into()),
        }
    }
}

simple_pass!(PlainTextResponse);
impl PlainTextResponse {
    pub fn new(status_code: StatusCode, content: Cow<'static, str>) -> Self {
        Self {
            inner: StaticResponse::new(
                status_code,
                content.to_string().into(),
                "text/plain".into(),
            ),
        }
    }
}

#[cfg(feature = "serde")]
simple_pass!(JsonResponse);
#[cfg(feature = "serde")]
impl JsonResponse {
    pub fn new<S: serde::Serialize>(status_code: StatusCode, content: &S) -> Self {
        Self {
            inner: StaticResponse::new(
                status_code,
                serde_json::to_vec(content).unwrap().into(),
                "application/json".into(),
            ),
        }
    }
}
