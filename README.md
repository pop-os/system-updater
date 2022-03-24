# Pop System Updater

DBus services to enable Linux distributions to schedule system updates across a variety of package managers.

## Build

This project uses [just](https://github.com/casey/just) as a command runner.

```sh
# Vendor dependencies
just vendor

# Build with vendored dependencies
just vendor=1

# Run GTK test
just debug=1 vendor=1 gtk-test

# install to custom root path
just rootdir=chroot prefix=/usr vendor=1 install

# List Recipes
just -l
```

## License

Licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0/).

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under the Mozilla Public License 2.0 (MPL-2.0).
