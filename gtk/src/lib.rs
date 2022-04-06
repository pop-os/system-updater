// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

#[macro_use]
extern crate cascade;

pub(crate) mod bsb;
pub(crate) mod dialog;
mod localize;
pub(crate) mod proxy;
pub(crate) mod utils;
mod widget;

pub use self::widget::SettingsWidget;
