# Pop System Updater

DBus services to enable Pop!_OS to schedule system updates across a variety of package managers.

## Build

This project uses [just](https://github.com/casey/just) as a command runner.

```sh
# Vendor dependencies
just vendor

# Build with vendored dependencies
just build-vendored

# install to custom root path
sudo just rootdir=/chroot prefix=/usr install
```

## License

Licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0/).

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under the Mozilla Public License 2.0 (MPL-2.0).
