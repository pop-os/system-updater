// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use chrono::NaiveTime;
use clokwerk::{AsyncScheduler, Interval as ClokwerkInterval, Job};
use config::{Interval, Schedule};
use pop_system_updater::config;
use pop_system_updater::dbus::PopService;
use pop_system_updater::dbus::{
    server::{self, Server},
    Event, IFACE,
};
use postage::mpsc::{self, Sender};
use postage::prelude::*;
use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use zbus::Connection;

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

impl Default for Service {
    fn default() -> Self {
        Service {
            updating: Arc::default(),
        }
    }
}

// Compile-time reference counter which can be split in half.
type Full<T> = static_rc::StaticRc<T, 3, 3>;

pub async fn run() {
    info!("initiating system service");
    crate::signal_handler::init();

    let (sender, mut receiver) = mpsc::channel(4);

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

    // The rescheduler closure will reload the service's scheduler from the `Config`.
    let reschedule = |schedule: &Schedule, sender: Sender<Event>| {
        let mut scheduler = AsyncScheduler::new();

        scheduler
            .every(match schedule.interval {
                Interval::Monday => ClokwerkInterval::Monday,
                Interval::Tuesday => ClokwerkInterval::Tuesday,
                Interval::Wednesday => ClokwerkInterval::Wednesday,
                Interval::Thursday => ClokwerkInterval::Thursday,
                Interval::Friday => ClokwerkInterval::Friday,
                Interval::Saturday => ClokwerkInterval::Saturday,
                Interval::Sunday => ClokwerkInterval::Sunday,
                Interval::Weekdays => ClokwerkInterval::Weekday,
            })
            .at_time(NaiveTime::from_hms(
                schedule.hour as u32,
                schedule.minute as u32,
                0,
            ))
            .run(move || {
                let mut sender = sender.clone();
                async move {
                    info!("initiating scheduled system update");
                    let _ = sender.send(Event::AutoUpdate).await;
                }
            });

        scheduler
    };

    // Create a full reference of the scheduler.
    let scheduler = config.schedule.as_ref().map(|schedule| reschedule(schedule, sender.clone()));

    let scheduler = Full::new(RefCell::new(scheduler));

    // Create two halves of the full reference to be given to the two futures below.
    let (scheduler1, scheduler2) = Full::split::<2, 1>(scheduler);

    let sig_duration = std::time::Duration::from_secs(1);

    let _ = std::thread::spawn({
        let mut sender = sender.clone();
        move || {
            async_io::block_on(async move {
                loop {
                    async_io::Timer::after(sig_duration).await;
                    if crate::signal_handler::status().is_some() {
                        info!("Found interrupt");
                        let _ = sender.send(Event::Exit).await;
                        break;
                    }
                }
            })
        }
    });

    let mut sender2 = sender.clone();

    futures::join!(
        restart_session_services(),
        // Check for updates every 12 hours.
        async move {
            loop {
                let _ = sender2.send(Event::Update).await;
                async_io::Timer::after(std::time::Duration::from_secs(60 * 60 * 12)).await;
            }
        },
        // Process events from the scheduler, which are sent to the event handler.
        async move {
            loop {
                // Updates should trigger no later than 1 minute after time elapsed.
                if let Some(scheduler) = scheduler2.borrow_mut().as_mut() {
                    scheduler.run_pending().await;
                }

                async_io::Timer::after(std::time::Duration::from_secs(60)).await;
            }
        },
        // The event handler, which processes all requests from DBus and the scheduler.
        async move {
            info!("listening for events");
            while let Some(event) = receiver.recv().await {
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

                        *scheduler1.borrow_mut() = if enable {
                            config.schedule.as_ref().map(|schedule| reschedule(schedule, sender.clone()))
                        } else {
                            None
                        };

                        let config = config.clone();
                        let task = smol::spawn(async move {
                            config::write_system_config(&config).await;
                            info!("system configuration file updated");
                        });

                        task.detach();
                    }

                    Event::SetSchedule(schedule) => {
                        info!("Changing scheduling to {:?}", schedule);

                        config.schedule = schedule;

                        *scheduler1.borrow_mut() = config.schedule.as_ref().map(|schedule| reschedule(schedule, sender.clone()));

                        let config = config.clone();
                        let task = smol::spawn(async move {
                            config::write_system_config(&config).await;
                            info!("system configuration file updated");
                        });

                        task.detach();
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
