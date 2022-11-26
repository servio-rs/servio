use bytes::Bytes;
use futures_core::Stream;
use futures_util::{future::Ready, stream::Iter};
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::{HeaderMap, StatusCode};
use servio_http::http::{HttpEvent, ResponseChunk, ResponseStart, EVENT_HTTP};
use servio_service::{Event, Scope, Service};
use std::borrow::Cow;
use std::convert::Infallible;
use std::vec::IntoIter;

#[derive(Default, Clone)]
pub struct StaticResponse {
    events: Vec<Event>,
}

impl StaticResponse {
    pub fn new(
        status_code: StatusCode,
        content: Bytes,
        media_type: Cow<'static, str>,
        headers: HeaderMap,
    ) -> Self {
        let mut resp = ResponseChunk::default();
        resp.body = content;

        let mut response_start = ResponseStart::default();
        response_start.status = status_code;
        response_start.headers = headers;

        let headers = &mut response_start.headers;
        headers.insert(CONTENT_LENGTH, resp.body.len().into());
        headers.insert(CONTENT_TYPE, media_type.as_ref().parse().unwrap());

        Self {
            events: vec![
                Event::new(EVENT_HTTP.into(), HttpEvent::ResponseStart(response_start)),
                Event::new(EVENT_HTTP.into(), HttpEvent::ResponseChunk(resp)),
            ],
        }
    }
}

impl<ServerStream> Service<ServerStream> for StaticResponse
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = Iter<IntoIter<Event>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::AppStream, Self::Error>>;

    fn call(&mut self, _scope: Scope, _server_events: ServerStream) -> Self::Future {
        let stream = futures_util::stream::iter(self.events.clone());
        futures_util::future::ok(stream)
    }
}

macro_rules! simple_pass {
    ($srv:ident) => {
        #[derive(Default, Clone)]
        pub struct $srv {
            inner: StaticResponse,
        }

        impl<ServerStream> Service<ServerStream> for $srv
        where
            ServerStream: Stream<Item = Event>,
        {
            type AppStream = <StaticResponse as Service<ServerStream>>::AppStream;
            type Error = <StaticResponse as Service<ServerStream>>::Error;
            type Future = <StaticResponse as Service<ServerStream>>::Future;

            fn call(&mut self, scope: Scope, server_events: ServerStream) -> Self::Future {
                self.inner.call(scope, server_events)
            }
        }
    };
}

simple_pass!(HtmlResponse);
impl HtmlResponse {
    pub fn new(status_code: StatusCode, content: Bytes, headers: HeaderMap) -> Self {
        Self {
            inner: StaticResponse::new(status_code, content, "text/html".into(), headers),
        }
    }
}

simple_pass!(PlainTextResponse);
impl PlainTextResponse {
    pub fn new(status_code: StatusCode, content: Cow<'static, str>, headers: HeaderMap) -> Self {
        Self {
            inner: StaticResponse::new(
                status_code,
                content.to_string().into(),
                "text/plain".into(),
                headers,
            ),
        }
    }
}

#[cfg(feature = "serde")]
simple_pass!(JsonResponse);
#[cfg(feature = "serde")]
impl JsonResponse {
    pub fn new<S: serde::Serialize>(
        status_code: StatusCode,
        content: &S,
        headers: HeaderMap,
    ) -> Self {
        Self {
            inner: StaticResponse::new(
                status_code,
                serde_json::to_vec(content).unwrap().into(),
                "application/json".into(),
                headers,
            ),
        }
    }
}
