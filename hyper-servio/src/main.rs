use hyper::service::Service;
use hyper::{Body, Request, Response, Server};

use futures::lock::Mutex as AsyncMutex;
use futures::StreamExt;
use servio::{AsgiService, Hello, HttpRequestEvent, HttpScope, EVENT_HTTP_REQUEST, PROTOCOL_HTTP};
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

type Counter = i32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(MakeSvc::new());
    println!("Listening on http://{}", addr);

    server.await?;
    Ok(())
}

struct WrapperFuture;

// struct WrapperService<T, ServerStream>
// where
//     T: servio::AsgiService<ServerStream>,
//     ServerStream: futures_core::Stream<Item = servio::Event>,
// {
//     inner: T,
//     _phantom: PhantomData<ServerStream>,
// }
//
// impl<T, ServerStream> Service<Request<Body>> for WrapperService<T, ServerStream>
// where
//     T: servio::AsgiService<ServerStream>,
//     ServerStream: futures_core::Stream<Item = servio::Event>,
// {
//     type Response = Response<Body>;
//     type Error = hyper::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
//
//     fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
//         Poll::Ready(Ok(()))
//     }
//
//     fn call(&mut self, req: Request<Body>) -> Self::Future {
//         let http_scope = HttpScope::default();
//
//         let scope = servio::Scope {
//             protocol: PROTOCOL_HTTP.into(),
//             scope: Arc::new(Box::new(http_scope)),
//         };
//
//         let app_stream = self.inner.call(scope /* ServerStream */);
//     }
// }

struct Svc {
    inner: Arc<AsyncMutex<Hello>>,
}

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let inner = self.inner.clone();

        let fut = async move {
            let http_scope = HttpScope::default();

            let mut guard = inner.lock().await;
            let scope = servio::Scope {
                protocol: PROTOCOL_HTTP.into(),
                scope: Arc::new(Box::new(http_scope)),
            };
            let mut appstream = guard.call(scope, futures::stream::empty()).unwrap();

            let event = appstream.next().await.unwrap();
            if event.event_type == EVENT_HTTP_REQUEST {
                let x = event.event.downcast::<HttpRequestEvent>().unwrap();

                return Ok(Response::builder().body(x.body.into()).unwrap());
            }

            // let body = Body::wrap_stream()
            Err(io::ErrorKind::Unsupported.into())
        };

        Box::pin(fut)
    }
}

struct MakeSvc {
    inner: Arc<AsyncMutex<Hello>>,
}

impl MakeSvc {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AsyncMutex::new(Hello {})),
        }
    }
}

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let fut = async move {
            Ok(Svc {
                inner: self.inner.clone(),
            })
        };

        Box::pin(fut)
    }
}
