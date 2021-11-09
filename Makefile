export prefix ?= /usr
sysconfdir ?= /etc
bindir = $(prefix)/bin
libdir = $(prefix)/lib

ID=com.system76.SystemUpdater
ID_LOCAL=$(ID).Local
BINARY=pop-system-updater

TARGET = debug
DEBUG ?= 0

.PHONY = all clean install uninstall vendor

ifeq ($(DEBUG),0)
	TARGET = release
	ARGS += --release
endif

VENDOR ?= 0
ifneq ($(VENDOR),0)
	ARGS += --frozen
endif

TARGET_BIN="$(DESTDIR)$(bindir)/$(ID)"
TARGET_DBUS_CONF="$(DESTDIR)$(sysconfdir)/dbus-1/system.d/$(ID).conf"
TARGET_SYSTEMD_SERVICE="$(DESTDIR)$(libdir)/systemd/system/$(ID).service"
TARGET_SESSION_SERVICE="$(DESTDIR)$(libdir)/systemd/user/$(ID_LOCAL).service"

all: extract-vendor
	cargo build $(ARGS)

clean:
	cargo clean

distclean:
	rm -rf .cargo vendor vendor.tar target

vendor:
	mkdir -p .cargo
	cargo vendor | head -n -1 > .cargo/config
	echo 'directory = "vendor"' >> .cargo/config
	tar pcf vendor.tar vendor
	rm -rf vendor

extract-vendor:
ifeq ($(VENDOR),1)
	rm -rf vendor; tar pxf vendor.tar
endif

gtk-test:
	cargo run -p pop-system-updater-gtk

install:
	install -Dm04755 "target/$(TARGET)/$(BINARY)" "$(TARGET_BIN)"
	install -Dm0644 "data/$(ID).conf" "$(TARGET_DBUS_CONF)"
	install -Dm0644 "data/$(ID).service" "$(TARGET_SYSTEMD_SERVICE)"
	install -Dm0644 "data/$(ID_LOCAL).service" "$(TARGET_SESSION_SERVICE)"

uninstall:
	rm "$(TARGET_BIN)" "$(TARGET_DBUS_CONF)" "$(TARGET_SYSTEMD_SERVICE)"