// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use crate::config::Frequency;

#[dbus_proxy(
    default_service = "com.system76.SystemUpdater.Local",
    interface = "com.system76.SystemUpdater.Local",
    default_path = "/com/system76/SystemUpdater/Local"
)]
pub trait LocalClient {
    fn notification_frequency(&mut self) -> zbus::Result<Frequency>;
    fn set_notification_frequency(&mut self, frequency: Frequency) -> zbus::Result<()>;
}
