use bytes::Bytes;
use std::borrow::Cow;

pub const PROTOCOL_WEBSOCKET: &str = "websocket";
pub const EVENT_WEBSOCKET: &str = "websocket";

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct WebSocketScope {
    pub subprotocols: Vec<Cow<'static, str>>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum WebSocketEvent {
    Connect(Connect),
    Accept(Accept),
    TextFrame(TextFrame),
    BinaryFrame(BinaryFrame),
    Close(Close),
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct Connect {}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct Accept {
    pub subprotocol: Option<Cow<'static, str>>,
    pub headers: http::HeaderMap,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct TextFrame {
    pub data: String,
}

#[non_exhaustive]
#[derive(Default, Clone, Debug)]
pub struct BinaryFrame {
    pub data: Bytes,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Close {
    pub code: u16,
    pub reason: Option<Cow<'static, str>>,
}

impl Default for Close {
    fn default() -> Self {
        Self {
            code: 1000,
            reason: Default::default(),
        }
    }
}
