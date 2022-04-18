// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

const APPCENTER: &str = "io.elementary.appcenter";

use notify_rust::{Notification, Timeout};

pub fn notify<F: FnOnce()>(summary: &str, body: &str, func: F) {
    Notification::new()
        .icon("distributor-logo")
        .summary(summary)
        .body(body)
        .action("default", "default")
        .timeout(Timeout::Never)
        .show()
        .expect("failed to show desktop notification")
        .wait_for_action(|action| match action {
            "default" => func(),
            "__closed" => (),
            _ => (),
        });
}

pub async fn updates_available() {
    restart_appcenter().await;

    notify(
        "System updates are available to install",
        "Click here to update the system",
        || {
            tokio::spawn(async move {
                let _ = tokio::process::Command::new(APPCENTER)
                    .arg("-u")
                    .status()
                    .await;
            });
        },
    )
}

/// Restart the appcenter to force that the packagekit cache is refreshed.
async fn restart_appcenter() {
    let _ = tokio::process::Command::new("killall")
        .arg(APPCENTER)
        .status()
        .await;

    tokio::spawn(async move {
        tokio::process::Command::new(APPCENTER)
            .arg("-s")
            .status()
            .await
    });
}
