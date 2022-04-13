// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use pop_system_updater::config::{Config, Frequency};
use pop_system_updater::dbus::{client::ClientProxy, local_client::LocalClientProxy};
use postage::mpsc::{channel, Sender};
use postage::prelude::*;
use zbus::Connection;

#[derive(Debug)]
pub enum ProxyEvent {
    Exit,
    SetNotificationFrequency(Frequency),
    UpdateConfig(Config),
}

pub fn initialize_service() -> Sender<ProxyEvent> {
    let (tx, mut rx) = channel(1);

    let background_process = async move {
        let connection = match Connection::system().await {
            Ok(connection) => connection,
            Err(why) => {
                eprintln!("could not initiate connection to service: {}", why);
                return;
            }
        };

        let mut proxy = match ClientProxy::new(&connection).await {
            Ok(proxy) => proxy,
            Err(why) => {
                eprintln!("could not get proxy from connection: {}", why);
                return;
            }
        };

        let session_connection = match Connection::session().await {
            Ok(connection) => connection,
            Err(why) => {
                eprintln!("could not initiate connection to service: {}", why);
                return;
            }
        };

        let mut session_proxy = match LocalClientProxy::new(&session_connection).await {
            Ok(proxy) => proxy,
            Err(why) => {
                eprintln!("could not get proxy from connection: {}", why);
                return;
            }
        };

        while let Some(event) = rx.recv().await {
            match event {
                ProxyEvent::Exit => break,

                ProxyEvent::UpdateConfig(config) => {
                    if let Err(why) = proxy.auto_update_set(config.auto_update).await {
                        eprintln!("failed to change auto-update setting: {}", why);
                    }

                    let result = match config.schedule {
                        Some(schedule) => proxy.update_scheduling_set(schedule).await,
                        None => proxy.update_scheduling_disable().await,
                    };

                    if let Err(why) = result {
                        eprintln!("failed to change scheduling: {}", why);
                    }
                }

                ProxyEvent::SetNotificationFrequency(frequency) => {
                    if let Err(why) = session_proxy.set_notification_frequency(frequency).await {
                        eprintln!("failed to update notification frequency: {:?}", why);
                    }
                }
            }
        }
    };

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(background_process);
    });

    tx
}
