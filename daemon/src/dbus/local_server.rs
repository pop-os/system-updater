// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use super::PopService;
use super::{Frequency, LocalEvent};
use crate::config::LocalConfig;

pub struct LocalServer {
    pub config: LocalConfig,
    pub service: PopService<LocalEvent>,
}

#[rustfmt::skip]
#[dbus_interface(name = "com.system76.SystemUpdater.Local")]
impl LocalServer {
    /// Get the frequency that the notification prompt will show.
    async fn notification_frequency(&mut self) -> Frequency {
        self.config.notification_frequency
    }

    /// Change the frequency that the notification prompt is shown.
    async fn set_notification_frequency(
        &mut self,
        frequency: Frequency,
    ) -> zbus::fdo::Result<()> {
        self.config.notification_frequency = frequency;

        let _ = futures::join!(
            crate::config::write_session_config(&self.config),
            self.service.send(LocalEvent::UpdateConfig(self.config.clone()))
        );

        Ok(())
    }
}
