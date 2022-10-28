use futures_core::Stream;
use std::any::Any;
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Scope {
    pub protocol: Cow<'static, str>,
    pub scope: Arc<dyn Any + Sync + Send>,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub event_type: Cow<'static, str>,
    pub event: Arc<dyn Any + Sync + Send>,
}

pub trait AsgiService<ServerStream: Stream<Item = Event>> {
    type AppStream: Stream<Item = Event> + Send + Unpin;
    type Error: StdError;

    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error>;
}
