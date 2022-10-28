extern crate core;

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use hyper::body::{Body, Frame};
use hyper::service::Service;
use hyper::{body::Incoming as IncomingBody, Request, Response};
use servio::{AsgiService, Event, Scope};
use servio_http::http::{HttpResponseEvent, HttpScope, EVENT_HTTP_RESPONSE_BODY, PROTOCOL_HTTP};
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{ready, Context, Poll};

pub struct Servio2Hyper<T> {
    inner: T,
    client: Option<SocketAddr>,
}

impl<T> Servio2Hyper<T> {
    pub fn new(service: T, client: Option<SocketAddr>) -> Self {
        Self {
            inner: service,
            client,
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
                let mut event = HttpResponseEvent::default();
                event.body = frame
                    .into_data()
                    .expect("only data is available in request");
                event.end = self.body.is_end_stream();

                Poll::Ready(Some(Event {
                    event_type: EVENT_HTTP_RESPONSE_BODY.into(),
                    event: Arc::new(event),
                }))
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
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            has_trailers: false,
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
        let res = ready!(self.stream.poll_next_unpin(cx));
        if let Some(event) = res {
            match event.event_type.as_ref() {
                EVENT_HTTP_RESPONSE_BODY => {
                    let event = event.event.downcast::<HttpResponseEvent>().unwrap();
                    self.body_end = event.end;
                    Poll::Ready(Some(Ok(Frame::data(event.body.clone()))))
                }
                _ => panic!("invalid http event"),
            }
        } else {
            Poll::Ready(None)
        }
    }

    fn is_end_stream(&self) -> bool {
        (!self.has_trailers && self.body_end) || self.trailers_end
    }
}

impl<T> Service<Request<IncomingBody>> for Servio2Hyper<T>
where
    T: AsgiService<BodyServerStream> + 'static,
{
    type Response = Response<BodyAppStream<T::AppStream>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        // Prepare request
        let (parts, body) = req.into_parts();

        let mut scope = HttpScope::default();
        scope.version = parts.version;
        scope.method = parts.method;
        scope.headers = parts.headers;
        scope.uri = parts.uri;
        scope.client = self.client;
        let scope = Scope {
            protocol: PROTOCOL_HTTP.into(),
            scope: Arc::new(scope),
        };

        let server_stream = BodyServerStream { body };

        // Fire scope and server stream into the wrapped service, get app stream in return
        let app_stream = self.inner.call(scope, server_stream).unwrap();

        let fut = async move {
            let body = BodyAppStream::new(app_stream);

            Ok(Response::builder().body(body).unwrap())
        };

        Box::pin(fut)
    }
}
