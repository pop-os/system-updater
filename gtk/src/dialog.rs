// SPDX-License-Identifier: MPL-2.0
// Copyright © 2021 System76

use crate::fl;
use crate::utils::*;
use gtk::prelude::*;
use pop_system_updater::config::{Config, Interval, Schedule};
use postage::prelude::*;
use std::rc::Rc;

pub struct Dialog(pub gtk::Dialog);

impl Dialog {
    pub fn new(widget: &gtk::Widget, func: impl Fn(Config) + 'static) -> Self {
        enum Event {
            Exit,
            UpdateConfig,
        }

        let when_available;
        let interval;
        let hour;
        let minute;
        let time_of_day;
        let schedule_label;

        let (tx, mut rx) = postage::mpsc::channel(1);

        let content = cascade! {
            gtk::ListBox::new();
            ..set_selection_mode(gtk::SelectionMode::None);
            ..set_header_func(Some(Box::new(separator_header)));
            ..add(&{
                when_available = gtk::Switch::builder()
                    .valign(gtk::Align::Center)
                    .build();

                let label = gtk::Label::builder()
                    .label(&fl!("update-when-available-label"))
                    .xalign(0.0)
                    .hexpand(true)
                    .vexpand(true)
                    .mnemonic_widget(&when_available)
                    .build();

                cascade! {
                    option_container();
                    ..attach(&label, 0, 0, 1, 1);
                    ..attach(&when_available, 1, 0, 1, 1);
                    ..show_all();
                }
            });
            ..add(&{
                interval = cascade! {
                    gtk::ComboBoxText::new();
                    ..set_valign(gtk::Align::Center);
                    ..append_text(&fl!("time-monday"));
                    ..append_text(&fl!("time-tuesday"));
                    ..append_text(&fl!("time-wednesday"));
                    ..append_text(&fl!("time-thursday"));
                    ..append_text(&fl!("time-friday"));
                    ..append_text(&fl!("time-saturday"));
                    ..append_text(&fl!("time-sunday"));
                    ..append_text(&fl!("time-weekdays"));
                };

                schedule_label = gtk::Label::builder()
                    .label(&fl!("schedule-label"))
                    .xalign(0.0)
                    .hexpand(true)
                    .vexpand(true)
                    .mnemonic_widget(&interval)
                    .build();

                hour = cascade! {
                    crate::bsb::BetterSpinButton::new(1, 12, 1, 3, 2);
                    ..set_valign(gtk::Align::Center);
                };

                minute = cascade! {
                    crate::bsb::BetterSpinButton::new(0, 59, 1, 10, 2);
                    ..set_valign(gtk::Align::Center);
                };

                time_of_day = cascade! {
                    gtk::ComboBoxText::new();
                    ..set_valign(gtk::Align::Center);
                    ..append_text(&fl!("time-am"));
                    ..append_text(&fl!("time-pm"));
                };

                let times = cascade! {
                    gtk::Box::new(gtk::Orientation::Horizontal, 4);
                    ..add(&interval);
                    ..add(&*hour);
                    ..add(&*minute);
                    ..add(&time_of_day);
                };

                cascade! {
                    option_container();
                    ..attach(&schedule_label, 0, 0, 1, 1);
                    ..attach(&times, 1, 0, 1, 1);
                    ..show_all();
                }
            });
            ..connect_destroy({
                let tx = tx.clone();
                move |_| glib_send(tx.clone(), Event::Exit)
            });
        };

        let dialog = gtk::Dialog::builder()
            .title(&fl!("schedule-dialog-title"))
            .attached_to(widget)
            .build();

        dialog.content_area().add(&{
            cascade! {
                option_frame(content.upcast_ref::<gtk::Widget>());
                ..set_margin_start(4);
                ..set_margin_end(4);
                ..set_margin_top(12);
                ..set_halign(gtk::Align::Center);
                ..set_hexpand(true);
            }
        });

        glib_spawn(async move {
            let mut config = pop_system_updater::config::load_system_config().await;

            let schedule = match config.schedule.as_ref() {
                Some(sched) => sched.clone(),
                None => {
                    config.schedule = Some(Config::default_schedule());
                    Config::default_schedule()
                }
            };
            interval.set_active(Some(match schedule.interval {
                Interval::Monday => 0,
                Interval::Tuesday => 1,
                Interval::Wednesday => 2,
                Interval::Thursday => 3,
                Interval::Friday => 4,
                Interval::Saturday => 5,
                Interval::Sunday => 6,
                Interval::Weekdays => 7,
            }));

            when_available.set_active(config.schedule.is_none());

            let (am, hour_value) = if schedule.hour >= 12 {
                (1, schedule.hour - 12)
            } else {
                (0, schedule.hour)
            };

            time_of_day.set_active(Some(am));
            hour.set_value(hour_value as u32);
            minute.set_value(schedule.minute as u32);

            // Connect widgets now that state is set.
            let update_config = Rc::new(Box::new(move || {
                glib_send(tx.clone(), Event::UpdateConfig);
            }));

            let update_sensitivity = |insensitive: bool| {
                hour.set_sensitive(!insensitive);
                minute.set_sensitive(!insensitive);
                time_of_day.set_sensitive(!insensitive);
                interval.set_sensitive(!insensitive);

                let label_ctx = schedule_label.style_context();

                if !insensitive {
                    label_ctx.remove_class("dim-label");
                } else {
                    label_ctx.add_class("dim-label");
                }
            };

            update_sensitivity(config.schedule.is_none());

            when_available.connect_changed_active({
                let update_config = update_config.clone();
                move |_| update_config()
            });

            interval.connect_changed({
                let update_config = update_config.clone();
                move |_| update_config()
            });

            time_of_day.connect_changed({
                let update_config = update_config.clone();
                move |_| update_config()
            });

            #[allow(clippy::redundant_closure)]
            hour.connect_update({
                let update_config = update_config.clone();
                move || update_config()
            });

            #[allow(clippy::redundant_closure)]
            minute.connect_update({
                let update_config = update_config.clone();
                move || update_config()
            });

            while let Some(event) = rx.recv().await {
                match event {
                    Event::UpdateConfig => {
                        update_sensitivity(config.schedule.is_none());
                        let pm = time_of_day.active() == Some(1);

                        let mut hour = hour.value() as u8;

                        if pm {
                            hour += 12;
                        }

                        func(Config {
                            auto_update: true,
                            schedule: if when_available.is_active() {
                                None
                            } else {
                                Some(Schedule {
                                    interval: match interval.active() {
                                        Some(0) => Interval::Monday,
                                        Some(1) => Interval::Tuesday,
                                        Some(2) => Interval::Wednesday,
                                        Some(3) => Interval::Thursday,
                                        Some(4) => Interval::Friday,
                                        Some(5) => Interval::Saturday,
                                        Some(6) => Interval::Sunday,
                                        Some(7) => Interval::Weekdays,
                                        _ => {
                                            eprintln!("Unknown interval option selected");
                                            continue;
                                        }
                                    },
                                    hour,
                                    minute: minute.value() as u8,
                                })
                            },
                        });
                    }
                    Event::Exit => break,
                }
            }
        });

        Dialog(dialog)
    }
}
