// Copyright 2021 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::path::{Path, PathBuf};

pub const SYSTEM_PATH: &str = "/etc/pop-system-updater/config.ron";
pub const LOCAL_CACHE: &str = ".cache/pop-system-updater/cache.ron";
pub const LOCAL_PATH: &str = ".config/pop-system-updater/config.ron";

#[derive(Clone, Debug, Default, Deserialize, Serialize, zbus::Type)]
pub struct Cache {
    pub last_update: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, zbus::Type)]
pub struct LocalCache {
    pub last_update: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// If it should automatically update when updates are available.
    pub auto_update: bool,

    /// When updates should be scheduled, if updates should be scheduled for.
    pub schedule: Option<Schedule>,
}

impl Config {
    #[must_use]
    pub const fn default_schedule() -> Schedule {
        Schedule {
            interval: Interval::Weekdays,
            hour: 22,
            minute: 0,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            auto_update: false,
            schedule: Some(Config::default_schedule()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, zbus::Type)]
pub struct Local {
    pub enabled: bool,
    pub notification_frequency: Frequency,
}

impl Default for Local {
    fn default() -> Self {
        Self {
            enabled: true,
            notification_frequency: Frequency::Weekly,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, zbus::Type)]
#[repr(u32)]
pub enum Frequency {
    Weekly = 0,
    Daily = 1,
    Monthly = 2,
}

#[derive(Clone, Debug, Deserialize, Serialize, zbus::Type)]
pub struct Schedule {
    pub interval: Interval,
    pub hour: u8,
    pub minute: u8,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Deserialize_repr, Serialize_repr, zbus::Type)]
pub enum Interval {
    Monday = 1,
    Tuesday = 1 << 1,
    Wednesday = 1 << 2,
    Thursday = 1 << 3,
    Friday = 1 << 4,
    Saturday = 1 << 5,
    Sunday = 1 << 6,
    Weekdays = 1 << 7,
}

pub async fn load_session() -> Local {
    load(&session_path()).await
}

pub async fn load_session_cache() -> LocalCache {
    load(&session_cache_path()).await
}

pub async fn load_system() -> Config {
    load(Path::new(SYSTEM_PATH)).await
}

async fn load<T: Default + DeserializeOwned + Serialize>(path: &Path) -> T {
    let file;
    if let Ok(file_) = tokio::fs::read_to_string(path).await {
        file = file_;
        match ron::from_str::<T>(&file) {
            Ok(config) => return config,
            Err(why) => {
                tracing::error!("failed to read config: {}", why);
            }
        }
    }

    T::default()
}

pub fn session_path() -> PathBuf {
    std::env::home_dir().expect("NO HOME").join(LOCAL_PATH)
}

pub fn session_cache_path() -> PathBuf {
    std::env::home_dir().expect("NO HOME").join(LOCAL_CACHE)
}
