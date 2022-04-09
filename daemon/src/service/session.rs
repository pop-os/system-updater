// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use config::{Frequency, LocalCache, LocalConfig};
use pop_system_updater::config;
use pop_system_updater::dbus::PopService;
use pop_system_updater::dbus::{
    client::ClientProxy, local_server::LocalServer, LocalEvent, IFACE_LOCAL,
};
use postage::mpsc;
use postage::prelude::*;
use std::time::{Duration, SystemTime};
use zbus::Connection;

pub async fn run() -> anyhow::Result<()> {
    let system_connection = Connection::system()
        .await
        .context("could not initiate connection to service")?;

    let _system_proxy = ClientProxy::new(&system_connection)
        .await
        .context("could not get proxy from connection")?;

    let mut config = config::load_session_config().await;
    let (mut sender, mut receiver) = mpsc::channel(1);

    let connection = Connection::session()
        .await
        .expect("failed to initiate session connection");

    connection
        .object_server()
        .at(
            IFACE_LOCAL,
            LocalServer {
                config: config.clone(),
                service: PopService {
                    sender: sender.clone(),
                },
            },
        )
        .await
        .expect("failed to serve service");

    connection
        .request_name("com.system76.SystemUpdater.Local")
        .await
        .expect("failed to request session name");

    let cache = &mut config::load_session_cache().await;

    const SECONDS_IN_DAY: u64 = 60 * 60 * 24;

    fn last_update_time_exceeded(config: &LocalConfig, cache: &LocalCache, now: u64) -> bool {
        if cache.last_update > now {
            return true;
        }

        match config.notification_frequency {
            Frequency::Daily => cache.last_update + SECONDS_IN_DAY <= now,
            Frequency::Weekly => cache.last_update + SECONDS_IN_DAY * 7 <= now,
            Frequency::Monthly => cache.last_update + SECONDS_IN_DAY * 30 <= now,
        }
    }

    async fn check_for_updates(config: &LocalConfig, cache: &mut LocalCache) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        if !last_update_time_exceeded(config, cache, now) {
            return;
        }

        cache.last_update = now;
        let f1 = config::write_session_cache(cache);
        let f2 = async {
            if crate::package_managers::updates_are_available().await {
                info!("displaying notification of available updates");
                crate::notify::updates_available().await;
            }
        };

        futures::join!(f1, f2);
    }

    check_for_updates(&config, cache).await;

    let scheduler = async move {
        let _ = sender.send(LocalEvent::CheckUpdates).await;
        async_io::Timer::after(Duration::from_secs(SECONDS_IN_DAY)).await;
    };

    let event_loop = async move {
        while let Some(event) = receiver.recv().await {
            debug!("{:?}", event);
            match event {
                LocalEvent::CheckUpdates => check_for_updates(&config, cache).await,
                LocalEvent::UpdateConfig(conf) => {
                    config = conf;
                    check_for_updates(&config, cache).await;
                }
            }
        }
    };

    futures::join!(scheduler, event_loop);

    Ok(())
}
