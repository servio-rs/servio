use bytes::Bytes;
use futures_core::future::BoxFuture;
use futures_core::Stream;
use futures_util::StreamExt;
use hyper::body::{Body, Frame};
use hyper::service::Service as HyperService;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use servio::{AsgiService, Event, Scope};
use servio_http::http::{
    HttpEvent, HttpScope, ResponseChunk, ResponseStart, ResponseTrailer, EVENT_HTTP, PROTOCOL_HTTP,
};
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{ready, Context, Poll};

pub struct Servio2Hyper<T> {
    inner: T,
    server: Option<SocketAddr>,
    client: Option<SocketAddr>,
}

impl<T> Servio2Hyper<T> {
    pub fn new(service: T, server: Option<SocketAddr>, client: Option<SocketAddr>) -> Self {
        Self {
            inner: service,
            server,
            client,
        }
    }
}

impl<T> Servio2Hyper<T>
where
    T: AsgiService<BodyServerStream> + 'static,
{
    async fn build_response(
        mut app_stream: T::AppStream,
    ) -> Result<
        <Servio2Hyper<T> as HyperService<Request<IncomingBody>>>::Response,
        <Servio2Hyper<T> as HyperService<Request<IncomingBody>>>::Error,
    > {
        let event = match app_stream.next().await {
            Some(event) => event,
            None => panic!("Unexpected EOF from application"),
        };

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
                let body = BodyAppStream::new(app_stream, *trailers);
                let response = {
                    let mut builder = Response::builder().status(status);
                    builder.headers_mut().unwrap().clone_from(headers);
                    builder.body(body).unwrap()
                };

                Ok(response)
            }
            _ => panic!("Unexpected message type"),
        }
    }
}

pub struct BodyServerStream {
    body: IncomingBody,
}

impl Stream for BodyServerStream {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let frame = ready!(Pin::new(&mut self.body).poll_frame(cx));

        match frame {
            None => Poll::Ready(None),
            Some(Err(e)) => panic!("{e}"),
            Some(Ok(frame)) => {
                let event = HttpEvent::ResponseChunk({
                    let mut event = ResponseChunk::default();
                    event.body = frame
                        .into_data()
                        .expect("only data is available in request");
                    event.more = !self.body.is_end_stream();
                    event
                });

                Poll::Ready(Some(Event::new(EVENT_HTTP.into(), event)))
            }
        }
    }
}

pub struct BodyAppStream<S>
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
    type Error = io::Error;

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
    T: AsgiService<BodyServerStream> + 'static,
{
    type Response = Response<BodyAppStream<T::AppStream>>;
    type Error = hyper::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        // Prepare request
        let (parts, body) = req.into_parts();

        let mut http_scope = HttpScope::default();
        http_scope.version = parts.version;
        http_scope.method = parts.method;
        http_scope.headers = parts.headers;
        http_scope.uri = parts.uri;
        http_scope.server = self.server;
        http_scope.client = self.client;

        let scope = Scope::new(PROTOCOL_HTTP.into()).with_scope(http_scope);

        let server_stream = BodyServerStream { body };

        // Fire scope and server stream into the wrapped service, get app stream in return
        let app_stream = self.inner.call(scope, server_stream).unwrap();

        Box::pin(Self::build_response(app_stream))
    }
}
