use crate::BoxService;
use futures_core::future::BoxFuture;
use futures_core::Stream;
use futures_util::stream::BoxStream;
use futures_util::FutureExt;
use servio_service::{Event, Scope, Service};
use std::collections::HashMap;
use std::io;

#[derive(Default)]
pub struct ProtocolRouter<ServerStream>
where
    ServerStream: Stream<Item = Event>,
{
    services:
        HashMap<String, BoxService<'static, ServerStream, BoxStream<'static, Event>, io::Error>>,
}

impl<ServerStream> ProtocolRouter<ServerStream>
where
    ServerStream: Stream<Item = Event>,
{
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }
}

impl<ServerStream> Service<ServerStream> for ProtocolRouter<ServerStream>
where
    ServerStream: Stream<Item = Event> + Send,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;
    type Future = BoxFuture<'static, Result<Self::AppStream, Self::Error>>;

    fn call(&mut self, scope: Scope, server_events: ServerStream) -> Self::Future {
        if let Some(inner) = self.services.get_mut(scope.protocol()) {
            inner.call(scope, server_events)
        } else {
            futures_util::future::err(io::ErrorKind::Unsupported.into()).boxed()
        }
    }
}
