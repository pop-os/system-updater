use gtk::prelude::*;

pub struct BetterSpinButton {
    pub root: gtk::Box,
    entry: gtk::Entry,
    padding: u32,
    min: u32,
    max: u32,
}

impl std::ops::Deref for BetterSpinButton {
    type Target = gtk::Box;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

impl BetterSpinButton {
    pub fn new(min: u32, max: u32, inc_small: u32, inc_large: u32, padding: u32) -> Self {
        // Increase the value while preventing it from exceeding the max.
        let increase = move |value: u32, inc: u32| (value + inc).min(max);

        // Decrease the value while preventing it from exceeding the min.
        let decrease = move |value: u32, inc: u32| {
            if value < inc {
                min
            } else {
                (value - inc).max(min)
            }
        };

        let entry = cascade! {
            gtk::Entry::default();
            ..set_max_width_chars(2);
            ..set_width_chars(2);
            // Configure arrow key presses to increment and decrement the value
            ..connect_key_press_event(move |entry, event| {
                let current = || entry.text().as_str().parse::<u32>().unwrap_or(min);
                let set = |value| entry.set_text(&*format_number(value, padding as usize));

                match event.keycode() {
                    Some(111) => set(increase(current(), inc_large)),
                    Some(113) => set(decrease(current(), inc_small)),
                    Some(114) => set(increase(current(), inc_small)),
                    Some(116) => set(decrease(current(), inc_large)),
                    _ => return gtk::Inhibit(false)
                }

                gtk::Inhibit(true)
            });
        };

        fn on_click(
            entry: &gtk::Entry,
            func: impl Fn(u32) -> u32 + 'static,
            padding: u32,
        ) -> impl Fn(&gtk::Button) + 'static {
            glib::clone!(@weak entry => move |_| {
                if entry.text_length() > 0 {
                    match entry.text().as_str().parse::<u32>() {
                        Ok(value) => entry.set_text(&*format_number(func(value), padding as usize)),
                        Err(_) => entry.set_text(""),
                    }
                }
            })
        }

        let plus = cascade! {
            create_button("value-increase-symbolic");
            ..connect_clicked(on_click(&entry, move |value| increase(value, inc_small), padding));
        };

        let minus = cascade! {
            create_button("value-decrease-symbolic");
            ..connect_clicked(on_click(&entry, move |value| decrease(value, inc_small), padding));
        };

        let root = cascade! {
            gtk::Box::default();
            ..set_orientation(gtk::Orientation::Vertical);
            ..add(&plus);
            ..add(&entry);
            ..add(&minus);
        };

        BetterSpinButton {
            entry,
            root,
            padding,
            max,
            min,
        }
    }

    pub fn connect_update(&self, func: impl Fn() + 'static) {
        let (tx, rx) = flume::unbounded();

        let id = self.entry.connect_changed(move |entry| {
            let _ = tx.send(entry.text());
        });

        let mut last_known = self.entry.text().to_string();

        let entry = self.entry.downgrade();
        let min = self.min;
        let max = self.max;
        let padding = self.padding;

        crate::utils::glib_spawn(async move {
            while let Ok(text) = rx.recv_async().await {
                let new_value = match text.as_str().parse::<u32>() {
                    Ok(value) => {
                        if value < min {
                            min
                        } else if value > max {
                            max
                        } else {
                            value
                        }
                    }

                    Err(_) => min,
                };

                let new_value = format_number(new_value, padding as usize);

                let entry = match entry.upgrade() {
                    Some(entry) => entry,
                    None => return,
                };

                entry.block_signal(&id);
                entry.set_text(&*new_value);
                entry.set_position(new_value.len() as i32);
                entry.unblock_signal(&id);

                if new_value != last_known {
                    last_known = new_value;

                    func();
                }
            }
        })
    }

    pub fn set_value(&self, value: u32) {
        self.entry
            .set_text(&*format_number(value, self.padding as usize));
    }

    pub fn value(&self) -> u32 {
        self.entry.text().as_str().parse::<u32>().unwrap_or(0)
    }
}

fn create_button(icon: &str) -> gtk::Button {
    cascade! {
        gtk::Button::from_icon_name(Some(icon), gtk::IconSize::Button);
        ..set_can_focus(false);
    }
}

fn format_number(value: u32, padding: usize) -> String {
    format!("{:01$}", value, padding)
}
