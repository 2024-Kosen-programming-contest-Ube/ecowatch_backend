use anyhow::Result;
use config::CONFIG;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod config;
mod database;
mod handlers;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    database::init().await;

    let addr = SocketAddr::from(([127, 0, 0, 1], CONFIG.port));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(handlers::route))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
