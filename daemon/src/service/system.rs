// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use anyhow::Context;
use async_cron_scheduler::{Job, JobId, Scheduler};
use chrono::Local;
use config::{Interval, Schedule};
use flume::Sender;
use futures::StreamExt;
use pop_system_updater::config::{self, Config};
use pop_system_updater::dbus::PopService;
use pop_system_updater::dbus::{
    server::{self, Server},
    Event, IFACE,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use zbus::Connection;

pub struct Service {
    updating: Arc<AtomicBool>,
    update_task: Option<JoinHandle<()>>,
    update_job: Option<JobId>,
    when_available_queue: Option<JoinHandle<()>>,
    scheduler: Scheduler<Local>,
}

impl Service {
    async fn auto_update(&mut self, connection: &zbus::Connection, sender: Sender<Event>) {
        if self.update_task.is_some() {
            info!("already performing an update");
            return;
        }

        info!("system update initiated");
        self.updating.store(true, Ordering::SeqCst);

        let connection = connection.clone();
        let updating = self.updating.clone();

        self.update_task = Some(tokio::task::spawn(async move {
            let _ = futures::join!(
                crate::package_managers::apt::update(connection.clone()),
                crate::package_managers::flatpak::update(connection.clone()),
                crate::package_managers::fwupd::update(connection.clone()),
                crate::package_managers::nix::update(&connection),
                crate::package_managers::snap::update(connection.clone())
            );

            updating.store(false, Ordering::SeqCst);
            let _ = sender.send_async(Event::UpdateComplete).await;
            info!("system update complete");
        }));
    }

    async fn check_for_updates(&self) {
        if self.update_task.is_some() {
            info!("already performing an update");
            return;
        }

        info!("checking for system updates");
        apt_cmd::lock::apt_lock_wait().await;
        crate::package_managers::apt::update_package_lists().await;
        info!("check for system updates complete");
    }

    async fn repair(&self, connection: &zbus::Connection) {
        if self.update_task.is_some() {
            info!("already performing an update");
            return;
        }

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

    fn schedule_when_available(&mut self, sender: &Sender<Event>) {
        if let Some(id) = self.update_job.take() {
            self.scheduler.remove(id);
        }

        if let Some(task) = self.when_available_queue.take() {
            task.abort();
        }

        self.update_job = Some(auto_job(&mut self.scheduler, sender));
    }

    async fn update_notification(&self, connection: &zbus::Connection) {
        let response = |ctx| async move {
            Server::updates_available(&ctx, crate::package_managers::updates_are_available().await)
                .await
        };

        server::context(connection, response).await;
    }

    fn update_scheduler(&mut self, config: &Config, sender: &Sender<Event>) {
        if let Some(id) = self.update_job.take() {
            self.scheduler.remove(id);
        }

        if let Some(task) = self.when_available_queue.take() {
            task.abort();
        }

        if config.auto_update {
            if let Some(ref conf) = config.schedule {
                self.update_job = Some(schedule_job(&mut self.scheduler, conf, sender));
            } else {
                let sender = sender.clone();
                self.when_available_queue = Some(tokio::task::spawn(async move {
                    info!("scheduling when available in 60 seconds");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    let _ = sender.send_async(Event::ScheduleWhenAvailable).await;
                }));
            }
        }
    }
}

// Watches for an interrupt signal.
async fn interrupt_handler(sender: Sender<Event>) {
    let sig_duration = std::time::Duration::from_secs(1);

    loop {
        tokio::time::sleep(sig_duration).await;
        if crate::signal_handler::status().is_some() {
            info!("Found interrupt");
            let _ = sender.send_async(Event::Exit).await;
            break;
        }
    }
}

// Check for updates every 12 hours.
async fn scheduled_check(sender: Sender<Event>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60 * 12));

    loop {
        interval.tick().await;
        let _ = sender.send_async(Event::CheckForUpdates).await;
    }
}

pub async fn run() -> anyhow::Result<()> {
    info!("initiating system service");
    crate::signal_handler::init();

    let (sender, receiver) = flume::bounded(1);

    let updating = Arc::new(AtomicBool::new(false));

    let connection = Connection::system()
        .await
        .context("failed to initialize dbus connection")?;

    connection
        .object_server()
        .at(
            IFACE,
            Server {
                updating: updating.clone(),
                service: PopService {
                    sender: sender.clone(),
                },
            },
        )
        .await
        .context("failed to serve service")?;

    connection
        .request_name("com.system76.SystemUpdater")
        .await
        .map_err(|why| match why {
            zbus::Error::NameTaken => anyhow::anyhow!("system service is already active"),
            other => anyhow::anyhow!("could not register system service: {}", other),
        })?;

    info!("DBus connection established");

    let mut config = config::load_system().await;

    let (scheduler, scheduler_service) = Scheduler::<Local>::launch(tokio::time::sleep);

    let mut service = Service {
        updating,
        update_job: None,
        update_task: None,
        when_available_queue: None,
        scheduler,
    };

    service.update_scheduler(&config, &sender);

    futures::join!(
        scheduler_service,
        interrupt_handler(sender.clone()),
        restart_session_services(),
        scheduled_check(sender.clone()),
        // The event handler, which processes all requests from DBus and the scheduler.
        async move {
            info!("listening for events");
            while let Ok(event) = receiver.recv_async().await {
                info!("received event: {:?}", event);
                match event {
                    Event::CheckForUpdates => {
                        service.check_for_updates().await;
                        service.update_notification(&connection).await;
                    }

                    Event::Repair => service.repair(&connection).await,

                    Event::ScheduleWhenAvailable => service.schedule_when_available(&sender),

                    Event::Update => service.auto_update(&connection, sender.clone()).await,

                    Event::UpdateComplete => service.update_task = None,

                    Event::SetAutoUpdate(enable) => {
                        info!("setting auto-update mode to {}", enable);

                        config.auto_update = enable;

                        service.update_scheduler(&config, &sender);

                        let config = config.clone();
                        tokio::spawn(async move {
                            config::write_system(&config).await;
                            info!("system configuration file updated");
                        });
                    }

                    Event::SetSchedule(schedule) => {
                        info!("Changing scheduling to {:?}", schedule);

                        config.schedule = schedule;

                        service.update_scheduler(&config, &sender);

                        let config = config.clone();
                        tokio::spawn(async move {
                            config::write_system(&config).await;
                            info!("system configuration file updated");
                        });
                    }

                    Event::Exit => {
                        info!("shutting down");
                        std::process::exit(0);
                    }
                }
            }

            info!("stop listening for events");
        }
    );

    Ok(())
}

/// Ensures that session services are always updated and restarted along with this service.
async fn restart_session_services() {
    info!("restarting any session services");
    futures::stream::iter(crate::accounts::user_names())
        .for_each_concurrent(None, |user| async {
            let accounts_service_file = ["/var/lib/AccountsService/users/", &user].concat();
            if !Path::new(&accounts_service_file).exists() {
                return;
            }

            let user = user;

            let _res = crate::utils::async_commands(&[&[
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

fn auto_job(scheduler: &mut Scheduler<Local>, sender: &Sender<Event>) -> JobId {
    info!("scheduling every 12 hours");
    tokio::spawn({
        let sender = sender.clone();
        async move {
            let _ = sender.send_async(Event::Update).await;
        }
    });

    let sender = sender.clone();
    scheduler.insert(Job::cron("0 0 */12 * * *").unwrap(), move |_| {
        let sender = sender.clone();
        tokio::spawn(async move {
            let _ = sender.send_async(Event::Update).await;
        });
    })
}

fn schedule_job(
    scheduler: &mut Scheduler<Local>,
    schedule: &Schedule,
    sender: &Sender<Event>,
) -> JobId {
    info!("scheduling for {:?}", schedule);
    let sender = sender.clone();
    scheduler.insert(Job::cron(&*cron_expression(schedule)).unwrap(), move |_| {
        let sender = sender.clone();
        tokio::spawn(async move {
            let _ = sender.send_async(Event::Update).await;
        });
    })
}

fn cron_expression(schedule: &Schedule) -> String {
    let days: &str = match schedule.interval {
        Interval::Sunday => "SUN",
        Interval::Monday => "MON",
        Interval::Tuesday => "TUE",
        Interval::Wednesday => "WED",
        Interval::Thursday => "THU",
        Interval::Friday => "FRI",
        Interval::Saturday => "SAT",
        Interval::Weekdays => "1-5",
    };

    let minute = schedule.minute.min(59);
    let hour = schedule.hour.min(23);

    let expression = format!("0 {minute} {hour} * * {days}");
    info!("setting cron expression {}", expression);
    expression
}
