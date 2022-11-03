mod websocket;

use bytes::Bytes;
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_util::StreamExt;
use http::{Request, Response};
use hyper::body::Incoming as IncomingBody;
use hyper::body::{Body, Frame};
use hyper::service::Service as HyperService;
use servio_http::http::{
    HttpEvent, HttpScope, RequestChunk, ResponseChunk, ResponseStart, ResponseTrailer, EVENT_HTTP,
    PROTOCOL_HTTP,
};
use servio_service::{Event, Scope, Service};
use std::error::Error as StdError;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub struct Servio2Hyper<T> {
    inner: T,
    server: Option<SocketAddr>,
    client: Option<SocketAddr>,
}

type BoxError = Box<dyn StdError + Send + Sync>;
type BoxBody = Pin<Box<dyn Body<Error = BoxError, Data = Bytes> + Send>>;

impl<T> Servio2Hyper<T>
where
    T: Service<BoxStream<'static, Event>> + 'static,
{
    pub fn new(service: T, server: Option<SocketAddr>, client: Option<SocketAddr>) -> Self {
        Self {
            inner: service,
            server,
            client,
        }
    }

    async fn build_response<S>(mut app_stream: S) -> Result<Response<BoxBody>, BoxError>
    where
        S: Stream<Item = Event> + Send + Unpin + 'static,
    {
        println!("abd1");
        let event = match app_stream.next().await {
            Some(event) => event,
            None => panic!("Unexpected EOF from application"),
        };
        println!("abd2");

        let event = match event.get::<HttpEvent>() {
            Some(event) => event,
            None => panic!("Cannot get message from scope"),
        };

        match event.as_ref() {
            HttpEvent::ResponseStart(ResponseStart {
                status,
                headers,
                trailers,
                ..
            }) => {
                let body: BoxBody = Box::pin(BodyAppStream::new(app_stream, *trailers));

                let response = {
                    let mut builder = Response::builder().status(status);
                    *builder.headers_mut().unwrap() = headers.clone();
                    builder.body(body).unwrap()
                };

                Ok(response)
            }
            _ => panic!("Unexpected message type"),
        }
    }

    pub(crate) fn make_http_scope(
        &self,
        method: http::Method,
        uri: http::Uri,
        version: http::Version,
        headers: http::HeaderMap,
    ) -> HttpScope {
        let mut http_scope = HttpScope::default();
        http_scope.method = method;
        http_scope.uri = uri;
        http_scope.version = version;
        http_scope.headers = headers;
        http_scope.server = self.server;
        http_scope.client = self.client;
        http_scope
    }
}

struct BodyServerStream {
    body: IncomingBody,
    end: bool,
}

impl BodyServerStream {
    pub fn new(body: IncomingBody) -> Self {
        Self { body, end: false }
    }
}

impl Stream for BodyServerStream {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.end {
            return Poll::Ready(None);
        }

        let frame = ready!(Pin::new(&mut self.body).poll_frame(cx));

        let http_event = match frame {
            None => {
                self.end = true;
                HttpEvent::RequestChunk(RequestChunk::default())
            }
            Some(Ok(frame)) => HttpEvent::RequestChunk({
                let mut event = RequestChunk::default();
                event.body = frame
                    .into_data()
                    .expect("only data is available in request");
                event.more = !self.body.is_end_stream();
                event
            }),
            Some(Err(e)) => panic!("{e}"),
        };

        Poll::Ready(Some(Event::new(EVENT_HTTP.into(), http_event)))
    }
}

struct BodyAppStream<S>
where
    S: Stream<Item = Event>,
{
    stream: S,
    has_trailers: bool,
    body_end: bool,
    trailers_end: bool,
}

impl<S> BodyAppStream<S>
where
    S: Stream<Item = Event>,
{
    pub fn new(stream: S, has_trailers: bool) -> Self {
        Self {
            stream,
            has_trailers,
            body_end: false,
            trailers_end: false,
        }
    }
}

impl<S> Body for BodyAppStream<S>
where
    S: Stream<Item = Event> + Unpin,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        loop {
            let event = match ready!(self.stream.poll_next_unpin(cx)) {
                Some(res) => res,
                None => return Poll::Ready(None),
            };

            if event.family() == EVENT_HTTP {
                let event = event.get::<HttpEvent>().unwrap();
                match event.as_ref() {
                    HttpEvent::ResponseChunk(ResponseChunk { body, more, .. }) => {
                        self.body_end = !*more;
                        let frame = Frame::data(body.clone());
                        return Poll::Ready(Some(Ok(frame)));
                    }
                    HttpEvent::ResponseTrailer(ResponseTrailer { headers, more, .. }) => {
                        self.trailers_end = !*more;
                        let frame = Frame::trailers(headers.clone());
                        return Poll::Ready(Some(Ok(frame)));
                    }
                    _ => panic!("Unexpected event: {event:?}"),
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        (!self.has_trailers && self.body_end) || self.trailers_end
    }
}

impl<T> HyperService<Request<IncomingBody>> for Servio2Hyper<T>
where
    T: Service<BoxStream<'static, Event>> + 'static,
{
    type Response = Response<BoxBody>;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        // Prepare request
        let (parts, body) = req.into_parts();

        let http_scope =
            self.make_http_scope(parts.method, parts.uri, parts.version, parts.headers);

        let scope = Scope::new(PROTOCOL_HTTP.into()).with_scope(http_scope);

        let server_stream = Box::pin(BodyServerStream::new(body));

        // Fire scope and server stream into the wrapped service, get app stream in return
        let app_stream = self.inner.call(scope, server_stream).unwrap();

        Box::pin(Self::build_response(app_stream))
    }
}
