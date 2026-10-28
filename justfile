mod cargo 'cargo.just'

name := 'pop-system-updater'
appid := 'com.system76.SystemUpdater'
rootdir := ''
prefix := '/usr'
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
base-dir := absolute_path(clean(rootdir / prefix))
bin-dst := base-dir / 'bin' / name
dbus-conf-dst := base-dir / 'share/dbus-1/system.d' / appid + '.conf'
system-service-dst := rootdir / 'lib/systemd/system' / appid + '.service'
session-service-dst := rootdir / 'lib/systemd/user' / appid + '.Local.service'

default: cargo::build-release

build-vendored: cargo::build-vendored

clean: cargo::clean

clean-dist: cargo::clean-dist

run: cargo::build-release
    sudo env RUST_BACKTRACE=1 RUST_LOG=debug ./target/release/pop-system-updater

install:
    install -Dm0755 {{ cargo-target-dir / 'release' / name }} {{ bin-dst }}
    install -Dm0644 data/{{ appid }}.conf {{ dbus-conf-dst }}
    install -Dm0644 data/{{ appid }}.service {{ system-service-dst }}
    install -Dm0644 data/{{ appid }}.Local.service {{ session-service-dst }}

uninstall:
    rm {{ bin-dst }} {{ dbus-conf-dst }} {{ session-service-dst }} {{ system-service-dst }}

vendor: cargo::vendor

vendor-extract: cargo::vendor-extract
