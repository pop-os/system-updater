use glib::translate::*;
use gtk::prelude::ContainerExt;
use pop_system_updater_gtk::{localize, SettingsWidget};

/// Localizes the system-updater widget strings.
///
/// # Warning
///
/// This must be called before attaching the widget.
#[no_mangle]
pub extern "C" fn pop_system_updater_localize() {
    localize();
}

/// Creates and attaches the system-updater widget to the container.
///
/// # Safety
///
/// The container pointer must be valid.
#[no_mangle]
pub unsafe extern "C" fn pop_system_updater_attach(container: *mut gtk_sys::GtkContainer) {
    if container.is_null() {
        eprintln!("cannot attach system updater widget to null container");
        return;
    }

    gtk::set_initialized();

    gtk::Container::from_glib_none(container).add(&SettingsWidget::new().inner);
}
