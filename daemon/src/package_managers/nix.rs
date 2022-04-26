// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::utils;

pub async fn update(conn: &zbus::Connection) {
    const SOURCE: &str = "nix";

    const COMMANDS: &[&[&str]] = &[
        &["nix-channel", "--update"],
        &["nix-env", "--upgrade"],
        &["nix-collect-garbage", "-d"],
    ];

    for command in COMMANDS {
        if !utils::command_exists(command[0]) {
            return;
        }
    }

    if let Err(why) = utils::async_commands(COMMANDS).await {
        utils::error_handler(conn, SOURCE, why).await;
    }
}
