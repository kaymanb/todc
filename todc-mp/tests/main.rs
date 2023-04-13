use std::net::{IpAddr, Ipv4Addr};
use std::str;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::Uri;
use turmoil::net::TcpListener;
use turmoil::Builder;

use todc_mp::{echo, fetch_url};

#[test]
fn responds_with_echo() {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let mut sim = Builder::new().build();

    sim.host("server1", move || async move {
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
    });

    sim.host("server2", move || async move {
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
    });

    sim.client("client", async move {
        let url = Uri::from_static("http://server1:9999/register");
        let result = fetch_url(url).await.unwrap();
        assert_eq!(str::from_utf8(&result)?, "Echo!");

        Ok(())
    });

    sim.run().unwrap();
}
