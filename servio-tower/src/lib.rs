use bytes::Bytes;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use http::{Request, Response};
use http_body::Body;
use servio_http::http::{HttpEvent, HttpScope, ResponseChunk, ResponseTrailer, EVENT_HTTP};
use servio_service::{Event, Scope};
use std::io;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

struct Tower2Servio<S> {
    inner: S,
}

struct ResBodyStream<B> {
    inner: Box<B>,
    end: bool,
}

impl<B> ResBodyStream<B>
where
    B: Body,
{
    fn new(body: B) -> Self {
        Self {
            inner: Box::new(body),
            end: false,
        }
    }
}

impl<B> Stream for ResBodyStream<B>
where
    B: Body<Data = Bytes> + Unpin,
{
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.end {
            return Poll::Ready(None);
        }

        let res = ready!(Pin::new(&mut self.inner).poll_frame(cx));

        let frame = match res {
            Some(Ok(frame)) => frame,
            Some(Err(e)) => unimplemented!(),
            None => panic!("Body exhausted unexpectedly"),
        };

        self.end = self.inner.is_end_stream();

        let event = if frame.is_data() {
            let mut event = ResponseChunk::default();
            event.body = frame.into_data().unwrap();
            event.more = !self.end;
            Event::new(EVENT_HTTP.into(), HttpEvent::ResponseChunk(event))
        } else if frame.is_trailers() {
            let mut event = ResponseTrailer::default();
            event.headers = frame.into_trailers().unwrap();
            event.more = !self.end;
            Event::new(EVENT_HTTP.into(), HttpEvent::ResponseTrailer(event))
        } else {
            unreachable!()
        };

        Poll::Ready(Some(event))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        todo!()
    }
}

impl<ServerStream, S> servio_service::Service<ServerStream> for Tower2Servio<S>
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;

    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error>
    where
        ReqBody: Body + Unpin + Send,
        S: tower::Service<Request<ReqBody>, Response = Response<ResBody>>,
    {
        let mut b = "".to_string();
        let mut req = Request::new(b);

        let http_scope = scope.get::<HttpScope>().unwrap();

        *req.method_mut() = http_scope.method.clone();
        *req.uri_mut() = http_scope.uri.clone();
        *req.version_mut() = http_scope.version;
        *req.headers_mut() = http_scope.headers.clone();

        req.extensions_mut().insert(scope);

        self.inner.call(req);

        todo!()
    }
}

//
// impl<ReqBody, ResBody, S, M> tower::Service<Request<ReqBody>> for Tower2Servio<S>
// where
//     S: tower::Service<Request<ReqBody>, Response = Response<ResBody>>,
//     M: MakeHeaderValue<Request<ReqBody>>,
// {
//     type Response = S::Response;
//     type Error = S::Error;
//     type Future = S::Future;
//
//     #[inline]
//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }
//
//     fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
//         self.mode.apply(&self.header_name, &mut req, &mut self.make);
//         self.inner.call(req)
//     }
// }
