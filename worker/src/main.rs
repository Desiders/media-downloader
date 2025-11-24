use froodi::axum::setup_async_default;
use std::net::SocketAddr;
use tokio::sync::broadcast::{Receiver, Sender, channel};
use tonic::{
    service::Routes,
    transport::{self, Server},
};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _};

use crate::{
    config::{Config, Version, get_config_path},
    presentation::grpc::{
        api::version::{self, VersionServiceServer},
        test::{self, EchoServiceServer},
    },
    signal::shutdown_signal,
};

mod di_container;
mod signal;

pub mod adapters;
pub mod config;
pub mod entities;
pub mod interactors;
pub mod presentation;
pub mod utils;
pub mod value_objects;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let config_path = &*get_config_path();
    let config = Config::from_fs(config_path)?;
    let version = Version::from_env("CARGO_PKG_VERSION")?;

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::builder().parse_lossy(config.logging.dirs.as_ref()))
        .init();

    let addr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    info!("Listening on {addr}. Version: {version}");

    let container = di_container::init(config, version);

    let routes = Routes::default()
        .add_service(EchoServiceServer::new(test::Service))
        .add_service(VersionServiceServer::new(version::Service));
    let router = setup_async_default(routes.into_axum_router(), container);

    let (shutdown_tx, _) = channel(1);

    let (err, _) = tokio::join!(
        tokio::spawn(run_server(router.into(), addr, shutdown_tx.subscribe())),
        tokio::spawn(handle_shutdown(shutdown_tx))
    );
    err.unwrap().map_err(Into::into)
}

async fn run_server(routes: Routes, addr: SocketAddr, mut shutdown_rx: Receiver<()>) -> Result<(), transport::Error> {
    Server::builder()
        .add_routes(routes)
        .serve_with_shutdown(addr, async move {
            let _ = shutdown_rx.recv().await;
        })
        .await
}

async fn handle_shutdown(shutdown_tx: Sender<()>) {
    let () = shutdown_signal().await;
    let _ = shutdown_tx.send(());
}
