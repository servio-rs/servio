use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_util::StreamExt;
use servio_service::{Event, Service};
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

pub struct BufferedStream {
    buf_rx: flume::r#async::RecvStream<'static, Event>,
}

impl BufferedStream {
    fn new(mut stream: BoxStream<'static, Event>) -> Self {
        let (buf_tx, buf_rx) = flume::bounded(100);

        tokio::spawn(async move {
            loop {
                let Some(event) = stream.next().await else {
                    break;
                };
                buf_tx.send_async(event).await;
            }
        });
        Self {
            buf_rx: buf_rx.into_stream(),
        }
    }
}

impl Stream for BufferedStream {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.buf_rx.poll_next_unpin(cx)
    }
}

impl<ServerStream, S> Service<ServerStream> for Buffer<S>
where
    ServerStream: Stream<Item = Event> + Send + 'static,
    S: Service<BufferedStream>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = S::Error;

    fn call(
        &mut self,
        scope: servio_service::Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        let server_buffered = BufferedStream::new(server_events.boxed());
        let app_stream = self.inner.call(scope, server_buffered)?;
        Ok(BufferedStream::new(app_stream.boxed()).boxed())
    }
}
