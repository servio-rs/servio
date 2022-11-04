use futures_core::Stream;
use futures_util::StreamExt;
use servio_service::{Event, Service};
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Buffer<S> {
    inner: S,
}

impl<S> Buffer<S> {
    pub fn new(srv: S) -> Self {
        Self { inner: srv }
    }
}

pub struct BufferedStream<S>
where
    S: Stream<Item = Event> + Send + 'static,
{
    buf_rx: flume::r#async::RecvStream<'static, Event>,
    _phantom: PhantomData<S>,
}

impl<S> BufferedStream<S>
where
    S: Stream<Item = Event> + Send + 'static,
{
    fn new(stream: S) -> Self {
        let mut s = Box::pin(stream);
        let (buf_tx, buf_rx) = flume::bounded(100);

        tokio::spawn(async move {
            loop {
                let Some(event) = s.next().await else {
                    break;
                };
                buf_tx.send_async(event).await;
            }
        });
        Self {
            buf_rx: buf_rx.into_stream(),
            _phantom: Default::default(),
        }
    }
}

impl<S> Stream for BufferedStream<S>
where
    S: Stream<Item = Event> + Send + Unpin,
{
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.buf_rx.poll_next_unpin(cx)
    }
}

impl<ServerStream, S> Service<ServerStream> for Buffer<S>
where
    ServerStream: Stream<Item = Event> + Send + Unpin + 'static,
    S: Service<BufferedStream<ServerStream>>,
{
    type AppStream = S::AppStream;
    type Error = S::Error;

    fn call(
        &mut self,
        scope: servio_service::Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        let server_buffered = BufferedStream::new(server_events);
        let app_stream = self.inner.call(scope, server_buffered)?;

        Ok(app_stream)
    }
}
