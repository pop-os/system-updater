rootdir := ''
prefix := '/usr'
clean := '0'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
cargo_args := vendor_args + ' ' + debug_args

sysconfdir := '/etc'
bindir := prefix + '/bin'
libdir := prefix + '/lib'

id := 'com.system76.SystemUpdater'
id_local := id + '.Local'
binary := 'pop-system-updater'

target_bin := rootdir + bindir + '/' + id
target_dbus_conf := rootdir + sysconfdir + '/dbus-1/systemd./' + id + '.conf'
target_systemd_service := rootdir + libdir + '/systemd/system/' + id + '.service'
target_session_service := rootdir + libdir + '/systemd/user/' + id_local + '.service'

# Compiles pop-system-updater.
all: _extract-vendor
    cargo build {{cargo_args}}

# Remove Cargo build artifacts.
clean:
    cargo clean

# Also remove .cargo and vendored dependencies.
distclean:
    rm -rf .cargo vendor vendor.tar target

# Run the GTK UI for testing purposes.
gtk-test:
    cargo run -p pop-system-updater-gtk {{cargo_args}}

# Install the compiled project into the system.
install:
    install -Dm04755 target/{{target}}/{{binary}} {{target_bin}}
    install -Dm0644 data/{{id}}.conf {{target_dbus_conf}}
    install -Dm0644 data/{{id}}.service {{target_systemd_service}}
    install -Dm0644 data/{{id_local}}.service {{target_session_service}}

# Uninstall the files that were installed.
uninstall:
    rm {{target_bin}} {{target_dbus_conf}} {{target_systemd_service}}

# Vendor Cargo dependencies locally.
vendor:
    mkdir -p .cargo
    cargo vendor --sync gtk/Cargo.toml \
        | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Used by packaging systems to generate a source package.
package_source:
    #!/usr/bin/env sh
    if test {{clean}} -eq 1; then
        just clean
    fi

    if test {{vendor}} -eq 1; then
        ischroot || just vendor
    fi

# Used by packaging systems to build a binary package.
package_build:
    env CARGO_HOME={{justfile_directory()}}/target/cargo \
        just debug={{debug}} vendor={{vendor}} sysconfdir='/usr/share'

# Extracts vendored dependencies if vendor=1
_extract-vendor:
    #!/usr/bin/env sh
    if test {{vendor}} -eq 1; then
        rm -rf vendor; tar pxf vendor.tar
    fi