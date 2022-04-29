// Copyright 2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[tokio::main]
async fn main() {
    pop_system_updater::appcenter::show_updates().await;
}
