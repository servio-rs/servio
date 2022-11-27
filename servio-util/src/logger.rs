use futures_core::Stream;
use servio_service::{Event, Service};
use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use tracing::debug;

pub struct Logger<Scope, S> {
    inner: S,
    _phantom: PhantomData<Scope>,
}

impl<Scope, S> Logger<Scope, S>
where
    Scope: Any + Sync + Send,
{
    pub fn new(srv: S) -> Self {
        Self {
            inner: srv,
            _phantom: Default::default(),
        }
    }
}

impl<ServerStream, Scope, S> Service<ServerStream> for Logger<Scope, S>
where
    Scope: Any + Sync + Send + Debug,
    ServerStream: Stream<Item = Event> + Send,
    S: Service<ServerStream>,
{
    type AppStream = S::AppStream;
    type Error = S::Error;
    type Future = S::Future;

    fn call(&mut self, scope: servio_service::Scope, server_events: ServerStream) -> Self::Future {
        if let Some(scope) = scope.get::<Scope>() {
            debug!("{scope:?}");
        }
        self.inner.call(scope, server_events)
    }
}
