// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use std::fs;

pub fn is_desktop_account(euid: u32) -> bool {
    match uid_min_max() {
        Ok((min, max)) => min <= euid && max >= euid,
        Err(_) => false,
    }
}

/// Users which are defined as being desktop accounts.
///
/// On Linux, this means the user ID is between `UID_MIN` and `UID_MAX` in `/etc/login.defs`.
pub fn user_names() -> Box<dyn Iterator<Item = String> + Send> {
    let (uid_min, uid_max) = match uid_min_max() {
        Ok(v) => v,
        Err(_) => return Box::new(std::iter::empty()),
    };

    Box::new((unsafe { users::all_users() }).filter_map(move |user| {
        if user.uid() >= uid_min && user.uid() <= uid_max {
            let name = user.name();
            if let Some(name) = name.to_str() {
                return Some(name.to_owned());
            }
        }

        None
    }))
}

/// The `UID_MIN` and `UID_MAX` values from `/etc/login.defs`.
pub fn uid_min_max() -> anyhow::Result<(u32, u32)> {
    let login_defs =
        fs::read_to_string("/etc/login.defs").context("could not read /etc/login.defs")?;

    let defs = whitespace_conf::parse(&login_defs);

    defs.get("UID_MIN")
        .zip(defs.get("UID_MAX"))
        .context("/etc/login.defs does not contain UID_MIN + UID_MAX")
        .and_then(|(min, max)| {
            let min = min.parse::<u32>().context("UID_MIN is not a u32 value")?;
            let max = max.parse::<u32>().context("UID_MAX is not a u32 value")?;
            Ok((min, max))
        })
}
