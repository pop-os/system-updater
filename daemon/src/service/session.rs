// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use config::{Frequency, LocalCache, LocalConfig};
use flume::Sender;
use pop_system_updater::config;
use pop_system_updater::dbus::PopService;
use pop_system_updater::dbus::{
    client::ClientProxy, local_server::LocalServer, LocalEvent, IFACE_LOCAL,
};
use std::time::{Duration, SystemTime};
use tokio::task::JoinHandle;
use zbus::Connection;

pub async fn run() -> anyhow::Result<()> {
    let system_connection = Connection::system()
        .await
        .context("could not initiate connection to service")?;

    let _system_proxy = ClientProxy::new(&system_connection)
        .await
        .context("could not get proxy from connection")?;

    let mut config = config::load_session_config().await;
    let (sender, receiver) = flume::bounded(1);

    let connection = Connection::session()
        .await
        .context("failed to initiate session connection")?;

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
        .context("failed to serve service")?;

        connection
            .request_name("com.system76.SystemUpdater.Local")
            .await
            .map_err(|why| match why {
                zbus::Error::NameTaken => anyhow::anyhow!("user service is already active"),
                other => anyhow::anyhow!("could not register user service: {}", other)
            })?;

    let mut state = State {
        cache: config::load_session_cache().await,
        schedule_handle: tokio::spawn(update_on(
            sender.clone(),
            Duration::from_secs(SECONDS_IN_DAY),
        )),
        sender,
    };

    state.check_for_updates(&config).await;

    while let Ok(event) = receiver.recv_async().await {
        match event {
            LocalEvent::CheckUpdates => state.check_for_updates(&config).await,
            LocalEvent::UpdateConfig(conf) => {
                config = conf;
                state.check_for_updates(&config).await;
            }
        }
    }

    Ok(())
}

pub struct State {
    cache: LocalCache,
    schedule_handle: JoinHandle<()>,
    sender: Sender<LocalEvent>,
}

impl State {
    async fn check_for_updates(&mut self, config: &LocalConfig) {
        self.schedule_handle.abort();

        if !config.enabled {
            info!("notifications disabled");
            return;
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        let next_update = next_update(config, &self.cache);

        if next_update > now {
            let next = next_update - now;
            info!("next update in {} seconds", next);
            let future = update_on(self.sender.clone(), Duration::from_secs(next));
            self.schedule_handle = tokio::spawn(future);
            return;
        }

        self.schedule_handle = tokio::spawn(update_on(
            self.sender.clone(),
            Duration::from_secs(SECONDS_IN_DAY),
        ));

        self.cache.last_update = now;
        let f1 = config::write_session_cache(&self.cache);
        let f2 = async {
            if crate::package_managers::updates_are_available().await {
                info!("displaying notification of available updates");
                crate::notify::updates_available().await;
            }
        };

        futures::join!(f1, f2);
    }
}

const SECONDS_IN_DAY: u64 = 60 * 60 * 24;

fn next_update(config: &LocalConfig, cache: &LocalCache) -> u64 {
    match config.notification_frequency {
        Frequency::Daily => cache.last_update + SECONDS_IN_DAY,
        Frequency::Weekly => cache.last_update + SECONDS_IN_DAY * 7,
        Frequency::Monthly => cache.last_update + SECONDS_IN_DAY * 30,
    }
}

async fn update_on(sender: Sender<LocalEvent>, duration: Duration) {
    tokio::time::sleep(duration).await;
    let _ = sender.send_async(LocalEvent::CheckUpdates).await;
}
