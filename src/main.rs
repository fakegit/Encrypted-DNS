use crate::cli::Args;
use crate::tcp::LocalTcpListener;
use crate::udp::LocalUdpListener;
use crate::upstream::HttpsClient;
use clap::Parser;
use std::process::ExitCode;
use tokio::join;
use tracing::{error, Level};

mod bootstrap;
mod cache;
mod cli;
mod common;
mod error;
mod tcp;
mod udp;
mod upstream;

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        upstream_address,
        local_address,
        local_port,
        upstream_port,
        verbose,
        cache,
    } = cli::Args::parse();

    if verbose {
        tracing_subscriber::fmt()
            .with_max_level(Level::INFO)
            .with_target(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(Level::WARN)
            .with_target(false)
            .init();
    }

    let https_client = match HttpsClient::new(upstream_address, upstream_port, cache).await {
        Ok(https_client) => https_client,
        Err(error) => {
            error!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let udp_listener = match LocalUdpListener::new(
        local_address.clone(),
        local_port,
        https_client.clone(),
    )
    .await
    {
        Ok(udp_listener) => udp_listener,
        Err(error) => {
            error!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let tcp_listener = match LocalTcpListener::new(
        local_address.clone(),
        local_port,
        https_client.clone(),
    )
    .await
    {
        Ok(udp_listener) => udp_listener,
        Err(error) => {
            error!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    join!(tcp_listener.listen(), udp_listener.listen());
    ExitCode::SUCCESS
}
