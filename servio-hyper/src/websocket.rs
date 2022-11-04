use crate::{BoxBody, BoxError, Servio2Hyper};
use bytes::Bytes;
use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_core::{FusedStream, Stream};
use futures_util::{SinkExt, StreamExt};
use http::header::{
    CONNECTION, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
    SEC_WEBSOCKET_VERSION, UPGRADE,
};
use http::{HeaderValue, Request, Response, StatusCode};
use hyper::body::Incoming as IncomingBody;
use hyper::body::{Body, Frame, SizeHint};
use hyper::service::Service as HyperService;
use hyper::upgrade::Upgraded;
use servio_http::http::{EVENT_HTTP, PROTOCOL_HTTP};
use servio_http::websocket::{
    Accept, BinaryFrame, Close, Connect, TextFrame, WebSocketEvent, WebSocketScope, EVENT_WEBSOCKET,
};
use servio_service::{Event, Scope, Service};
use std::borrow::Cow;
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Role};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

struct Servio2HyperWebSocket<T> {
    inner: Servio2Hyper<T>,
}

impl<T> Servio2HyperWebSocket<T>
where
    T: Service<BoxStream<'static, Event>> + 'static,
{
    pub fn can_upgrade(scope: &Request<IncomingBody>) -> bool {
        scope.method() == http::Method::GET
            && scope.version() >= http::Version::HTTP_11
            && scope
                .headers()
                .get(CONNECTION)
                .and_then(|h| h.to_str().ok())
                .map(|h| {
                    h.split(|c| c == ' ' || c == ',')
                        .any(|p| p.eq_ignore_ascii_case("Upgrade"))
                })
                .unwrap_or(false)
            && scope
                .headers()
                .get(UPGRADE)
                .and_then(|h| h.to_str().ok())
                .map(|h| h.eq_ignore_ascii_case("websocket"))
                .unwrap_or(false)
            && scope
                .headers()
                .get(SEC_WEBSOCKET_VERSION)
                .map(|h| h == "13")
                .unwrap_or(false)
            && scope.headers().contains_key(SEC_WEBSOCKET_KEY)
    }

    // app_stream is already accepted
    async fn handle_connection<S>(
        mut app_stream: S,
        channel_tx: flume::Sender<Event>,
        mut ws_stream: WebSocketStream<Upgraded>,
    ) where
        S: Stream<Item = Event> + FusedStream + Send + Unpin + 'static,
    {
        loop {
            tokio::select! {
                e = app_stream.select_next_some() => {
                    let e: Event = e;
                    if e.family() == EVENT_WEBSOCKET {
                        if let Some(event) = e.get::<WebSocketEvent>() {
                            match event.as_ref() {
                                WebSocketEvent::Accept(..) | WebSocketEvent::Connect(..) => panic!("unexpected message"),
                                WebSocketEvent::TextFrame(TextFrame{data, ..}) => {let _ = ws_stream.send(Message::Text(data.clone())).await;}
                                WebSocketEvent::BinaryFrame(BinaryFrame{data, ..}) => {let _ = ws_stream.send(Message::Binary(data.to_vec())).await;}
                                WebSocketEvent::Close(Close{code, reason, ..}) => {
                                    let reason = reason.clone().unwrap_or("".into());
                                    let _ = ws_stream.close(Some(CloseFrame{code: (*code).into(), reason})).await;
                                }
                                _ => {}
                            }
                        }
                    }
                },
                e = ws_stream.select_next_some() => {
                    let e: Option<Message> = e.ok();

                    let out_event = match e {
                        Some(Message::Text(s)) => {
                            let mut event = TextFrame::default();
                            event.data = s;
                            Some(WebSocketEvent::TextFrame(event))
                        }
                        Some(Message::Binary(b)) => {
                            let mut event = BinaryFrame::default();
                            event.data = b.into();
                            Some(WebSocketEvent::BinaryFrame(event))
                        }
                        Some(Message::Close(Some(CloseFrame{code, reason}))) => {
                            let mut event = Close::default();
                            event.code = code.into();
                            event.reason = Some(reason);
                            Some(WebSocketEvent::Close(event))
                        }
                        Some(Message::Close(None)) => {
                            Some(WebSocketEvent::Close(Close::default()))
                        }
                        // handle end of stream
                        None => break,
                        _ => None
                    };

                    if let Some(event) = out_event {
                        channel_tx.send_async(Event::new(EVENT_WEBSOCKET.into(), event)).await.unwrap();
                    }
                }
            }
        }
    }

    async fn upgrade<S>(app_stream: S, req: Request<IncomingBody>, channel_tx: flume::Sender<Event>)
    where
        S: Stream<Item = Event> + Send + Unpin + 'static,
    {
        match hyper::upgrade::on(req).await {
            Ok(upgraded) => {
                Self::handle_connection(
                    app_stream.fuse(),
                    channel_tx,
                    WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await,
                )
                .await;
            }
            Err(e) => println!("upgrade error: {}", e),
        }
    }

    async fn build_response<S>(
        app_stream: S,
        req: Request<IncomingBody>,
        channel_tx: flume::Sender<Event>,
    ) -> Result<Response<BoxBody>, BoxError>
    where
        S: Stream<Item = Event> + Send + Unpin + 'static,
    {
        let ws_event = WebSocketEvent::Connect(Connect::default());
        channel_tx
            .send_async(Event::new(EVENT_WEBSOCKET.into(), ws_event))
            .await
            .unwrap();

        let mut app_stream = app_stream.peekable();
        let peeked: &Event = Pin::new(&mut app_stream).peek().await.unwrap();
        match peeked.family() {
            EVENT_HTTP => Servio2Hyper::<T>::build_response(app_stream).await,
            EVENT_WEBSOCKET => {
                let event = app_stream
                    .next()
                    .await
                    .unwrap()
                    .get::<WebSocketEvent>()
                    .unwrap();
                match event.as_ref() {
                    WebSocketEvent::Accept(Accept {
                        subprotocol,
                        headers,
                        ..
                    }) => {
                        let body: BoxBody = Box::pin(Empty {});

                        let key = req.headers().get(SEC_WEBSOCKET_KEY);
                        let derived = key.map(|k| derive_accept_key(k.as_bytes()));

                        let response = {
                            let mut res = Response::new(body);
                            *res.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
                            *res.version_mut() = req.version();

                            *res.headers_mut() = headers.clone();

                            res.headers_mut()
                                .append(CONNECTION, HeaderValue::from_static("Upgrade"));
                            res.headers_mut()
                                .append(UPGRADE, HeaderValue::from_static("websocket"));
                            res.headers_mut()
                                .append(SEC_WEBSOCKET_ACCEPT, derived.unwrap().parse().unwrap());

                            if let Some(subprotocol) = subprotocol {
                                res.headers_mut().append(
                                    SEC_WEBSOCKET_PROTOCOL,
                                    subprotocol.to_string().parse().unwrap(),
                                );
                            }

                            tokio::spawn(Self::upgrade(app_stream, req, channel_tx));

                            res
                        };

                        Ok(response)
                    }
                    _ => panic!("Unexpected message type"),
                }
            }
            _ => panic!("Unexpected event family"),
        }
    }
}

pub struct Empty {}

impl Body for Empty {
    type Data = Bytes;
    type Error = Box<dyn Error + Send + Sync>;

    #[inline]
    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(None)
    }

    fn is_end_stream(&self) -> bool {
        true
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::with_exact(0)
    }
}

impl<T> HyperService<Request<IncomingBody>> for Servio2HyperWebSocket<T>
where
    T: Service<BoxStream<'static, Event>> + 'static,
{
    type Response =
        Response<Pin<Box<dyn Body<Data = Bytes, Error = Box<dyn Error + Send + Sync>> + Send>>>;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&mut self, req: Request<IncomingBody>) -> Self::Future {
        if !Self::can_upgrade(&req) {
            return self.inner.call(req);
        }

        let http_scope = self.inner.make_http_scope(
            req.method().clone(),
            req.uri().clone(),
            req.version(),
            req.headers().clone(),
        );

        let mut ws_scope = WebSocketScope::default();
        ws_scope.subprotocols = http_scope
            .headers
            .get_all(SEC_WEBSOCKET_PROTOCOL)
            .iter()
            .map(|h| h.to_str().unwrap())
            .flat_map(|h| h.split(|c| c == ' ' || c == ','))
            .map(|s| Cow::from(s.to_string()))
            .collect();

        // Prepare request
        let scope = Scope::new(PROTOCOL_HTTP.into())
            .with_scope(http_scope)
            .with_scope(ws_scope);

        let (channel_tx, channel_rx) = flume::bounded(0);

        let server_stream = Box::pin(channel_rx.into_stream());

        // Fire scope and server stream into the wrapped service, get app stream in return
        let app_stream = self.inner.inner.call(scope, server_stream).unwrap();

        Box::pin(Self::build_response(app_stream, req, channel_tx))
    }
}
