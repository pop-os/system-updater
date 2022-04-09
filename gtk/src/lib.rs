// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

#[macro_use]
extern crate cascade;

pub(crate) mod bsb;
pub(crate) mod dialog;
mod localize;
pub(crate) mod proxy;
pub(crate) mod utils;
mod widget;

pub use self::widget::SettingsWidget;
