// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use zbus::dbus_proxy;

#[dbus_proxy(
    interface = "io.elementary.appcenter",
    default_path = "/io/elementary/appcenter"
)]
trait ElementaryAppcenter {
    fn get_component_from_desktop_id(&self, desktop_id: &str) -> zbus::Result<String>;

    fn install(&self, component_id: &str) -> zbus::Result<()>;

    fn search_components(&self, query: &str) -> zbus::Result<Vec<String>>;

    fn uninstall(&self, component_id: &str) -> zbus::Result<()>;

    fn update(&self, component_id: &str) -> zbus::Result<()>;

    fn show_updates(&self) -> zbus::Result<()>;
}
