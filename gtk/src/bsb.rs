use gtk::prelude::*;

pub struct BetterSpinButton {
    pub root: gtk::Box,
    entry: gtk::Entry,
    pub padding: u32,
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
                0
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
                let current = || entry.text().as_str().parse::<u32>().unwrap_or(0);
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
        }
    }

    pub fn connect_update(&self, func: impl Fn() + 'static) {
        let (tx, rx) = flume::unbounded();

        let id = self.entry.connect_changed(move |entry| {
            let _ = tx.send(entry.text());
        });

        let mut last_known = self.entry.text().to_string();

        let entry = self.entry.downgrade();
        crate::utils::glib_spawn(async move {
            while let Ok(text) = rx.recv_async().await {
                if text.as_str().parse::<u32>().is_ok() {
                    func();

                    let nchars = text.as_str().len();

                    if nchars == 1 {
                        last_known = ["0", text.as_str()].concat();
                    } else if nchars > 2 {
                        last_known = text.as_str()[nchars - 2..].to_owned();
                    } else {
                        last_known = text.to_string();
                        continue;
                    }
                }

                let entry = match entry.upgrade() {
                    Some(entry) => entry,
                    None => return,
                };

                entry.block_signal(&id);
                entry.set_text(&*last_known);
                entry.set_position(last_known.len() as i32);
                entry.unblock_signal(&id)
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
    gtk::Button::from_icon_name(Some(icon), gtk::IconSize::Button)
}

fn format_number(value: u32, padding: usize) -> String {
    format!("{:01$}", value, padding)
}
