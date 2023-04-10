use std::net::{IpAddr, Ipv4Addr};
use std::str;

use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Uri};
use turmoil::net::{TcpListener, TcpStream};
use turmoil::Builder;

use todc_mp::echo;

#[test]
fn responds_with_echo() {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let mut sim = Builder::new().build();

    sim.host("server", move || {
        // TODO: Why this?
        async move {
            let listener = TcpListener::bind(addr).await?;
            loop {
                let (stream, _) = listener.accept().await?;
                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(stream, service_fn(echo))
                        .await
                    {
                        println!("error serving connection: {:?}", err);
                    }
                });
            }
        }
    });

    sim.client("client", async move {
        let url = Uri::from_static("http://server:9999");
        let host = url.host().expect("uri has no host");
        let port = url.port_u16().unwrap_or(80);
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(addr).await?;

        let (mut sender, conn) = hyper::client::conn::http1::handshake(stream).await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });

        let authority = url.authority().unwrap().clone();

        let req = Request::builder()
            .uri(url)
            .header(hyper::header::HOST, authority.as_str())
            .body(Empty::<Bytes>::new())?;

        let res = sender.send_request(req).await?;

        println!("Response: {}", res.status());

        let body = res.collect().await?.to_bytes();
        assert_eq!(str::from_utf8(&body)?, "Echo!");

        Ok(())
    });

    sim.run().unwrap();
}
