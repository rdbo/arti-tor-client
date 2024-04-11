use anyhow::Context;
use arti_client::{TorClient, TorClientConfig};
use http::{uri::Scheme, Uri};
use http_body_util::{BodyExt, Empty};
use hyper::{
    body::{Body, Bytes},
    Request,
};
use hyper_util::rt::TokioIo;
use std::str::FromStr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Setting up tor client config...");
    let client_config = TorClientConfig::default();

    println!("Creating bootstrapped tor client...");
    let client = TorClient::create_bootstrapped(client_config).await?;

    println!("Parsing URI...");
    let uri = Uri::from_str("http://icanhazip.com")?;
    let host = uri.host().context("Missing host in URI")?;
    let port = match (uri.port_u16(), uri.scheme()) {
        (Some(port), _) => port,
        (_, Some(scheme)) if *scheme == Scheme::HTTPS => 443,
        _ => 80,
    };

    println!("Connecting tor client to '{}:{}'", host, port);
    let stream = client
        .connect(("icanhazip.com", port))
        .await
        .context("Failed to connect tor client to specified address")?;

    println!("Setting up HTTP...");
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    println!("Sending GET request to specified address...");
    let request = Request::builder()
        .method("GET")
        .header("Host", host)
        .uri(uri)
        .body(Empty::<Bytes>::new())?;
    let mut resp = sender.send_request(request).await?;

    println!("Response status: {}", resp.status());
    println!("Response headers: {:#?}", resp.headers());
    println!("Response body:");
    while let Some(next) = resp.frame().await {
        let frame = next?;
        if let Some(chunk) = frame.data_ref() {
            io::stdout().write_all(&chunk).await?;
        }
    }
    println!();

    Ok(())
}
