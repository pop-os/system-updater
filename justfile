rootdir := ''
prefix := '/usr'
clean := '0'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
cargo_args := vendor_args + ' ' + debug_args

root := rootdir + prefix
bindir := root + '/bin'
includedir := root + '/include'
sysconfdir := root + '/share'
libdir := root + '/lib'

id := 'com.system76.SystemUpdater'
id_local := id + '.Local'
binary := 'pop-system-updater'
gtklib := 'pop_system_updater_gtk'

target_bin := bindir + '/' + binary
target_dbus_conf := sysconfdir + '/dbus-1/system.d/' + id + '.conf'
target_systemd_service := rootdir + '/lib/systemd/system/' + id + '.service'
target_session_service := rootdir + '/lib/systemd/user/' + id_local + '.service'

# Compiles pop-system-updater.
all: _extract_vendor
    cargo build -p pop-system-updater {{cargo_args}}
    cargo build -p gtk-ffi {{cargo_args}}
    just pkgconfig

pkgconfig:
    #!/bin/bash
    cat > target/{{gtklib}}.pc <<- EOM
    libdir={{libdir}}
    includedir={{includedir}}
    $(cat target/{{gtklib}}.pc.stub)
    EOM

# Remove Cargo build artifacts.
clean:
    cargo clean

# Also remove .cargo and vendored dependencies.
distclean:
    rm -rf .cargo vendor vendor.tar target

# Run the GTK UI for testing purposes.
run:
    cargo run -p pop-system-updater-gtk

# Run the systemd service for testing.
run_service:
    just && sudo env RUST_BACKTRACE=1 RUST_LOG=debug ./target/release/pop-system-updater

# Install the compiled project into the system.
install:
    install -Dm0755 target/{{target}}/{{binary}} {{target_bin}}
    install -Dm0644 data/{{id}}.conf {{target_dbus_conf}}
    install -Dm0644 data/{{id}}.service {{target_systemd_service}}
    install -Dm0644 data/{{id_local}}.service {{target_session_service}}
    install -Dm0644 gtk-ffi/{{gtklib}}.h {{includedir}}/{{gtklib}}.h
    install -Dm0644 target/{{target}}/lib{{gtklib}}.so {{libdir}}/lib{{gtklib}}.so
    install -Dm0655 target/{{gtklib}}.pc {{libdir}}/pkgconfig/{{gtklib}}.pc

# Uninstall the files that were installed.
uninstall:
    rm {{target_bin}} {{target_dbus_conf}} {{target_systemd_service}}

# Vendor Cargo dependencies locally.
vendor:
    mkdir -p .cargo
    cargo vendor --sync gtk/Cargo.toml \
        --sync daemon/Cargo.toml \
        | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Used by packaging systems to generate a source package.
package_source:
    #!/usr/bin/env sh
    if test {{clean}} = 1; then
        just clean
    fi

    if test {{vendor}} = 1; then
        ischroot || just vendor
    fi

# Used by packaging systems to build a binary package.
package_build:
    just debug={{debug}} vendor={{vendor}} sysconfdir='/usr/share'

# Extracts vendored dependencies if vendor=1
_extract_vendor:
    #!/usr/bin/env sh
    if test {{vendor}} = 1; then
        rm -rf vendor; tar pxf vendor.tar
    fi
