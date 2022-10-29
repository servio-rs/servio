use futures_core::Stream;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Scope {
    protocol: Cow<'static, str>,
    scopes: HashMap<TypeId, Arc<dyn Any + Sync + Send>>,
}

impl Scope {
    pub fn new(protocol: Cow<'static, str>) -> Self {
        Self {
            protocol,
            scopes: Default::default(),
        }
    }

    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    pub fn with_protocol(self, protocol: Cow<'static, str>) -> Self {
        Self {
            protocol,
            scopes: self.scopes,
        }
    }

    pub fn get<T: Any + Sync + Send>(&self) -> Option<Arc<T>> {
        self.scopes
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast::<T>()
            .ok()
    }

    pub fn insert<T: Any + Sync + Send>(&mut self, scope: T) -> Option<Arc<T>> {
        self.scopes
            .insert(TypeId::of::<T>(), Arc::new(scope))
            .map(|arc| arc.downcast::<T>().unwrap())
    }

    pub fn remove<T: Any + Sync + Send>(&mut self) -> Option<Arc<T>> {
        self.scopes
            .remove(&TypeId::of::<T>())
            .map(|arc| arc.downcast::<T>().unwrap())
    }

    pub fn with_scope<T: Any + Sync + Send>(mut self, scope: T) -> Self {
        let _ = self.insert(scope);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    family: Cow<'static, str>,
    event: Arc<dyn Any + Sync + Send>,
}

impl Event {
    pub fn new<T: Any + Sync + Send>(family: Cow<'static, str>, event: T) -> Self {
        Self {
            family,
            event: Arc::new(event),
        }
    }

    pub fn family(&self) -> &str {
        &self.family
    }

    pub fn get<T: Any + Sync + Send>(&self) -> Option<Arc<T>> {
        self.event.clone().downcast::<T>().ok()
    }
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
