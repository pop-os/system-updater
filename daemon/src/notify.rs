// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::time::Duration;

use notify_rust::{Hint, Notification, Timeout, Urgency};

pub fn notify<F: FnOnce()>(summary: &str, body: &str, func: F) {
    let show_notification = || {
        Notification::new()
            .icon("distributor-logo")
            .summary(summary)
            .body(body)
            .action("default", "default")
            .hint(Hint::Urgency(Urgency::Normal))
            .hint(Hint::Resident(true))
            .timeout(Timeout::Never)
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
            "default" | "clicked" => func(),
            "__closed" => eprintln!("dismissed update notification"),
            _ => (),
        });
}

pub fn updates_available() {
    notify(
        "System updates are available to install",
        "Click here to view available updates",
        || {
            // TODO: open the updates page
            tokio::spawn(async move {
                _ = tokio::process::Command::new("cosmic-store").status().await;
            });
        },
    );
}
