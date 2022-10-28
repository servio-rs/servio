use futures_core::Stream;
use servio::{AsgiService, Event, Scope};
use servio_http::http::{HttpResponseEvent, HttpScope, EVENT_HTTP_RESPONSE_BODY};
use std::any::TypeId;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub struct HelloStream {}

impl Stream for HelloStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut http_event = HttpResponseEvent::default();
        http_event.body = "Hello, world!".into();
        http_event.end = true;

        Poll::Ready(Some(Event {
            event_type: EVENT_HTTP_RESPONSE_BODY.into(),
            event: Arc::new(http_event),
        }))
    }
}

pub struct HelloWorldService {}

impl<ServerStream> AsgiService<ServerStream> for HelloWorldService
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = HelloStream;
    type Error = io::Error;

    fn call(
        &mut self,
        scope: Scope,
        _server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        println!("{:?}", scope.scope.as_ref().type_id());
        println!("{:?}", TypeId::of::<HttpScope>());
        println!("{scope:?}");

        Ok(HelloStream {})
    }
}
