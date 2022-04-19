// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use async_cron_scheduler::*;
use chrono::Local;
use config::{Interval, Schedule};
use flume::Sender;
use pop_system_updater::config;
use pop_system_updater::dbus::PopService;
use pop_system_updater::dbus::{
    server::{self, Server},
    Event, IFACE,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use zbus::Connection;

#[derive(Default)]
pub struct Service {
    updating: Arc<AtomicBool>,
}

impl Service {
    async fn auto_update(&self, connection: &zbus::Connection) {
        info!("system update initiated");
        self.updating.store(true, Ordering::SeqCst);

        let _ = futures::join!(
            crate::package_managers::apt::update(connection.clone()),
            crate::package_managers::flatpak::update(connection.clone()),
            crate::package_managers::fwupd::update(connection.clone()),
            crate::package_managers::nix::update(connection),
            crate::package_managers::snap::update(connection.clone())
        );

        self.updating.store(false, Ordering::SeqCst);
        info!("system update complete");
    }

    async fn check_for_updates(&self) {
        info!("checking for system updates");
        let _ = apt_cmd::lock::apt_lock_wait().await;
        let _ = crate::package_managers::apt::update_package_lists().await;
        info!("check for system updates complete");
    }

    async fn repair(&self, connection: &zbus::Connection) {
        info!("performing a system repair");

        let result = crate::package_managers::apt::repair().await;

        let response = |ctx| async move {
            match result {
                Ok(()) => Server::repair_ok(&ctx).await,
                Err(why) => Server::repair_err(&ctx, &why.to_string()).await,
            }
        };

        server::context(connection, response).await;

        info!("system repair attempt complete");
    }

    async fn update_notification(&self, connection: &zbus::Connection) {
        let response = |ctx| async move {
            Server::updates_available(&ctx, crate::package_managers::updates_are_available().await)
                .await
        };

        server::context(connection, response).await;
    }
}

pub async fn run() {
    info!("initiating system service");
    crate::signal_handler::init();

    let (sender, receiver) = flume::bounded(4);

    let service = Service::default();

    let connection = Connection::system()
        .await
        .expect("failed to initialize dbus connection");

    connection
        .object_server()
        .at(
            IFACE,
            Server {
                updating: service.updating.clone(),
                service: PopService {
                    sender: sender.clone(),
                },
            },
        )
        .await
        .expect("failed to serve service");

    connection
        .request_name("com.system76.SystemUpdater")
        .await
        .expect("failed to request name");

    info!("DBus connection established");

    let mut config = config::load_system_config().await;

    let (mut scheduler, scheduler_service) = Scheduler::<Local>::launch(tokio::time::sleep);

    let mut update_job: Option<JobId> = None;

    if let Some(ref conf) = config.schedule {
        update_job = Some(schedule_job(&mut scheduler, conf, &sender));
    };

    let sig_duration = std::time::Duration::from_secs(1);

    futures::join!(
        scheduler_service,
        {
            let sender = sender.clone();
            async move {
                loop {
                    tokio::time::sleep(sig_duration).await;
                    if crate::signal_handler::status().is_some() {
                        info!("Found interrupt");
                        let _ = sender.send(Event::Exit);
                        break;
                    }
                }
            }
        },
        restart_session_services(),
        // Check for updates every 12 hours.
        {
            let sender = sender.clone();
            async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(60 * 60 * 12));
                loop {
                    interval.tick().await;
                    let _ = sender.send(Event::Update);
                }
            }
        },
        // The event handler, which processes all requests from DBus and the scheduler.
        async move {
            info!("listening for events");
            while let Ok(event) = receiver.recv_async().await {
                info!("received event: {:?}", event);
                match event {
                    Event::AutoUpdate => {
                        if config.auto_update {
                            service.auto_update(&connection).await
                        }
                    }

                    Event::CheckForUpdates => {
                        service.check_for_updates().await;
                        service.update_notification(&connection).await;
                    }

                    Event::Repair => service.repair(&connection).await,

                    Event::Update => {
                        if config.schedule.is_none() {
                            service.auto_update(&connection).await;
                        } else {
                            service.check_for_updates().await;
                        }
                    }

                    Event::SetAutoUpdate(enable) => {
                        info!("setting auto-update mode to {}", enable);

                        config.auto_update = enable;
                        if let Some(id) = update_job.take() {
                            scheduler.remove(id);
                        }

                        if enable {
                            if let Some(ref conf) = config.schedule {
                                update_job = Some(schedule_job(&mut scheduler, conf, &sender));
                            }
                        }

                        let config = config.clone();
                        tokio::spawn(async move {
                            config::write_system_config(&config).await;
                            info!("system configuration file updated");
                        });
                    }

                    Event::SetSchedule(schedule) => {
                        info!("Changing scheduling to {:?}", schedule);

                        config.schedule = schedule;

                        if let Some(id) = update_job.take() {
                            scheduler.remove(id);
                        }

                        if let Some(ref conf) = config.schedule {
                            update_job = Some(schedule_job(&mut scheduler, conf, &sender));
                        }

                        let config = config.clone();
                        tokio::spawn(async move {
                            config::write_system_config(&config).await;
                            info!("system configuration file updated");
                        });
                    }

                    Event::Exit => {
                        info!("shutting down");
                        std::process::exit(1);
                    }
                }
            }

            info!("stop listening for events");
        }
    );
}

/// Ensures that session services are always updated and restarted along with this service.
async fn restart_session_services() {
    info!("restarting any session services");
    use futures::StreamExt;
    futures::stream::iter(crate::accounts::user_names())
        .for_each_concurrent(None, |user| async {
            let accounts_service_file = ["/var/lib/AccountsService/users/", &user].concat();
            if !Path::new(&accounts_service_file).exists() {
                return;
            }

            let user = user;

            let _ = crate::utils::async_commands(&[&[
                "runuser",
                "-u",
                &user,
                "--",
                "systemctl",
                "--user",
                "restart",
                "com.system76.SystemUpdater.Local",
            ]])
            .await;
        })
        .await;
}

fn schedule_job(
    scheduler: &mut Scheduler<Local>,
    schedule: &Schedule,
    sender: &Sender<Event>,
) -> JobId {
    info!("scheduling for {:?}", schedule);
    let sender = sender.clone();
    scheduler.insert(Job::cron(&*cron_expression(schedule)).unwrap(), move |_| {
        info!("UPDATE TRIGGERED");
        let _ = sender.send(Event::Update);
    })
}

fn cron_expression(schedule: &Schedule) -> String {
    let days: &str = match schedule.interval {
        Interval::Sunday => "0",
        Interval::Monday => "1",
        Interval::Tuesday => "2",
        Interval::Wednesday => "3",
        Interval::Thursday => "4",
        Interval::Friday => "5",
        Interval::Saturday => "6",
        Interval::Weekdays => "1-5",
    };

    let minute = schedule.minute.min(59);
    let hour = schedule.hour.min(23);

    format!("0 {minute} {hour} * * {days}")
}
