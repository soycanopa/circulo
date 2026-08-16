use std::net::SocketAddr;
use std::sync::Arc;

use circulo_adapter_fake::FakeAdapter;
use circulo_daemon::{listen_addr, router, AppState, DEFAULT_ADDR};
use circulo_persist::Store;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("circulo-daemon: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let raw = std::env::var("CIRCULO_DAEMON_ADDR").ok();
    let addr: SocketAddr = listen_addr(raw.as_deref())?;
    let db = std::env::var("CIRCULO_DB_PATH").ok();
    let store = match db {
        Some(path) => Store::open(path)?,
        None => Store::open_default()?,
    };
    let state = AppState::new(store, Arc::new(FakeAdapter::new()));
    let listener = TcpListener::bind(addr).await?;
    println!("circulo-daemon listening on http://{addr} (default {DEFAULT_ADDR})");
    axum::serve(listener, router(state)).await?;
    Ok(())
}
