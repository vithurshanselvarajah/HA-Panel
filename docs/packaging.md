# Packaging

## Debian and Ubuntu

The Tauri bundler makes the `.deb`.

````sh
./scripts/build.sh
npx tauri build --config src-tauri/tauri.conf.json
````

The package lands in `src-tauri/target/release/bundle/deb/`.

### Installing it

````sh
sudo apt install ./ha-panel_0.1.0_amd64.deb
sudo sh packaging/install.sh
````

The installer creates the kiosk user, writes a default configuration, and
starts the service. Every prompt can be answered in advance:

| Variable | Meaning |
| --- | --- |
| `HA_URL` | The Home Assistant address. |
| `DEVICE_NAME` | The name the panel shows. |
| `SKIP_SETUP=1` | Do not write a configuration. |
| `SKIP_SERVICE=1` | Do not start the service. |
| `INSTALL_COMPOSITOR=yes` | Always start a compositor. |
| `INSTALL_COMPOSITOR=no` | Never start a compositor. |

### The two installation variants

A panel runs in one of two ways, and both are installed by the same
package.

**With a desktop.** The device already has a session — a Phosh shell on
the Athena image, or a normal desktop. The app attaches to it.

````sh
sudo systemctl enable --now ha-panel.service
````

**Without a desktop.** The device has no session, so the package starts a
bare wlroots compositor and puts the app inside it.

````sh
sudo systemctl enable --now ha-panel.target
````

The target starts both `ha-panel-compositor.service` and
`ha-panel.service`. The compositor is `cage` by default; set
`HA_PANEL_COMPOSITOR=labwc` in the unit's environment to use labwc
instead.

## Alpine

Tauri has no `.apk` bundle target, so the Alpine package is built by hand
from `packaging/alpine/APKBUILD`.

The build runs **inside an Alpine container**, because the Rust
application has to link against the GTK and WebKitGTK libraries that
Alpine ships rather than the ones a Debian runner has. A cross build from
glibc would produce a binary that links the wrong libraries.

````sh
./scripts/build-alpine.sh --arch amd64
./scripts/build-alpine.sh --arch arm64
````

The container image is pinned by digest. The script runs
`scripts/alpine-build-inner.sh` inside it, which installs the toolchain,
builds the frontend, the Rust application and the Go sidecar, and then
runs `abuild` over `packaging/alpine/APKBUILD`. The `.apk` lands in
`dist/`.

> **This has not been tested on a real Alpine device.** The APKBUILD, the
> OpenRC service and the build scripts are written and reviewed, but they
> have not been through an actual `abuild`. Treat them as a starting point
> rather than as something known to work.

### The Alpine version

Alpine 3.23. It is the first Alpine release whose Rust (1.91) meets
Tauri's 1.90 minimum, so an older one cannot build this. Alpine 3.22
ships Rust 1.87 and does not work.

The Rust toolchain is pinned to a specific version through `rustup`,
rather than using Alpine's own `rust` package, so that the Alpine build
and the Debian build agree about the compiler.

### The Go sidecar on Alpine

The sidecar is built without cgo, which works because the SDK's ALSA
backend loads `libasound.so.2` at runtime rather than linking it. The
trade is that a sidecar built this way has no ALSA backend; see
[sidecar.md](sidecar.md).

## The architectures

| Architecture | Devices |
| --- | --- |
| `amd64` | Intel and AMD |
| `arm64` | Raspberry Pi 4, Raspberry Pi 5, the Athena board |

The Athena board is an MT8183, which is a 64-bit ARM SoC, so it takes the
`arm64` package.

## Upgrading

There is no self-update. A new version is installed the way the current one
was:

````sh
sudo apt install ./ha-panel_0.2.0_amd64.deb
sudo systemctl restart ha-panel.service
````

The configuration file is not touched by an upgrade. The secrets file is
not touched either.

## What the package installs

| Path | What |
| --- | --- |
| `/usr/lib/ha-panel/ha-panel` | The application. |
| `/usr/lib/ha-panel/ha-panel-sidecar` | The EchoMuse sidecar. |
| `/usr/lib/ha-panel/web/` | The frontend. |
| `/etc/ha-panel/config.json` | The configuration. |
| `/etc/ha-panel/secrets.json` | The secrets, if you put them there. |
| `/var/lib/ha-panel/` | The state: the EchoMuse credentials, the cache. |
| `/usr/lib/systemd/system/ha-panel.service` | The service. |
| `/usr/lib/systemd/system/ha-panel-compositor.service` | The compositor service. |
| `/usr/lib/systemd/system/ha-panel.target` | Both of them. |

## The installed size

Roughly 12 MB, most of which is the sidecar and the application. The
frontend is about 70 KB of JavaScript and CSS.

The runtime dependencies are the system's: `libwebkit2gtk-4.1-0`,
`libgtk-3-0`, and the tools the app drives — `wlopm`, `xset`, `amixer`,
`nmcli`. They are already on a device with a desktop.
