// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::{Event, PopService};
use crate::config::Schedule;
use std::future::Future;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use zbus::SignalContext;

pub struct Server {
    pub updating: Arc<AtomicBool>,
    pub service: PopService<Event>,
}

#[rustfmt::skip]
#[dbus_interface(name = "com.system76.SystemUpdater")]
impl Server {
    async fn auto_update_set(&mut self, enable: bool) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetAutoUpdate(enable))
    }

    /// Check if any updates are available to install.
    async fn check_for_updates(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::CheckForUpdates)
    }

    /// Check if a system update is currently being performed.
    async fn is_updating(&mut self) -> bool {
        self.updating.load(Ordering::SeqCst)
    }

    async fn repair(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::Repair)
    }

    async fn update_scheduling_disable(&mut self) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetSchedule(None))
    }

    async fn update_scheduling_set(&mut self, schedule: Schedule) -> zbus::fdo::Result<()> {
        self.service.send(Event::SetSchedule(Some(schedule)))
    }

    /// Initiates a system update.
    async fn update_system(&mut self) -> zbus::fdo::Result<()> {
        if !self.updating.load(Ordering::SeqCst) {
            self.service.send(Event::Update)?;
        }

        Ok(())
    }

    #[dbus_interface(signal)]
    pub async fn error(ctx: &SignalContext<'_>, source: &str, why: &str) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn progress(ctx: &SignalContext<'_>, source: &str, percent: u8) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn repair_err(ctx: &SignalContext<'_>, why: &str) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn repair_ok(ctx: &SignalContext<'_>) -> zbus::Result<()>;

    #[dbus_interface(signal)]
    pub async fn updates_available(ctx: &SignalContext<'_>, available: bool) -> zbus::Result<()>;
}

pub async fn context<'a, C, F>(conn: &zbus::Connection, future: C)
where
    C: FnOnce(SignalContext<'static>) -> F + 'a,
    F: Future<Output = zbus::Result<()>> + 'a,
{
    if let Ok(iface) = conn
        .object_server()
        .interface::<_, Server>(super::IFACE)
        .await
    {
        if let Err(why) = future(iface.signal_context().to_owned()).await {
            error!("context failed with {:?}", why);
        }
    }
}
