// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub async fn show_updates() {
    let connection = match zbus::Connection::session().await {
        Ok(conn) => conn,
        Err(why) => {
            eprintln!("could not get connection to dbus session: {}", why);
            return;
        }
    };

    let proxy = match pop_shop_zbus::ElementaryAppcenterProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(why) => {
            eprintln!("could not connect to io.elementary.appcenter: {}", why);
            return;
        }
    };

    let mut spawned = false;

    loop {
        match proxy.show_updates().await {
            Ok(()) => return,
            Err(why) => {
                if spawned {
                    eprintln!(
                        "io.elementary.appcenter show-updates dbus method failed: {:?}",
                        why
                    );

                    let _res = tokio::process::Command::new("io.elementary.appcenter")
                        .arg("--show-updates")
                        .spawn();

                    return;
                }

                // Kill the process if it's running.
                let _res = tokio::process::Command::new("sh")
                    .args(&["-c", "kill $(pidof io.elementary.appcenter)"])
                    .status()
                    .await;

                // Start a new service process.
                let _res = tokio::process::Command::new("io.elementary.appcenter")
                    .arg("--silent")
                    .spawn();

                spawned = true;

                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        }
    }
}
