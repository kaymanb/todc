#![allow(dead_code, unused_imports)]
use std::net::{IpAddr, Ipv4Addr};

use hyper::server::conn::http1;
use hyper::Uri;
use hyper_util::rt::TokioIo;
use turmoil::net::TcpListener;
use turmoil::{Builder, Sim};

use todc_net::abd_95::AtomicRegister;

mod abd_95 {
    mod common;
    #[cfg(feature = "turmoil")]
    mod linearizability;
    #[cfg(feature = "turmoil")]
    mod local;
    #[cfg(feature = "turmoil")]
    mod read;
    #[cfg(feature = "turmoil")]
    mod write;
}

const SERVER_PREFIX: &str = "server";
const PORT: u32 = 9999;

async fn serve(register: AtomicRegister<u32>) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, register).await {
                println!("Internal Server Error: {:?}", err);
            }
        });
    }
}

fn simulate_servers<'a>(n: usize) -> (Sim<'a>, Vec<AtomicRegister<u32>>) {
    let mut sim = Builder::new().build();

    let mut registers = Vec::new();

    let neighbors: Vec<Uri> = (0..n)
        .map(|i| {
            format!("http://{SERVER_PREFIX}-{i}:{PORT}")
                .parse()
                .unwrap()
        })
        .collect();

    for i in 0..n {
        let mut neighbors = neighbors.clone();
        neighbors.remove(i);
        let register: AtomicRegister<u32> = AtomicRegister::new(neighbors);
        let name = format!("{SERVER_PREFIX}-{i}");
        let register_clone = register.clone();
        sim.host(name, move || serve(register_clone.clone()));
        registers.push(register);
    }
    (sim, registers)
}
