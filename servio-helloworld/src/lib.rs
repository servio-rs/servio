use futures_core::stream::BoxStream;
use futures_core::Stream;
use servio::{AsgiService, Event, Scope};
use servio_http::http::{HttpEvent, ResponseChunk, ResponseStart, EVENT_HTTP};
use std::io;
pub struct HelloWorldService;

impl<ServerStream> AsgiService<ServerStream> for HelloWorldService
where
    ServerStream: Stream<Item = Event>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;

    fn call(
        &mut self,
        _scope: Scope,
        _server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        let mut resp = ResponseChunk::default();
        resp.body = "Hello, world!".into();

        let events = [
            Event::new(
                EVENT_HTTP.into(),
                HttpEvent::ResponseStart(ResponseStart::default()),
            ),
            Event::new(EVENT_HTTP.into(), HttpEvent::ResponseChunk(resp)),
        ];

        Ok(Box::pin(futures_util::stream::iter(events)))
    }
}
