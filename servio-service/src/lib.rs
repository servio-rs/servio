#![forbid(unsafe_code)]

use futures_core::Stream;
use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

/// Request metadata, containing protocol identifier and a set of scopes, identified by type.
///
/// It works like a TypeMap with a tag. A typical way to use the scope is to match protocol
/// identifier by value and get one or multiple scopes.
/// `Scope` is itself protocol-independent with protocol identifiers and scope types provided in
/// other crates. For example, HTTP and WebSocket are defined in servio-http crate.
///
/// ASGI equivalent: [Connection Scope](https://asgi.readthedocs.io/en/latest/specs/main.html#connection-scope)
#[derive(Clone, Debug)]
pub struct Scope {
    protocol: Cow<'static, str>,
    scopes: HashMap<TypeId, Arc<dyn Any + Sync + Send>, BuildHasherDefault<TypeIdHasher>>,
}

impl Scope {
    /// Creates a new `Scope` with a specified protocol identifier.
    #[inline]
    pub fn new(protocol: Cow<'static, str>) -> Self {
        Self {
            protocol,
            scopes: Default::default(),
        }
    }

    /// Returns a protocol identifier of `Scope`.
    #[inline]
    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Returns new `Scope` with specified prococol identifier, consuming surrent `Scope`.
    #[inline]
    pub fn with_protocol(self, protocol: Cow<'static, str>) -> Self {
        Self {
            protocol,
            scopes: self.scopes,
        }
    }

    /// Returns reference-counted scope of provided type.
    /// This scope can be saved by middleware for internal use.
    #[inline]
    pub fn get<T: Any + Sync + Send>(&self) -> Option<Arc<T>> {
        self.scopes
            .get(&TypeId::of::<T>())?
            .clone()
            .downcast::<T>()
            .ok()
    }

    /// Returns reference to scope of provided type.
    #[inline]
    pub fn get_ref<T: Any + Sync + Send>(&self) -> Option<&T> {
        self.scopes.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// Inserts a scope of specified type into `Scope`.
    #[inline]
    pub fn insert<T: Any + Sync + Send>(&mut self, scope: T) -> Option<Arc<T>> {
        self.scopes
            .insert(TypeId::of::<T>(), Arc::new(scope))
            .map(|arc| arc.downcast::<T>().unwrap())
    }

    /// Removes a scope of specified type from `Scope`.
    /// This may be useful in middlewares, that change protocol identifier or in inter-middleware
    /// communication.
    #[inline]
    pub fn remove<T: Any + Sync + Send>(&mut self) -> Option<Arc<T>> {
        self.scopes
            .remove(&TypeId::of::<T>())
            .map(|arc| arc.downcast::<T>().unwrap())
    }

    /// Consumes `self` and returns new `Scope` with scope inserted in internal map.
    #[inline]
    pub fn with_scope<T: Any + Sync + Send>(mut self, scope: T) -> Self {
        let _ = self.insert(scope);
        self
    }
}

/// Structure for representing an event, that is sent or received over Server- and AppStreams.
/// Event contains family, that could be used for matching inside servers, apps and middlewares.
///
/// ASGI equivalent: [Event](https://asgi.readthedocs.io/en/latest/specs/main.html#events)
#[derive(Clone, Debug)]
pub struct Event {
    family: Cow<'static, str>,
    event: Arc<dyn Any + Sync + Send>,
}

impl Event {
    /// Creates new event of specified family and type.
    #[inline]
    pub fn new<T: Any + Sync + Send>(family: Cow<'static, str>, event: T) -> Self {
        Self {
            family,
            event: Arc::new(event),
        }
    }

    /// Returns event family.
    #[inline]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// Returns reference-counted event of concrete type.
    #[inline]
    pub fn get<T: Any + Sync + Send>(&self) -> Option<Arc<T>> {
        self.event.clone().downcast::<T>().ok()
    }

    /// Returns reference event of concrete type
    #[inline]
    pub fn get_ref<T: Any + Sync + Send>(&self) -> Option<&T> {
        self.event.downcast_ref::<T>()
    }
}

/// Trait, representing a Service, that is used to handle connections.
///
/// It can handle multiple connections at simultaneously.
///
/// ASGI equivalent: [Application](https://asgi.readthedocs.io/en/latest/specs/main.html#applications)
pub trait Service<ServerStream: Stream<Item = Event>> {
    type AppStream: Stream<Item = Event> + Send + Unpin;
    type Error: std::error::Error;

    /// Main function of a service.
    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error>;
}

#[derive(Default)]
struct TypeIdHasher {
    value: u64,
}

impl Hasher for TypeIdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(bytes.len(), 8);
        let _ = bytes
            .try_into()
            .map(|array| self.value = u64::from_ne_bytes(array));
    }
}
