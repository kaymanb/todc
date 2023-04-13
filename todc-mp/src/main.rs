use std::net::SocketAddr;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use tokio::net::TcpListener;

use todc_mp::echo;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service_fn(echo))
                .await
            {
                println!("Error serving connection: {err}");
            }
        });
    }
}
