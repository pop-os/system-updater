// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

pub mod apt;
pub mod apt_pop;
pub mod flatpak;
pub mod nix;

pub mod fwupd {
    pub async fn update(conn: zbus::Connection) {
        use crate::utils;

        const SOURCE: &str = "fwupdmgr";

        if !utils::command_exists(SOURCE) {
            return;
        }

        if let Err(why) = utils::async_command(&[SOURCE, "refresh", "--force"]).await {
            utils::error_handler(&conn, SOURCE, why).await;
        }
    }
}

pub mod snap {
    pub async fn update(conn: zbus::Connection) {
        use crate::utils;

        const SOURCE: &str = "snap";

        info!("{}: updating software for system", SOURCE);

        if !utils::command_exists(SOURCE) {
            return;
        }

        if let Err(why) = utils::async_command(&[SOURCE, "refresh"]).await {
            utils::error_handler(&conn, SOURCE, why).await;
        }

        info!("{}: updated software for system", SOURCE);
    }
}

pub async fn updates_are_available() -> bool {
    // TODO: Flatpak
    if let Ok(packages) = apt::packages_to_fetch().await {
        return !packages.is_empty();
    }

    false
}
