// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::path::{Path, PathBuf};
use zvariant::Type;

pub const SYSTEM_CACHE: &str = "/var/cache/pop-system-updater/cache.ron";
pub const SYSTEM_PATH: &str = "/etc/pop-system-updater/config.ron";
pub const LOCAL_CACHE: &str = ".cache/pop-system-updater/cache.ron";
pub const LOCAL_PATH: &str = ".config/pop-system-updater/config.ron";

#[derive(Clone, Debug, Default, Deserialize, Serialize, Type)]
pub struct Cache {
    pub last_update: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Type)]
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

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct LocalConfig {
    pub enabled: bool,
    pub notification_frequency: Frequency,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notification_frequency: Frequency::Weekly,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize, Type)]
#[repr(u32)]
pub enum Frequency {
    Weekly = 0,
    Daily = 1,
    Monthly = 2,
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct Schedule {
    pub interval: Interval,
    pub hour: u8,
    pub minute: u8,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Deserialize_repr, Serialize_repr, Type)]
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

pub async fn load_session_config() -> LocalConfig {
    load(&session_config_path()).await
}

pub async fn load_session_cache() -> LocalCache {
    load(&session_cache_path()).await
}

pub async fn load_system_config() -> Config {
    load(Path::new(SYSTEM_PATH)).await
}

pub async fn load_system_cache() -> Cache {
    load(Path::new(SYSTEM_CACHE)).await
}

pub async fn write_session_config(config: &LocalConfig) {
    write(&session_config_path(), config).await
}

pub async fn write_session_cache(cache: &LocalCache) {
    write(&session_cache_path(), cache).await
}

pub async fn write_system_config(config: &Config) {
    write(Path::new(SYSTEM_PATH), config).await
}

pub async fn write_system_cache(cache: &Cache) {
    write(Path::new(SYSTEM_CACHE), cache).await
}

async fn load<T: Default + DeserializeOwned + Serialize>(path: &Path) -> T {
    info!("loading config: {:?}", path);
    let file;
    if let Ok(file_) = tokio::fs::read_to_string(path).await {
        file = file_;
        match ron::from_str::<T>(&file) {
            Ok(config) => return config,
            Err(why) => {
                error!("failed to read config: {}", why);
            }
        }
    }

    let config = T::default();
    write(path, &config).await;
    config
}

async fn write<T: Serialize>(path: &Path, config: &T) {
    info!("writing config: {:?}", path);

    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir(parent).await;
    }

    let config = match ron::to_string(config) {
        Ok(config) => config,
        Err(why) => {
            error!("failed to serialize config: {}", why);
            return;
        }
    };

    if let Err(why) = tokio::fs::write(path, config.as_bytes()).await {
        error!("failed to write config file: {}", why);
    }
}

fn session_config_path() -> PathBuf {
    #[allow(deprecated)]
    std::env::home_dir().expect("NO HOME").join(LOCAL_PATH)
}

fn session_cache_path() -> PathBuf {
    #[allow(deprecated)]
    std::env::home_dir().expect("NO HOME").join(LOCAL_CACHE)
}
