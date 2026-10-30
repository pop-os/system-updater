// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::{Event, PopService};
use pop_system_updater_zbus::Schedule;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use zbus::object_server::SignalEmitter;

pub struct Server {
    pub updating: Arc<AtomicBool>,
    pub service: PopService<Event>,
}

#[zbus::interface(name = "com.system76.SystemUpdater")]
impl Server {
    async fn auto_update_set(&mut self, enable: bool) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetAutoUpdate(enable)).await
    }

    /// Check if any updates are available to install.
    async fn check_for_updates(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::CheckForUpdates).await
    }

    /// Check if a system update is currently being performed.
    async fn is_updating(&self) -> bool {
        self.updating.load(Ordering::SeqCst)
    }

    async fn repair(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::Repair).await
    }

    async fn update_scheduling_disable(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetSchedule(None)).await
    }

    async fn update_scheduling_set(&mut self, schedule: Schedule) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetSchedule(Some(schedule))).await
    }

    /// Initiates a system update.
    async fn update_system(&mut self) -> zbus::fdo::Result<()> {
        if !self.updating.load(Ordering::SeqCst) {
            self.service.send(Event::Update).await?;
        }

        Ok(())
    }

    #[zbus(signal)]
    pub async fn error(emitter: &SignalEmitter<'_>, source: &str, why: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn progress(
        emitter: &SignalEmitter<'_>,
        source: &str,
        percent: u8,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn repair_err(emitter: &SignalEmitter<'_>, why: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn repair_ok(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn updates_available(
        emitter: &SignalEmitter<'_>,
        available: bool,
    ) -> zbus::Result<()>;
}
