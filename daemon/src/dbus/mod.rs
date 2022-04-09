// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub mod client;
pub mod local_client;
pub mod local_server;
pub mod server;

use crate::config::{Frequency, LocalConfig, Schedule};
use postage::prelude::Sink;

// Where this service's interface is being served at.
pub const IFACE: &str = "/com/system76/SystemUpdater";
pub const IFACE_LOCAL: &str = "/com/system76/SystemUpdater/Local";

#[derive(Debug)]
pub enum Event {
    AutoUpdate,
    CheckForUpdates,
    Exit,
    Repair,
    Update,
    SetSchedule(Option<Schedule>),
    SetAutoUpdate(bool),
}

#[derive(Debug)]
pub enum LocalEvent {
    CheckUpdates,
    UpdateConfig(LocalConfig),
}

pub struct PopService<E> {
    pub sender: postage::mpsc::Sender<E>,
}

impl<E: std::fmt::Debug> PopService<E> {
    async fn send(&mut self, event: E) -> zbus::fdo::Result<()> {
        if let Err(why) = self.sender.send(event).await {
            Err(zbus::fdo::Error::Failed(format!("{}", why)))
        } else {
            Ok(())
        }
    }
}
