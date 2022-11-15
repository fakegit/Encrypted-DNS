use crate::cli::Args;
use crate::local::UdpListener;
use crate::upstream::HttpsClient;
use clap::Parser;
use std::process::ExitCode;
use tracing::{error, Level};

mod bootstrap;
mod cache;
mod cli;
mod common;
mod error;
mod local;
mod upstream;

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        upstream_address,
        local_address,
        local_port,
        upstream_port,
        verbose,
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

    let https_client = match HttpsClient::new(upstream_address, upstream_port).await {
        Ok(https_client) => https_client,
        Err(error) => {
            error!("{}", error);
            return ExitCode::FAILURE;
        }
    };

    let udp_listener = match UdpListener::new(local_address, local_port, https_client).await {
        Ok(udp_listener) => udp_listener,
        Err(error) => {
            error!("{}", error);
            return ExitCode::FAILURE;
        }
    };
    udp_listener.listen().await;

    ExitCode::SUCCESS
}
