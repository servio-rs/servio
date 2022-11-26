pub mod buffer;
mod logger;
pub mod response;
pub mod routers;

use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;
use futures_core::Stream;
pub use logger::*;
use servio_service::{Event, Scope, Service};

pub struct BoxService<'a, SS, AS, E> {
    inner: Box<
        dyn Service<
                SS,
                AppStream = BoxStream<'a, AS>,
                Error = E,
                Future = BoxFuture<'a, Result<AS, E>>,
            > + Send
            + 'a,
    >,
}

impl<'a, SS, AS, E> BoxService<'a, SS, AS, E>
where
    SS: Stream<Item = Event>,
{
    pub fn new<S>(inner: S) -> Self
    where
        S: Service<SS, AppStream = AS, Error = E> + Send + 'static,
        S::Future: Send + 'static,
    {
        // let inner = Box::new(inner.map_future(|f: S::Future| Box::pin(f) as _));
        // BoxService { inner }
        todo!()
    }
}

impl<'a, SS, AS, E> Service<SS> for BoxService<'a, SS, AS, E>
where
    SS: Stream<Item = Event>,
    AS: Stream<Item = Event> + Send + Unpin,
    E: std::error::Error,
{
    type AppStream = AS;
    type Error = E;
    type Future = BoxFuture<'a, Result<AS, E>>;

    fn call(&mut self, scope: Scope, server_events: SS) -> Self::Future {
        // self.inner.call(scope, server_events)
        todo!()
    }
}

struct BoxMapper {}
