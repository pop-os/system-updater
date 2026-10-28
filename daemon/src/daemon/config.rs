use pop_system_updater_zbus::{
    Config, Local, LocalCache,
    config::{self, SYSTEM_PATH},
};
use serde::Serialize;
use std::path::Path;

pub async fn write_session(config: &Local) {
    write(&config::session_path(), config).await;
}

pub async fn write_session_cache(cache: &LocalCache) {
    write(&config::session_cache_path(), cache).await;
}

pub async fn write_system(config: &Config) {
    write(Path::new(SYSTEM_PATH), config).await;
}

async fn write<T: Serialize>(path: &Path, config: &T) {
    if let Some(parent) = path.parent() {
        let _res = tokio::fs::create_dir(parent).await;
    }

    let config = match ron::to_string(config) {
        Ok(config) => config,
        Err(why) => {
            tracing::error!("failed to serialize config: {}", why);
            return;
        }
    };

    if let Err(why) = tokio::fs::write(path, config.as_bytes()).await {
        tracing::error!("failed to write config file: {}", why);
    }
}
