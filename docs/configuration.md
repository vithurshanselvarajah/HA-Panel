# Configuration

## The two files

HA-Panel keeps its configuration in `/etc/ha-panel/`, in two files.

| File | Owner | Mode | Holds |
| --- | --- | --- | --- |
| `config.json` | `root:ha-panel` | `0660` | Everything that is not a secret. |
| `secrets.json` | `ha-panel` | `0600` | The Home Assistant long-lived token. |

They are separate because they have different owners. The configuration file
is group-writable so the kiosk user can save its own settings; the secrets
file is not, so a long-lived token cannot be read by anything that happens
to be in the kiosk group.

Neither file is written unless something changed. The app writes the
configuration atomically: a temporary file in the same directory, then a
rename, so a power cut leaves the old file rather than a half-written one.

## config.json

Every field has a default, and a missing file is not an error. The app
starts with the defaults and shows its settings, so a device that has not
been provisioned yet still comes up.

````json
{
  "ha": {
    "url": "http://homeassistant.local:8123",
    "dashboard_path": "/lovelace",
    "fullscreen": true,
    "reload_on_reconnect": true
  },
  "kiosk": {
    "handle_width": 12,
    "drawer_width": 380,
    "show_handle": true,
    "on_screen_keyboard": true,
    "screen_off_timeout_secs": 0,
    "screen_on_timeout_secs": 0
  },
  "audio": {
    "backend": "auto",
    "volume": 50,
    "muted": false,
    "output_device": null,
    "input_device": null
  },
  "display": {
    "screen_power": "auto",
    "brightness": null,
    "brightness_command": null,
    "screen_off_command": null,
    "screen_on_command": null
  },
  "network": {
    "wifi_enabled": true,
    "preferred_connection": null
  },
  "echomuse": {
    "enabled": false,
    "controller_url": null,
    "controller_tls_port": 0,
    "device_id": null,
    "board": "ha-panel",
    "version": "0.1.0",
    "credentials_dir": "/var/lib/ha-panel/echomuse",
    "socket_path": "/run/ha-panel/sidecar.sock",
    "input_device": null,
    "output_device": null
  }
}
````

### ha

| Field | Meaning |
| --- | --- |
| `url` | The base address of the Home Assistant instance. |
| `dashboard_path` | The path of the dashboard to open. `/lovelace` is the default view. |
| `fullscreen` | Whether the Home Assistant window is fullscreen. |
| `reload_on_reconnect` | Whether to reload the dashboard when the network comes back. |

### kiosk

| Field | Meaning |
| --- | --- |
| `handle_width` | The width of the edge handle, in logical pixels. |
| `drawer_width` | The width of the drawer, in logical pixels. |
| `show_handle` | Whether the edge handle is shown. |
| `on_screen_keyboard` | Whether the built-in keyboard is offered. |
| `screen_off_timeout_secs` | Idle seconds before the screen turns off. Zero disables it. |
| `screen_on_timeout_secs` | Idle seconds before the screen turns back on. Zero disables it. |

### audio

`backend` is one of `auto`, `pulse`, `pipewire` or `alsa`. `auto` prefers
PipeWire, then PulseAudio, then ALSA. A named backend that is not present
falls back to one that is, and the fallback is logged rather than being an
error — a device may have had its sound server removed.

`output_device` and `input_device` are the endpoint names the backend
reported. They are null for the system default.

### display

`screen_power` is one of `auto`, `wlopm`, `xset` or `disabled`. `auto`
picks `wlopm` under Wayland and `xset` under X11.

The three command fields are for a device whose backlight or display power
is not exposed through the standard mechanisms. `%` in an argument is
replaced by the requested value.

> The command is split on whitespace and run directly. It is never handed
> to a shell, so nothing in the configuration file can cause another
> command to run.

### network

`wifi_enabled` turns the Wi-Fi controls on and off in the drawer.

### echomuse

The voice link. See [sidecar.md](sidecar.md).

## secrets.json

````json
{
  "ha_long_lived_token": "eyJhbGciOiJIUzI1NiJ9..."
}
````

The token is a Home Assistant long-lived access token. Create one in Home
Assistant under your profile, at the bottom of the page.

The app writes it into the Home Assistant frontend's own token store before
the page's scripts run, so the kiosk is logged in at every boot with no
login screen and no credentials on the device beyond this token.

## Provisioning a device

````sh
# The configuration.
install -d -o root -g ha-panel -m 0750 /etc/ha-panel
cat >/etc/ha-panel/config.json <<'EOF'
{
  "ha": { "url": "https://homeassistant.example.com" }
}
EOF
chown root:ha-panel /etc/ha-panel/config.json
chmod 0660 /etc/ha-panel/config.json

# The secret.
cat >/etc/ha-panel/secrets.json <<'EOF'
{ "ha_long_lived_token": "..." }
EOF
chown ha-panel:ha-panel /etc/ha-panel/secrets.json
chmod 0600 /etc/ha-panel/secrets.json
````

## The kiosk user

The service runs as `ha-panel`, a system user with no shell and no home
directory of its own. It is in the `audio`, `video`, `input`, `render` and
`network` groups, so it can drive the sound card and talk to
NetworkManager.

It needs these capabilities:

| Capability | Why |
| --- | --- |
| `CAP_NET_ADMIN` | Not needed — NetworkManager does the work. |
| Access to `/dev/snd` | The microphone and the speaker. |
| Access to `/dev/dri` | The GPU, for the WebKitGTK compositor. |
| D-Bus, as a member of `network` | NetworkManager over D-Bus. |
| Read access to `/sys/class/backlight` | The brightness. |
