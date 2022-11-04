pub mod buffer;
mod logger;
pub mod response;
pub mod routers;

pub use logger::*;
use servio_service::Service;

pub type BoxService<'a, ServerStream, AppStream, Error> =
    Box<dyn Service<ServerStream, AppStream = AppStream, Error = Error> + Send + 'a>;
