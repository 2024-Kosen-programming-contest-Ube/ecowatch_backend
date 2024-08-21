use anyhow::Result;
use dotenvy;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

mod database;
mod handlers;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("Failed to read .env file");

    database::init().await;

    let port = {
        let port_string: String = env::var("PORT").expect("PORT must be set");
        port_string.parse::<u16>().expect("PORT must be u16")
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

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
