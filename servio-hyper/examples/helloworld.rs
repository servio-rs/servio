use http::StatusCode;
use hyper::server::conn::http1;
use servio_hyper::Servio2Hyper;
use servio_util::response::PlainTextResponse;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();

    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);

    loop {
        let (stream, client) = listener.accept().await?;

        let service = PlainTextResponse::new(StatusCode::OK, "Hello, world!".into());
        let hyper_service = Servio2Hyper::new(service, Some(addr), Some(client));

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, hyper_service)
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
