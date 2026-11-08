# Architecture

## The shape of it

HA-Panel is a Tauri application. It owns three windows and one sidecar.

```
┌──────────────────────────────────────────────────────────┐
│ compositor (cage/labwc, or an existing session)          │
│                                                          │
│  ┌────────────────────────────────┐  ┌───────────────┐  │
│  │ "home"  — HA webview          │  │ "drawer"      │  │
│  │ fullscreen, always on top      │  │ local page,   │  │
│  │ remote URL, no IPC granted     │  │ settings +    │  │
│  │                                │  │ device control│  │
│  └────────────────────────────────┘  └───────────────┘  │
│                                       ┌───────────────┐  │
│                                       │ "keyboard"    │  │
│                                       │ local page,   │  │
│                                       │ hidden until  │  │
│                                       │ a text field  │  │
│                                       │ is focused    │  │
│                                       └───────────────┘  │
└──────────────────────────────────────────────────────────┘

  ha-panel (Rust) ──spawns──▶ ha-panel-sidecar (Go)
        │                            │
        │ JSON-RPC over stdio        │ EchoMuse protocol
        │ JSON-RPC over UDS          │ (WebSocket planes)
        ▼                            ▼
   /etc/ha-panel/*.json         EchoMuse controller
```

## Why the Home Assistant page is a top-level webview

Home Assistant sends `X-Frame-Options: SAMEORIGIN` on every response unless
`http.use_x_frame_options` is set to `false` in `configuration.yaml`. A kiosk
that had to ask the user to weaken that header, and that would silently break
the moment the user upgraded Home Assistant, is not a kiosk we want to ship.
So the HA page is loaded as a real top-level document in its own window and
gets no privileges from us.

That has one consequence worth stating plainly: **the HA page cannot call into
the Rust side.** Tauri grants IPC to a remote page through a capability, and a
capability names its allowed origins as URL patterns — which would have to be
the HA URL, and the HA URL is decided at runtime in the settings file. There
is no way to write a static capability for a URL the user has not chosen yet.

So the drawer is opened by a **thin always-on-top handle window** pinned to the
right edge of the screen. It is a separate local window, so it needs no
capability at all, and it works whether the HA URL is `http://10.0.0.5:8123`
or `https://home.example.com`.

## Why the drawer is a window and not an overlay

Tauri's `set_ignore_cursor_events` — the call that would let a transparent
overlay sit over the page and let clicks fall through to it — is implemented on
Linux with X11's `input_shape_combine_region`. On Wayland there is no
equivalent: the compositor does not let a client punch a hole in its own
input region. The Athena image runs Phoc with `xwayland=false`, so a click-
through overlay is not available on the target device.

The drawer is therefore an ordinary window, always on top, that slides in from
the right edge. On X11 it is placed flush against the right edge; on Wayland
the same geometry is used, and the compositor's own rules decide what is
visible.

## Windows

| Label | Origin | Purpose |
| --- | --- | --- |
| `home` | Remote (the HA URL) | The dashboard. Fullscreen, no decorations, no taskbar entry. |
| `drawer` | Local | Settings, device control, status. Hidden by default, slides in from the right. |
| `keyboard` | Local | On-screen keyboard. Hidden until the user focuses a text field. |
| `handle` | Local | The thin strip that opens the drawer. Always visible, always on top, very narrow. |

The `handle` window is the only always-visible chrome. It is 12 logical pixels
wide, so it costs almost nothing to composite and never covers content that
matters.

## Home Assistant authentication

Home Assistant's frontend stores its tokens in `localStorage` under the key
`hassTokens`. The app writes the user's long-lived access token into that key
before the page's first script runs, using a Tauri initialization script that
is injected into the top frame.

This gives a kiosk that is logged in at every boot with no login screen and no
credential entry on the device. The long-lived token itself lives in the
secrets file, which is separate from the settings file and is not world
readable.

The initialization script is injected into the page it loads. It only writes
`hassTokens`; it exposes no command surface to the page.

## Device control

Device control lives in the Rust process, not in the sidecar, because it is
cheap there and it keeps the sidecar's job narrow.

| Concern | Backend | Detection |
| --- | --- | --- |
| Volume, mute | PulseAudio / PipeWire via `pactl` / `wpctl`, else ALSA `amixer` | Probe for the binaries and the sound server |
| Audio in/out | Same as above, listing sinks and sources | Same |
| Screen power | `wlopm` on Wayland, `xset dpms` on X11 | `$WAYLAND_DISPLAY` / `$DISPLAY` |
| Brightness | `/sys/class/backlight/*/brightness`, else `ddcutil` | Scan sysfs |
| Wi-Fi | NetworkManager over D-Bus | D-Bus socket + `org.freedesktop.NetworkManager` present |
| Wi-Fi fallback | `nmcli` | `nmcli` on `$PATH` |

Every backend reports *unsupported* rather than failing silently. The drawer
disables a control that has no backend, instead of offering a button that does
nothing.

Brightness and screen power fall back to a configurable command in the
settings file, so a device whose backlight is not exposed through sysfs can
still be driven.

## The EchoMuse sidecar

The EchoMuse SDK is a Go library. It speaks the device half of the EchoMuse
protocol: it registers with a controller, opens the control, data and shell
planes, and hands the application callbacks for the things the controller
asks of the device.

The sidecar is a thin Go program that:

1. owns the SDK client, the ALSA microphone, and the speaker;
2. translates controller requests into audio I/O;
3. reduces the controller's ring animations to a small state object;
4. exposes all of it over JSON-RPC.

The Rust app spawns it and talks to it over JSON-RPC on stdio. The same
protocol is available on a Unix domain socket so that other tools on the
device — a health check, a provisioning script, a debugging shell — can query
the sidecar without going through the app.

The SDK is vendored into `sidecar/vendor/echomuse-sdk` as a plain copy at a
pinned upstream commit. Nothing in the SDK repository is modified. The
provenance is recorded in `sidecar/vendor/echomuse-sdk/PROVENANCE.md`.

## Logging and writes

The disk is eMMC, and every write costs something. The rules:

- The app writes to disk only when the user changes a setting.
- Logs go to a fixed-size ring buffer in memory. The buffer is dumped to
  stderr — which systemd captures into the journal, in RAM — only when the
  log level is at or above `warn`, or when the user asks for a diagnostics
  dump.
- There is no log rotation to manage, because there are no log files.
- The webview data directory is a single directory that is written by the
  WebKitGTK runtime itself. Its size is bounded by the WebKit cache limits
  set in the app's environment.

## Running without a desktop

The `ha-panel-compositor` systemd unit starts a wlroots compositor (cage by
default, labwc if configured) and then the app inside it. The
`ha-panel` unit is for deployments that already have a session — a Phosh
shell on the Athena image, or a normal desktop.

Both variants install the same binary and the same configuration.

## Resource budget

Measured targets on a Cortex-A53 class device with 2 GB of RAM:

| Process | Target RSS |
| --- | --- |
| `ha-panel` (Rust) | < 60 MB |
| `ha-panel-sidecar` (Go) | < 25 MB |
| WebKitGTK web process | < 120 MB |

The Rust process is the only part that scales with the number of features
enabled. The sidecar does not start at all unless the EchoMuse integration is
configured, which keeps the default installation to two processes.
