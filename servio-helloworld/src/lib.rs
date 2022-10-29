use futures_core::stream::BoxStream;
use futures_core::Stream;
use servio::{AsgiService, Event, Scope};
use servio_http::http::{HttpEvent, HttpScope, ResponseChunk, EVENT_HTTP};
use std::any::TypeId;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

struct HelloStream {}

impl Stream for HelloStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let http_event = HttpEvent::ResponseChunk({
            let mut event = ResponseChunk::default();
            event.body = "Hello, world!".into();
            event
        });

        Poll::Ready(Some(Event::new(EVENT_HTTP.into(), http_event)))
    }
}

pub struct HelloWorldService;

impl<ServerStream> AsgiService<ServerStream> for HelloWorldService
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;

    fn call(
        &mut self,
        scope: Scope,
        _server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        println!("{:?}", TypeId::of::<HttpScope>());
        println!("{scope:?}");

        Ok(Box::pin(HelloStream {}))
    }
}
