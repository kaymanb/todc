use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::Incoming;
use hyper::http::StatusCode;
use hyper::server::conn::http1;
use hyper::{Request, Response, Uri};
use hyper_util::rt::TokioIo;
use rand::rngs::StdRng;
use rand::{thread_rng, Rng, SeedableRng};
use serde_json::Value as JSON;
use turmoil::net::{TcpListener, TcpStream};
use turmoil::{Builder, Sim};

use todc_net::register::abd_95::AtomicRegister;

pub const SERVER_PREFIX: &str = "server";
pub const PORT: u32 = 9999;

type FetchResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Simulate n replicates of a register.
pub fn simulate_servers<'a>(n: usize) -> (Sim<'a>, Vec<AtomicRegister<u32>>) {
    let sim = Builder::new().build();
    simulate_registers(n, sim)
}

/// Simulate n replicas of a register with a fixed RNG seed.
pub fn simulate_servers_with_seed<'a>(n: usize) -> (Sim<'a>, Vec<AtomicRegister<u32>>, u64) {
    let seed: u64 = thread_rng().gen();
    let rng = StdRng::seed_from_u64(seed);
    let sim = Builder::new().build_with_rng(Box::new(rng));
    let (sim, registers) = simulate_registers(n, sim);
    (sim, registers, seed)
}

/// Submits a GET request to the URL.
pub async fn get(url: Uri) -> FetchResult<Response<Incoming>> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let io = TokioIo::new(TcpStream::connect(addr).await?);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err}");
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .body(empty())?;

    let res = sender.send_request(req).await?;
    Ok(res)
}

/// Submits a POST request, with a JSON body, to the URL.
pub async fn post(url: Uri, body: JSON) -> FetchResult<Response<Incoming>> {
    let host = url.host().expect("uri has no host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{host}:{port}");
    let io = TokioIo::new(TcpStream::connect(addr).await?);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err}");
        }
    });

    let authority = url.authority().unwrap().clone();

    let req = Request::builder()
        .uri(url)
        .header(hyper::header::HOST, authority.as_str())
        .method("POST")
        .body(full(body))?;

    let res = sender.send_request(req).await?;
    Ok(res)
}

/// Adds n register instances to the simulation.
fn simulate_registers(n: usize, mut sim: Sim) -> (Sim, Vec<AtomicRegister<u32>>) {
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

/// Serve a register as a service.
async fn serve(register: AtomicRegister<u32>) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let addr = (IpAddr::from(Ipv4Addr::UNSPECIFIED), 9999);
    let listener = TcpListener::bind(addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let register = register.clone();
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, register).await {
                println!("Error Serving Connection: {:?}", err);
            }
        });
    }
}

/// Returns an empty response body.
fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

/// Returns a JSON response body.
fn full(value: JSON) -> BoxBody<Bytes, hyper::Error> {
    Full::<Bytes>::new(Bytes::from(value.to_string()))
        .map_err(|never| match never {})
        .boxed()
}

#[test]
fn invalid_route_responds_not_found() {
    let (mut sim, _) = simulate_servers(3);

    sim.client("client", async move {
        let url = Uri::from_static("http://server-0:9999/register/foo/bar");
        let response = get(url).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        Ok(())
    });
    sim.run().unwrap();
}

/// Asserts that a pair of reads that are concurrent with a write will return
/// the appropriate values.
///
/// This particular scenario is alluded to by Attiya, Bar-Noy, and Dolev
/// [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869) when providing
/// intuition for why a `read` operation must announce its values to all others
/// prior to returning.
///
/// > Informally, this announcement is needed since, otherwise, it is possible
/// > for a read operation to return the label of a write operation that is
/// > concurrent with it, and for a later read operation to return an earlier
/// > label.
///
/// In the test, we manipulate the network in order to simulate a scenario
/// where the first read _does_ return the value (i.e. label) of the write
/// operation it is concurrent with, and the the second concurrent must as
/// well, despite the later not having any direct communication with the
/// process doing the writing.
#[test]
fn pair_of_reads_with_concurrent_write_respond_correctly() {
    const NUM_SERVERS: usize = 5;
    const VALUE: u32 = 123;

    let (mut sim, registers) = simulate_servers(NUM_SERVERS);
    sim.set_max_message_latency(Duration::from_millis(1));

    let register_0 = registers[0].clone();
    sim.client("concurrent-writer", async move {
        // Initially, server-0 is isolated in the network. In particular,
        // server-1 and server-2 will not recieve information about the written
        // value until messages from server-0 are released.
        turmoil::hold("server-0", "server-1");
        turmoil::hold("server-0", "server-2");
        turmoil::hold("server-0", "server-3");
        turmoil::hold("server-0", "server-4");

        register_0.write(VALUE).await.unwrap();
        Ok(())
    });

    let register_1 = registers[1].clone();
    let register_2 = registers[2].clone();
    sim.client("reader", async move {
        // First, release messages between server-0 to server-1 and wait for
        // the concurrent write to be delivered.
        turmoil::release("server-0", "server-1");
        std::thread::sleep(Duration::from_secs(1));

        // Then, perform a read on server-1.
        let read_value = register_1.read().await.unwrap();
        assert_eq!(read_value, VALUE);

        // Next, hold messages between server-1 and server-2, preventing server-2
        // from asking for information about the value returned by the ealier read.
        turmoil::hold("server-1", "server-2");

        // Perform a read on server-2.
        let read_value = register_2.read().await.unwrap();
        assert_eq!(read_value, VALUE);

        // Finally, release all messeges from server-0, allow the write to complete.
        turmoil::release("server-0", "server-2");
        turmoil::release("server-0", "server-3");
        turmoil::release("server-0", "server-4");
        Ok(())
    });

    sim.run().unwrap();
}
