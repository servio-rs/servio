use std::borrow::Cow;

pub const PROTOCOL_WEBSOCKET: &str = "websocket";

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct WebsocketScope {
    pub subprotocols: Vec<Cow<'static, str>>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum WebSocketEvent {
    Accept(Accept),
    Close,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct Accept {
    pub subprotocol: Cow<'static, str>,
    pub headers: http::HeaderMap,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Close {
    pub code: u16,
    pub reason: Cow<'static, str>,
}

impl Default for Close {
    fn default() -> Self {
        Self {
            code: 1000,
            reason: Default::default(),
        }
    }
}
