use actix_http::body::{BodySize, MessageBody};
use actix_http::Payload;
use actix_service::boxed::BoxFuture;
use actix_service::{fn_factory, Service as ActixService};
use actix_web::{get, web, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use futures_core::Stream;
use futures_util::stream::Peekable;
use futures_util::{FutureExt, StreamExt};
use http::StatusCode;
use servio::{AsgiService, Event, Scope};
use servio_helloworld::{HelloStream, HelloWorldService};
use servio_http::http::{
    HttpRequestEvent, HttpResponseEvent, HttpScope, EVENT_HTTP_REQUEST_BODY,
    EVENT_HTTP_RESPONSE_BODY, PROTOCOL_HTTP,
};
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{ready, Context, Poll};

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

struct Servio2Actix {
    inner: Mutex<HelloWorldService>,
}

impl Servio2Actix {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HelloWorldService {}),
        }
    }
}

impl ActixService<actix_http::Request> for Servio2Actix {
    type Response = HttpResponse<PayloadAppStream<HelloStream>>;
    type Error = ();
    type Future = BoxFuture<Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&self, req: actix_http::Request) -> Self::Future {
        let (request, payload) = req.into_parts();

        let mut http_scope = HttpScope::default();
        http_scope.version = request.version;
        http_scope.method = request.method.clone();
        http_scope.headers = http::HeaderMap::from_iter(request.headers().clone().into_iter());
        http_scope.uri = request.uri.clone();

        http_scope.client = request.peer_addr;

        let scope = Scope {
            protocol: PROTOCOL_HTTP.into(),
            scope: Arc::new(http_scope),
        };

        let app_stream = self
            .inner
            .lock()
            .unwrap()
            .call(scope, PayloadServerStream::new(payload))
            .unwrap();

        let fut = async {
            let body = PayloadAppStream {
                payload: app_stream,
            };
            let response = Self::Response::with_body(StatusCode::OK, body);
            Ok(response)
        };

        fut.boxed()
    }
}

struct PayloadServerStream {
    payload: Peekable<Payload>,
}

impl PayloadServerStream {
    fn new(payload: Payload) -> Self {
        Self {
            payload: payload.peekable(),
        }
    }
}

struct PayloadAppStream<S>
where
    S: Stream<Item = Event> + Unpin,
{
    payload: S,
}

impl<S> MessageBody for PayloadAppStream<S>
where
    S: Stream<Item = Event> + Unpin,
{
    type Error = io::Error;

    fn size(&self) -> BodySize {
        BodySize::Stream
    }

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Bytes, Self::Error>>> {
        let res = ready!(self.payload.poll_next_unpin(cx));

        match res {
            None => Poll::Ready(None),
            Some(event) => match event.event_type.as_ref() {
                EVENT_HTTP_RESPONSE_BODY => {
                    let event = event.event.downcast::<HttpResponseEvent>().unwrap();
                    Poll::Ready(Some(Ok(event.body.clone())))
                }
                _ => unimplemented!(),
            },
        }
    }
}

impl Stream for PayloadServerStream {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let res = ready!(self.payload.poll_next_unpin(cx));

        match res {
            None => Poll::Ready(None),
            Some(Ok(body)) => {
                let end = matches!(
                    Pin::new(&mut self.payload).peek().poll_unpin(cx),
                    Poll::Ready(None)
                );

                let mut event = HttpRequestEvent::default();
                event.body = body;
                event.end = end;

                Poll::Ready(Some(Self::Item {
                    event_type: EVENT_HTTP_REQUEST_BODY.into(),
                    event: Arc::new(event),
                }))
            }
            Some(Err(e)) => panic!("{e:?}"),
        }
    }
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let fac = fn_factory(|| Servio2Actix {});
    HttpServer::new(|| Servio2Actix::new())
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
