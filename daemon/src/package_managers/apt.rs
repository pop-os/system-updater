// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::utils;
use anyhow::Context;
use apt_cmd::fetch::{FetchEvents, PackageFetcher};
use apt_cmd::lock::apt_lock_wait;
use apt_cmd::request::Request as AptRequest;
use apt_cmd::{AptGet, AptMark, Dpkg};
use tokio::process::{Child, Command};
use std::process::Stdio;
use futures::Stream;
use futures::StreamExt;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

pub async fn update(conn: zbus::Connection) -> bool {
    const SOURCE: &str = "apt";

    if !utils::command_exists(SOURCE) {
        return false;
    }

    info!("performing system update with apt");

    let mut service_requires_update = false;

    if system_update(&mut service_requires_update).await.is_err() {
        if let Ok(release) = os_release::OS_RELEASE.as_ref() {
            if release.name == "Pop!_OS" {
                let _ = super::apt_pop::regenerate(&release.version_codename).await;
            }
        }

        let mut count = 0;
        while let Err(why) = repair().await {
            if count == 2 {
                utils::error_handler(&conn, SOURCE, why).await;
                return false;
            }

            count += 1;
        }
    }

    info!("{}: updated software for system", SOURCE);
    service_requires_update
}

pub async fn repair() -> anyhow::Result<()> {
    let _ = AptMark::new().hold(["pop-system-updater"]).await;

    apt_lock_wait().await;
    let apt_get_result = AptGet::new()
        .noninteractive()
        .fix_broken()
        .force()
        .allow_downgrades()
        .status()
        .await
        .context("failed to repair broken packages with `apt-get install -f`");

    apt_lock_wait().await;
    let dpkg_result = Dpkg::new()
        .configure_all()
        .status()
        .await
        .context("failed to configure packages with `dpkg --configure -a`");

    let _ = AptMark::new().unhold(["pop-system-updater"]).await;

    apt_get_result.and(dpkg_result)
}

async fn system_update(service_requires_update: &mut bool) -> anyhow::Result<()> {
    update_package_lists().await;

    info!("getting list of packages to update");
    let packages = packages_to_fetch()
        .await
        .context("could not get packages to fetch")?;

    let mut packages: Vec<&str> = packages.iter().map(String::as_str).collect();

    info!("fetching packages");
    let mut events = fetch(&packages).await.context("could not fetch packages")?;

    while let Some(_event) = events.next().await {}

    if let Some(id) = packages.iter().position(|&p| p == "pop-system-updater") {
        info!("service requires update");
        *service_requires_update = true;
        packages.swap_remove(id);
    }

    upgrade().await.context("could not upgrade packages")?;

    Ok(())
}

pub async fn update_package_lists() {
    info!("updating package lists");
    apt_lock_wait().await;
    let result = AptGet::new()
        .update()
        .await
        .context("could not `apt update` package lists");

    if let Err(why) = result {
        error!("potential issue with package lists configuration: {}", why);
    }
}

pub async fn packages_to_fetch() -> anyhow::Result<Vec<String>> {
    let _ = apt_lock_wait().await;

    let (mut child, packages) = upgradable_packages()
        .await
        .context("could not get system updates from apt")?;

    let packages = packages.collect::<Vec<String>>().await;

    info!("debian packages requiring updates: {}", packages.len());

    child
        .wait()
        .await
        .context("could not check for updates from apt")?;

    Ok(packages)
}

async fn uris(packages: &[&str]) -> anyhow::Result<HashSet<AptRequest>> {
    apt_lock_wait().await;

    AptGet::new()
        .noninteractive()
        .fetch_uris(&{
            let mut args = vec!["install"];
            args.extend_from_slice(packages);
            args
        })
        .await
        .context("failed to exec `apt-get install --print-uris`")?
        .context("failed to fetch package URIs from `apt-get install`")
}

pub async fn upgrade() -> anyhow::Result<()> {
    apt_lock_wait().await;

    let _ = AptMark::new().hold(["pop-system-updater"]).await;

    let result = AptGet::new()
        .noninteractive()
        .force()
        .allow_downgrades()
        .upgrade()
        .await
        .context("failed to install updates");

    let _ = AptMark::new().unhold(["pop-system-updater"]).await;

    result
}

/// Get a list of APT URIs to fetch for this operation, and then fetch them.
async fn fetch(packages: &[&str]) -> anyhow::Result<FetchEvents> {
    let uris = uris(packages)
        .await
        .context("failed to get URIs for apt packages")?;

    apt_lock_wait().await;
    let _lock_files = hold_apt_locks()?;

    const ARCHIVES: &str = "/var/cache/apt/archives/";
    const PARTIAL: &str = "/var/cache/apt/archives/partial/";

    const CONCURRENT_FETCHES: usize = 4;
    const DELAY_BETWEEN: u64 = 100;
    const RETRIES: u32 = 3;

    let client = isahc::HttpClient::new().expect("failed to create HTTP Client");

    // The system which fetches packages we send requests to
    let events = PackageFetcher::new(client)
        .concurrent(CONCURRENT_FETCHES)
        .delay_between(DELAY_BETWEEN)
        .retries(RETRIES)
        .fetch(
            futures::stream::iter(uris.into_iter().map(Arc::new)),
            Path::new(PARTIAL).into(),
            Path::new(ARCHIVES).into(),
        );

    Ok(events)
}

const DPKG_LOCK: &str = "/var/lib/dpkg/lock";
const LISTS_LOCK: &str = "/var/lib/apt/lists/lock";

fn hold_apt_locks() -> anyhow::Result<(File, File)> {
    let lists = File::open(LISTS_LOCK)
        .with_context(|| format!("failed to acquire lock for {}", LISTS_LOCK))?;

    let dpkg = File::open(DPKG_LOCK)
        .with_context(|| format!("failed to acquire lock for {}", DPKG_LOCK))?;

    Ok((lists, dpkg))
}

pub type Packages = Pin<Box<dyn Stream<Item = String>>>;

// Fetch all upgradeable debian packages from system apt repositories.
pub async fn upgradable_packages() -> anyhow::Result<(Child, Packages)> {
    let mut child = Command::new("apt-get")
        .args(&["full-upgrade", "--dry-run"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to launch `apt`")?;

    let stdout = child.stdout.take().unwrap();

    let stream = Box::pin(async_stream::stream! {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut buffer = String::new();

        while let Ok(read) = reader.read_line(&mut buffer).await {
            if read == 0 {
                break
            }

            let mut words = buffer.split_ascii_whitespace();
            if let Some("Inst") = words.next() {
                if let Some(package) = words.next() {
                    yield package.into();
                }
            }

            buffer.clear();
        }
    });

    Ok((child, stream))
}
