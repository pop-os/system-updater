// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::dialog::Dialog;
use crate::fl;
use crate::localize::localizer;
use crate::proxy::ProxyEvent;
use crate::utils::*;
use gtk::prelude::*;
use i18n_embed::DesktopLanguageRequester;
use pop_system_updater::config::{Config, Frequency, Interval};
use postage::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

pub struct SettingsWidget {
    pub inner: gtk::Widget,
}

impl SettingsWidget {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let localizer = localizer();
        let requested_languages = DesktopLanguageRequester::requested_languages();

        if let Err(error) = localizer.select(&requested_languages) {
            eprintln!(
                "Error while loading languages for pop-system-updater-gtk {}",
                error
            );
        }

        #[derive(Debug)]
        enum Event {
            AutomaticUpdatesToggled,
            ChangeNotificationSchedule,
            DialogComplete(Config),
            Exit,
        }

        let mut ptx = crate::proxy::initialize_service();

        let (tx, mut rx) = postage::mpsc::channel(1);

        let automatic_updates;
        let schedule_label;
        let schedule_description;
        let notification_schedule;

        let content = cascade! {
            gtk::ListBox::new();
            ..set_selection_mode(gtk::SelectionMode::None);
            ..set_header_func(Some(Box::new(separator_header)));
            ..add(&{
                automatic_updates = gtk::Switch::builder()
                    .valign(gtk::Align::Center)
                    .build();

                let label = gtk::Label::builder()
                    .label(&fl!("automatic-updates-label"))
                    .xalign(0.0)
                    .hexpand(true)
                    .vexpand(true)
                    .mnemonic_widget(&automatic_updates)
                    .build();

                cascade! {
                    option_container();
                    ..add(&label);
                    ..add(&automatic_updates);
                    ..show_all();
                }
            });
            ..add({
                let button_image = gtk::Image::builder()
                    .icon_name("go-next-symbolic")
                    .icon_size(gtk::IconSize::Button)
                    .build();

                schedule_label = gtk::Label::builder()
                    .label(&fl!("automatically-install-label"))
                    .xalign(0.0)
                    .hexpand(true)
                    .vexpand(true)
                    .build();

                schedule_description = gtk::Label::builder()
                    .xalign(0.0)
                    .valign(gtk::Align::Center)
                    .vexpand(true)
                    .build();

                let container = cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 4);
                    ..add(&schedule_description);
                    ..add(&button_image);
                };

                &cascade! {
                    option_container();
                    ..add(&schedule_label);
                    ..add(&container);
                    ..show_all();
                }
            });
            ..add(&{
                let label = gtk::Label::builder()
                    .label(&fl!("update-notifications-label"))
                    .xalign(0.0)
                    .hexpand(true)
                    .build();

                notification_schedule = cascade! {
                    gtk::ComboBoxText::new();
                    ..set_valign(gtk::Align::Center);
                    ..append_text(&fl!("schedule-daily"));
                    ..append_text(&fl!("schedule-weekly"));
                    ..append_text(&fl!("schedule-monthly"));
                };

                cascade! {
                    option_container();
                    ..add(&label);
                    ..add(&notification_schedule);
                    ..show_all();
                }
            });
            ..connect_destroy({
                let tx = tx.clone();
                let ptx = ptx.clone();
                move |_| {
                    let mut tx = tx.clone();
                    let mut ptx = ptx.clone();
                    glib_spawn(async move {
                        let f1 = tx.send(Event::Exit);
                        let f2 = ptx.send(ProxyEvent::Exit);

                        let _ = futures::join!(f1, f2);
                    });
                }
            });
        };

        let dialog_active = Rc::new(Cell::new(true));

        content.connect_row_activated(
            glib::clone!(@weak dialog_active, @strong tx, @strong ptx => move |_, row| {
                if dialog_active.get() && row.index() == 1 {
                    let tx = tx.clone();
                    let dialog = Dialog::new(row.upcast_ref::<gtk::Widget>(), move |conf| {
                        glib_send(tx.clone(), Event::DialogComplete(conf));
                    });

                    dialog.0.run();

                    unsafe {
                        dialog.0.destroy();
                    }
                }
            }),
        );

        let widget = Self {
            inner: option_frame(content.upcast_ref::<gtk::Widget>()).upcast::<gtk::Widget>(),
        };

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        glib_spawn(async move {
            let _context = runtime.enter();
            let update_schedule_description = |config: &Config| {
                let text: String = if config.auto_update {
                    if let Some(schedule) = config.schedule.as_ref() {
                        let (hour, am_pm) = if schedule.hour < 12 {
                            (schedule.hour + 1, fl!("time-am"))
                        } else {
                            (schedule.hour - 11, fl!("time-pm"))
                        };

                        format!(
                            "Update on {} at {:02}:{:02} {}",
                            match schedule.interval {
                                Interval::Monday => fl!("time-monday"),
                                Interval::Tuesday => fl!("time-tuesday"),
                                Interval::Wednesday => fl!("time-wednesday"),
                                Interval::Thursday => fl!("time-thursday"),
                                Interval::Friday => fl!("time-friday"),
                                Interval::Saturday => fl!("time-saturday"),
                                Interval::Sunday => fl!("time-sunday"),
                                Interval::Weekdays => fl!("time-weekdays"),
                            },
                            hour,
                            schedule.minute,
                            am_pm
                        )
                    } else {
                        fl!("update-when-available")
                    }
                } else {
                    fl!("off")
                };

                schedule_description.set_text(&text);
            };

            let change_scheduling_sensitivity = |config: &Config| {
                let label_ctx = schedule_label.style_context();
                let description_ctx = schedule_description.style_context();

                if config.auto_update {
                    label_ctx.remove_class("dim-label");
                    description_ctx.remove_class("dim-label");
                } else {
                    label_ctx.add_class("dim-label");
                    description_ctx.add_class("dim-label");
                }

                dialog_active.set(config.auto_update);
                update_schedule_description(config);
            };

            let (mut system_config, session_config) = futures::join!(
                pop_system_updater::config::load_system_config(),
                pop_system_updater::config::load_session_config()
            );

            automatic_updates.set_active(system_config.auto_update);
            change_scheduling_sensitivity(&system_config);

            notification_schedule.set_active(Some(session_config.notification_frequency as u32));

            automatic_updates.connect_changed_active(glib::clone!(@strong tx => move |_| {
                glib_send(tx.clone(), Event::AutomaticUpdatesToggled);
            }));

            notification_schedule.connect_changed(glib::clone!(@strong tx => move |_| {
                glib_send(tx.clone(), Event::ChangeNotificationSchedule);
            }));

            while let Some(event) = rx.recv().await {
                match event {
                    Event::ChangeNotificationSchedule => {
                        let _ = ptx
                            .send(ProxyEvent::SetNotificationFrequency(
                                match notification_schedule.active() {
                                    Some(0) => Frequency::Daily,
                                    Some(1) => Frequency::Weekly,
                                    _ => Frequency::Monthly,
                                },
                            ))
                            .await;
                    }

                    Event::DialogComplete(conf) => {
                        system_config = conf.clone();

                        change_scheduling_sensitivity(&conf);

                        let _ = ptx.send(ProxyEvent::UpdateConfig(conf)).await;
                    }

                    Event::AutomaticUpdatesToggled => {
                        system_config.auto_update = automatic_updates.is_active();

                        change_scheduling_sensitivity(&system_config);

                        let _ = ptx
                            .send(ProxyEvent::UpdateConfig(system_config.clone()))
                            .await;
                    }

                    Event::Exit => break,
                }
            }
        });

        widget
    }
}
