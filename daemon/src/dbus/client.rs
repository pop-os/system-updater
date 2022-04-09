// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::config::Schedule;

#[dbus_proxy(
    interface = "com.system76.SystemUpdater",
    default_service = "com.system76.SystemUpdater",
    default_path = "/com/system76/SystemUpdater"
)]
pub trait Client {
    fn auto_update_set(&mut self, enable: bool) -> zbus::Result<()>;

    fn check_for_updates(&mut self) -> zbus::Result<()>;

    fn is_updating(&mut self) -> zbus::Result<bool>;

    fn repair(&mut self) -> zbus::Result<()>;

    fn update_scheduling_disable(&mut self) -> zbus::Result<()>;

    fn update_scheduling_set(&mut self, schedule: Schedule) -> zbus::Result<()>;

    fn update_system(&mut self) -> zbus::Result<()>;

    #[dbus_proxy(signal)]
    fn error(&self, why: &str) -> zbus::Result<()>;

    #[dbus_proxy(signal)]
    fn progress(&self, source: &str, percent: u8) -> zbus::Result<()>;
}
