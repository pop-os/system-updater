use std::{env, fs::File, io::Write, path::PathBuf, os::unix::prelude::OsStrExt};

fn main() {
    cdylib_link_lines::metabuild();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut target_dir = out_dir.as_path();

    while let Some(parent) = target_dir.parent() {
        if target_dir.as_os_str().as_bytes().ends_with(b"target") {
            break
        }
        target_dir = parent;
    }

    let pkg_config = format!(
        include_str!("pop_system_updater_gtk.pc.in"),
        name = "pop_system_updater_gtk",
        description = env::var("CARGO_PKG_DESCRIPTION").unwrap(),
        version = env::var("CARGO_PKG_VERSION").unwrap()
    );

    std::fs::create_dir_all("../target").unwrap();

    File::create("../target/pop_system_updater_gtk.pc.stub")
        .expect("failed to create pc.stub")
        .write_all(pkg_config.as_bytes())
        .expect("failed to write pc.stub");
}
