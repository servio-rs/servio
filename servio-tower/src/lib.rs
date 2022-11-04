use futures_core::stream::BoxStream;
use futures_core::Stream;
use http::Response;
use http_body::Body;
use servio_http::http::HttpScope;
use servio_service::{Event, Scope};
use std::io;

struct Tower2Servio<S> {
    inner: S,
}

struct ResBodyStream<B: Body> {
    inner: B,
}

impl<ReqBody, ResBody, ServerStream, S> servio_service::Service<ServerStream> for Tower2Servio<S>
where
    ServerStream: Stream<Item = Event>,
    S: tower::Service<http::Request<ReqBody>, Response = Response<ResBody>>,
{
    type AppStream = BoxStream<'static, Event>;
    type Error = io::Error;

    fn call(
        &mut self,
        scope: Scope,
        server_events: ServerStream,
    ) -> Result<Self::AppStream, Self::Error> {
        let mut req = http::Request::new("".to_string());

        let http_scope = scope.get::<HttpScope>().unwrap();

        *req.method_mut() = http_scope.method.clone();
        *req.uri_mut() = http_scope.uri.clone();
        *req.version_mut() = http_scope.version;
        *req.headers_mut() = http_scope.headers.clone();

        req.extensions_mut().insert(scope);

        self.inner.call(req);
        todo!()
    }
}

//
// impl<ServerStream, TowerServiceT, BodyT> servio_service::Service<ServerStream>
//     for Tower2Servio<TowerServiceT>
// where
//     ServerStream: Stream<Item = Event>,
//     TowerServiceT: tower::Service<http::Request<BodyT>>,
//     BodyT: http_body::Body,
// {
//     type AppStream = ();
//     type Error = ();
//
//     fn call(
//         &mut self,
//         scope: Scope,
//         server_events: ServerStream,
//     ) -> Result<Self::AppStream, Self::Error> {
//         todo!()
//     }
// }
