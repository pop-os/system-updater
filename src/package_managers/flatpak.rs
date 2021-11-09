// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use crate::utils;
use futures::StreamExt;
use std::path::Path;

pub async fn update(conn: zbus::Connection) {
    const SOURCE: &str = "flatpak";

    if !utils::command_exists(SOURCE) {
        return;
    }

    let system = async {
        info!("{}: updating software for system", SOURCE);
        let refresh = &[SOURCE, "update", "--noninteractive"];
        let prune = &[SOURCE, "remove", "--unused", "--noninteractive"];
        let repair = &[SOURCE, "repair"];

        if utils::async_commands(&[refresh, prune]).await.is_err() {
            if let Err(why) = utils::async_commands(&[repair, refresh, prune]).await {
                utils::error_handler(&conn, SOURCE, why).await;
            }
        }
        info!("{}: updated software for system", SOURCE);
    };

    let users = async {
        futures::stream::iter(crate::accounts::user_names())
            .for_each_concurrent(None, |user| async {
                let accounts_service_file = ["/var/lib/AccountsService/users/", &user].concat();
                if !Path::new(&accounts_service_file).exists() {
                    return;
                }

                let user = user;
                info!("{}: updating software for {}", SOURCE, user);
                let refresh = &[
                    "runuser",
                    "-u",
                    &user,
                    "--",
                    SOURCE,
                    "update",
                    "--noninteractive",
                ];

                let prune = &[
                    "runuser",
                    "-u",
                    &user,
                    "--",
                    SOURCE,
                    "remove",
                    "--unused",
                    "--noninteractive",
                ];

                let repair = &["runuser", "-u", &user, "--", SOURCE, "repair", "--user"];

                if utils::async_commands(&[refresh, prune]).await.is_err() {
                    if let Err(why) = utils::async_commands(&[repair, refresh, prune]).await {
                        utils::error_handler(&conn, SOURCE, why).await;
                    }
                }

                info!("{}: updated software for {}", SOURCE, user);
            })
            .await;
    };

    futures::join!(system, users);
}
