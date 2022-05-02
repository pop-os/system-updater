// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[macro_use]
extern crate cascade;

pub(crate) mod bsb;
pub(crate) mod dialog;
pub mod localize;
pub(crate) mod proxy;
pub(crate) mod utils;
mod widget;

pub use self::widget::SettingsWidget;

pub fn localize() {
    let localizer = localize::localizer();
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!(
            "Error while loading language for system-updater-gtk {}",
            error
        );
    }
}
