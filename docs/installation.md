# Installation

How to put HA-Panel on a device and make it start at boot.

## What you are installing

| | |
| --- | --- |
| The application | `/usr/bin/ha-panel` |
| The sidecar | `/usr/bin/ha-panel-sidecar` |
| The configuration | `/etc/ha-panel/config.json` |
| The service | `ha-panel.service` |

The frontend is compiled into the application, so there is no separate
directory of web files to install.

The package is about 12 MB. The runtime dependencies are the system's:
`libwebkit2gtk-4.1-0`, `libgtk-3-0`, and the tools the app drives —
`wlopm`, `xset`, `amixer`, `nmcli`.

## The two ways to run it

A panel runs in one of two ways, and both are installed by the same
package. The difference is whether the device already has a desktop
session.

**With a desktop.** The device has a session — a Phosh shell on the
Athena image, or a normal desktop. The app attaches to it.

**Without a desktop.** The device has no session, so the package starts a
bare wlroots compositor and puts the app inside it.

Both are started by systemd, and both are on the same box at the same
time only if you ask for it.

---

## Debian and Ubuntu

### From a release

```sh
sudo apt install ./HA-Panel_0.1.0_amd64.deb
```

For an ARM device — a Raspberry Pi 4, a Raspberry Pi 5, the Athena
board — use the `arm64` package instead:

```sh
sudo apt install ./HA-Panel_0.1.0_arm64.deb
```

### From source

```sh
git clone https://github.com/vithurshanselvarajah/HA-Panel
cd HA-Panel
./scripts/build.sh
npx tauri build --config src-tauri/tauri.conf.json
sudo apt install ./src-tauri/target/release/bundle/deb/*.deb
```

### Setting it up

The package creates the `ha-panel` user and a default configuration, but
does not know your Home Assistant's address. There are two ways to finish
the job.

**The installer**, which asks:

```sh
sudo sh packaging/install.sh
```

It asks for the Home Assistant address and the name of the panel, writes
the configuration, and starts the service. Every question can be
answered in advance, so the whole thing can run unattended:

```sh
HA_URL="https://homeassistant.example.com" \
DEVICE_NAME="Kitchen" \
  sudo -E sh packaging/install.sh
```

| Variable | What it does |
| --- | --- |
| `HA_URL` | The Home Assistant address. |
| `DEVICE_NAME` | The name the panel shows in Home Assistant. |
| `SKIP_SETUP=1` | Do not write a configuration. |
| `SKIP_SERVICE=1` | Do not start the service. |
| `INSTALL_COMPOSITOR=yes` | Always start a compositor. |
| `INSTALL_COMPOSITOR=no` | Never start a compositor. |

**By hand**, if you would rather write the file yourself:

```sh
sudo install -d -o root -g ha-panel -m 0750 /etc/ha-panel

sudo tee /etc/ha-panel/config.json >/dev/null <<'EOF'
{
  "ha": {
    "url": "https://homeassistant.example.com",
    "dashboard_path": "/lovelace",
    "fullscreen": true
  },
  "kiosk": {
    "handle_width": 12,
    "drawer_width": 380,
    "show_handle": true,
    "on_screen_keyboard": true
  },
  "audio": {
    "backend": "auto",
    "volume": 50,
    "muted": false
  },
  "display": {
    "screen_power": "auto"
  },
  "network": {
    "wifi_enabled": true
  },
  "echomuse": {
    "enabled": false
  }
}
EOF

sudo chown root:ha-panel /etc/ha-panel/config.json
sudo chmod 0660 /etc/ha-panel/config.json
```

### Logging in

The panel shows the Home Assistant login screen unless you give it a
long-lived access token. Without one, you have to type the password
into the panel once per reboot, which is not what a kiosk is for.

Create a token in Home Assistant: click your name in the bottom left,
then **Security**, then **Long-lived access tokens**, then
**Create token**. Copy the token it gives you.

Put it in the secrets file:

```sh
sudo install -o ha-panel -g ha-panel -m 0600 /dev/null /etc/ha-panel/secrets.json

sudo tee /etc/ha-panel/secrets.json >/dev/null <<'EOF'
{
  "ha_long_lived_token": "eyJhbGciOiJIUzI1NiJ9..."
}
EOF
```

The app writes the token into the Home Assistant frontend's own token
store before the page's scripts run, so the panel is logged in at every
boot.

### Starting it

With a desktop session:

```sh
sudo systemctl enable --now ha-panel.service
```

Without one:

```sh
sudo systemctl enable --now ha-panel.target
```

The target starts both `ha-panel-compositor.service` and
`ha-panel.service`. The compositor is `cage` by default; to use labwc
instead:

```sh
sudo systemctl edit ha-panel-compositor.service
```

```ini
[Service]
Environment=HA_PANEL_COMPOSITOR=labwc
```

### Checking it worked

```sh
systemctl status ha-panel.service
journalctl -u ha-panel.service -f
```

The panel's own log is in memory. It reaches the journal only at `warn`
and above, so a healthy panel is quiet. To see everything:

```sh
sudo systemctl edit ha-panel.service
```

```ini
[Service]
Environment=HA_PANEL_LOG=info
```

```sh
sudo systemctl restart ha-panel.service
journalctl -u ha-panel.service -f
```

---

## Alpine

### From a release

```sh
apk add --allow-untrusted ./ha-panel-0.1.0-r0.apk
```

Alpine's package manager will refuse an unsigned package without
`--allow-untrusted`. A signed one does not need it.

### From source

The Alpine package is built inside an Alpine container, because the
application has to link the GTK and WebKitGTK libraries that Alpine
ships rather than the ones a Debian runner has:

```sh
docker build ... # or, more simply:
./scripts/build-alpine.sh --arch amd64
```

Which produces the `.apk` in `dist/`. See [packaging.md](packaging.md)
for what has and has not been verified.

### Setting it up

Alpine has no systemd; the package uses OpenRC.

```sh
sudo rc-update add ha-panel default
sudo rc-service ha-panel start
```

The configuration is the same file, in the same place:

```sh
sudo install -d -o root -g ha-panel -m 0750 /etc/ha-panel
sudo tee /etc/ha-panel/config.json >/dev/null <<'EOF'
{
  "ha": { "url": "http://homeassistant.local:8123" },
  "echomuse": { "enabled": false }
}
EOF
sudo chown root:ha-panel /etc/ha-panel/config.json
sudo chmod 0660 /etc/ha-panel/config.json
```

### The compositor

Alpine has no Phosh, so a headless panel needs a compositor. Install
one:

```sh
sudo apk add cage
```

And start it before the panel:

```sh
sudo rc-update add cage default
sudo rc-service cage start
```

### The logs

```sh
sudo rc-service ha-panel status
tail -f /var/log/ha-panel.log
```

---

## Provisioning a fleet

A panel that is installed by hand is a panel that has to be installed by
hand. If you have more than one, write the configuration into the image
and let the device manage itself.

### In the image

If you build the device image yourself, put the configuration in it:

```sh
install -d -o root -g ha-panel -m 0750 /etc/ha-panel
install -m 0660 -o root -g ha-panel config.json /etc/ha-panel/config.json
install -m 0600 -o ha-panel -g ha-panel secrets.json /etc/ha-panel/secrets.json
systemctl enable ha-panel.service
```

### Over the network

The panel has no API of its own, but the configuration is a file and the
service can be restarted over SSH:

```sh
ssh panel 'sudo install -m 0660 -o root -g ha-panel /dev/stdin /etc/ha-panel/config.json' \
  < config.json
ssh panel 'sudo systemctl restart ha-panel.service'
```

A panel that is provisioned this way should have its SSH keys in place
first.

### The token

The long-lived token is the same for every panel, or a different one for
each. One per panel is better: a token that is only valid on one device
means one compromised panel does not give away the others.

---

## Upgrading

There is no self-update. A new version is installed the way the current
one was:

```sh
sudo apt install ./HA-Panel_0.2.0_amd64.deb
sudo systemctl restart ha-panel.service
```

The configuration and the secrets are not touched by an upgrade.

## Uninstalling

```sh
sudo apt remove ha-panel
```

The configuration in `/etc/ha-panel/` is left alone, because it holds
the token and the Wi-Fi settings. Remove it by hand if you want it gone:

```sh
sudo rm -rf /etc/ha-panel /var/lib/ha-panel
```

---

## Troubleshooting

**The panel is black, or shows nothing.**

```sh
systemctl status ha-panel.service
journalctl -u ha-panel.service -n 50
```

The commonest cause is that the app has no display. Check that
`$DISPLAY` or `$WAYLAND_DISPLAY` is set in the service's environment.

**The drawer does not open.**

The edge handle is a separate window that opens the drawer. If the
handle is not visible, `show_handle` is off in the configuration. If the
handle is visible but does nothing, the app is not reaching the Home
Assistant page — which is what the drawer is layered over.

**The panel shows the Home Assistant login screen.**

There is no long-lived token, or the token has been revoked. Check that
`/etc/ha-panel/secrets.json` exists and is owned by `ha-panel`:

```sh
sudo ls -l /etc/ha-panel/secrets.json
```

**The volume control does nothing.**

The app drives `wpctl`, `pactl` or `amixer`, whichever is present. On a
device with no sound server, it drives ALSA, and ALSA has no way to
switch the default device at runtime. Check what is installed:

```sh
which wpctl pactl amixer
```

**Wi-Fi does not show up in the drawer.**

NetworkManager has to be running and reachable:

```sh
systemctl status NetworkManager
nmcli device status
```

**The screen does not turn off.**

Screen power needs `wlopm` on Wayland or `xset` on X11:

```sh
which wlopm xset
```

If neither is there, set a command in the configuration instead — see
[configuration.md](configuration.md).

**The panel is slow.**

The app is not the usual culprit; the webview is. Check that the
WebKitGTK process is not being restarted repeatedly:

```sh
journalctl -u ha-panel.service | grep -i "link down"
```

That message means the panel lost its connection to Home Assistant.
