use crate::{
    http::{HttpResponseEvent, HttpScope, EVENT_HTTP_RESPONSE_BODY},
    AsgiService, Event, Scope,
};
use futures_core::Stream;
use std::any::TypeId;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub struct HelloStream {}

impl Stream for HelloStream {
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(Event {
            event_type: EVENT_HTTP_RESPONSE_BODY.into(),
            event: Arc::new(HttpResponseEvent {
                body: "Hello, world!".into(),
                end: true,
            }),
        }))
    }
}

pub struct Hello {}

impl<ServerStream> AsgiService<ServerStream> for Hello
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
