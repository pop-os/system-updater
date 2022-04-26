// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

const APPCENTER: &str = "io.elementary.appcenter";

use std::time::Duration;

use notify_rust::{Hint, Notification, Urgency};

pub fn notify<F: FnOnce()>(summary: &str, body: &str, func: F) {
    let show_notification = || {
        Notification::new()
            .icon("distributor-logo")
            .summary(summary)
            .body(body)
            .action("default", "default")
            .hint(Hint::Urgency(Urgency::Critical))
            .show()
    };

    let mut notification = show_notification();

    while notification.is_err() {
        std::thread::sleep(Duration::from_secs(1));

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

pub fn updates_available() {
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
    );
}
