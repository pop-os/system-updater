// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::utils;

pub async fn update(conn: &zbus::Connection) {
    const SOURCE: &str = "nix";

    if !utils::command_exists("nix-env") {
        return;
    }

    let upgrade = &["nix-env", "--upgrade"];
    let prune = &["nix-collect-garbage", "-d"];

    if let Err(why) = utils::async_commands(&[upgrade, prune]).await {
        utils::error_handler(conn, SOURCE, why).await;
    }
}
