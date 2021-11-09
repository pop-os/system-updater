// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

#[macro_use]
extern crate cascade;

use gio::{prelude::*, ApplicationFlags};
use gtk::{prelude::*, Application};
use pop_system_updater_gtk::SettingsWidget;

pub const APP_ID: &str = "com.system76.UpgradeManager";

fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    glib::set_program_name(APP_ID.into());

    let application = Application::new(Some(APP_ID), ApplicationFlags::empty());

    application.connect_activate(|app| {
        if let Some(window) = app.window_by_id(0) {
            window.present();
        }
    });

    application.connect_startup(|app| {
        let widget = SettingsWidget::new();

        let headerbar = cascade! {
            gtk::HeaderBar::new();
            ..set_title(Some("Pop! System Update Scheduler"));
            ..set_show_close_button(true);
            ..show();
        };

        let _window = cascade! {
            gtk::ApplicationWindow::new(app);
            ..set_titlebar(Some(&headerbar));
            ..set_icon_name(Some("firmware-manager"));
            ..set_keep_above(true);
            ..set_window_position(gtk::WindowPosition::Center);
            ..add(cascade! {
                &widget.inner;
                // ..set_border_width(12);
                ..set_margin_top(24);
                ..set_halign(gtk::Align::Center);
            });
            ..show();
        };
    });

    application.run();
}
