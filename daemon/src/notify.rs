// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

const APPCENTER: &str = "io.elementary.appcenter";

use std::time::Duration;

use notify_rust::{Hint, Notification, Timeout, Urgency};

pub async fn notify<F: FnOnce()>(summary: &str, body: &str, func: F) {
    let show_notification = || {
        Notification::new()
            .icon("distributor-logo")
            .summary(summary)
            .body(body)
            .action("default", "default")
            .timeout(Timeout::Never)
            .hint(Hint::Resident(true))
            .hint(Hint::Urgency(Urgency::Critical))
            .show()
    };

    let mut notification = show_notification();

    while notification.is_err() {
        tokio::time::sleep(Duration::from_secs(1)).await;

        notification = show_notification();
    }

    notification
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
        "Click here to view available updates",
        || {
            tokio::spawn(async move {
                let _ = tokio::process::Command::new(APPCENTER)
                    .arg("-u")
                    .status()
                    .await;
            });
        },
    )
    .await
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
