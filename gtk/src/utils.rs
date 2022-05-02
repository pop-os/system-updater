// Copyright 2021-2022 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use gtk::prelude::*;
use postage::sink::Sink;
use std::future::Future;

pub fn glib_spawn<F: Future<Output = ()> + 'static>(future: F) {
    glib::MainContext::default().spawn_local(future);
}

pub fn glib_send<E: 'static, S: Sink<Item = E> + Unpin + 'static>(mut sink: S, event: E) {
    glib_spawn(async move {
        let _res = sink.send(event).await;
    });
}

pub fn option_container() -> gtk::Grid {
    gtk::Grid::builder()
        .margin_start(20)
        .margin_end(20)
        .margin_top(8)
        .margin_bottom(8)
        .column_spacing(24)
        .row_spacing(4)
        .width_request(-1)
        .height_request(32)
        .build()
}

pub fn option_frame(widget: &gtk::Widget) -> gtk::Frame {
    cascade! {
        gtk::Frame::new(None);
        ..set_margin_bottom(12);
        ..add(widget);
        ..show_all();
    }
}

pub fn separator_header(current: &gtk::ListBoxRow, _before: Option<&gtk::ListBoxRow>) {
    current.set_header(Some(&gtk::Separator::new(gtk::Orientation::Horizontal)));
}

pub fn as_12(hour: u8) -> (u8, bool) {
    if hour == 0 {
        (12, false)
    } else if hour < 12 {
        (hour, false)
    } else if hour == 12 {
        (12, true)
    } else {
        (hour - 12, true)
    }
}
