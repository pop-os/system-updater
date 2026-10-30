// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[macro_use]
extern crate tracing;

mod accounts;
mod daemon;
mod notify;
mod package_managers;
mod service;
mod signal_handler;
mod utils;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use `env RUST_LOG=debug` for debug logging.
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // If root then system service, else local session service.
    let main_future = async move {
        let effective_uid = users::get_effective_uid();
        if effective_uid == 0 {
            crate::service::system::run().await
        } else if accounts::is_desktop_account(effective_uid) {
            crate::service::session::run().await
        } else {
            Ok(())
        }
    };

    // Spawns main future on tokio runtime for best async performance.
    tokio::spawn(main_future)
        .await
        .context("failed to spawn on tokio runtime")?
}
