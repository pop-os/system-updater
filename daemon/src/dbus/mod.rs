// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub mod client;
pub mod local_client;
pub mod local_server;
pub mod server;

use crate::config::{Frequency, LocalConfig, Schedule};

// Where this service's interface is being served at.
pub const IFACE: &str = "/com/system76/SystemUpdater";
pub const IFACE_LOCAL: &str = "/com/system76/SystemUpdater/Local";

#[derive(Debug)]
pub enum Event {
    CheckForUpdates,
    Exit,
    Repair,
    ScheduleWhenAvailable,
    SetSchedule(Option<Schedule>),
    SetAutoUpdate(bool),
    Update,
}

#[derive(Debug)]
pub enum LocalEvent {
    CheckUpdates,
    UpdateConfig(LocalConfig),
}

pub struct PopService<E> {
    pub sender: flume::Sender<E>,
}

impl<E: std::fmt::Debug> PopService<E> {
    async fn send(&mut self, event: E) -> zbus::fdo::Result<()> {
        if let Err(why) = self.sender.send_async(event).await {
            Err(zbus::fdo::Error::Failed(format!("{}", why)))
        } else {
            Ok(())
        }
    }
}
