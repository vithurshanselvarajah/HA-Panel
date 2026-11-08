# Getting started

What HA-Panel does, and how to have it working on a panel in about ten
minutes.

## What it is

A wall-mounted display that shows the Home Assistant dashboard fullscreen,
with a control drawer for the things a panel needs: volume, mute, screen
power, brightness, Wi-Fi, and the audio input and output endpoints.

It runs on a device with 2 GB of RAM and 4 GB of storage. The
application is about 5 MB and the frontend about 70 KB.

## The five minute version

```sh
# On the device.
sudo apt install ./HA-Panel_0.1.0_amd64.deb
sudo sh packaging/install.sh
```

The installer asks for two things: the address of your Home Assistant,
and the name of the panel. Then it starts the service.

That is the whole installation. The panel shows your dashboard.

## The drawer

The panel is the Home Assistant dashboard and nothing else. To reach the
controls, tap the narrow strip down the right-hand edge of the screen.

The drawer slides in from the right and has four tabs.

**Controls** — volume, mute, the audio output and input, and the screen.
Every control that the device cannot do is greyed out rather than
missing, so you can see what the panel is capable of.

**Settings** — the Home Assistant address, the display commands, the
voice link, and the kiosk's own behaviour. Changes are saved when you
press Save.

**Network** — the Wi-Fi networks in range, and a button to join one.

**Logs** — the panel's log. It is kept in memory and normally holds
nothing; a problem is the only thing that fills it.

## Logging in

The panel shows the Home Assistant login screen on the first boot unless
you give it a long-lived access token. Without one, you would be typing
a password into the panel after every reboot, which is not what a kiosk
is for.

In Home Assistant, click your name in the bottom left, then **Security**,
then **Long-lived access tokens**, then **Create token**. Copy the token
it gives you.

```sh
sudo install -o ha-panel -g ha-panel -m 0600 /dev/null /etc/ha-panel/secrets.json
sudo tee /etc/ha-panel/secrets.json >/dev/null <<'EOF'
{
  "ha_long_lived_token": "eyJhbGciOiJIUzI1NiJ9..."
}
EOF
sudo systemctl restart ha-panel.service
```

The panel is now logged in.

## Making it start at boot

It already does — the installer runs `systemctl enable`. To check:

```sh
systemctl is-enabled ha-panel.service
```

## If there is no desktop on the device

The panel needs a display server. If the device has one — a Phosh shell
on the Athena image, or a normal desktop — nothing more is needed.

If it does not, the package can start a bare wlroots compositor:

```sh
sudo systemctl enable --now ha-panel.target
sudo apt install cage
```

The target starts the compositor and then the panel.

## What the panel writes to the disk

Almost nothing, which is deliberate: the storage is eMMC and the panel is
expected to run for months untouched.

| | |
| --- | --- |
| `/etc/ha-panel/config.json` | Written when you press Save. |
| `/etc/ha-panel/secrets.json` | Written when you provision it. |
| `/var/lib/ha-panel/` | The EchoMuse credentials, if the voice link is on. |
| `/var/cache/ha-panel/` | The WebKitGTK cache. |

The panel's log is in memory. It reaches the journal only when something
fails.

## Where to go next

| | |
| --- | --- |
| [installation.md](installation.md) | Installing, provisioning, upgrading, troubleshooting. |
| [development.md](development.md) | Running it on your own machine, without installing anything. |
| [configuration.md](configuration.md) | Every setting, and what it does. |
| [architecture.md](architecture.md) | How it is built and why. |
| [sidecar.md](sidecar.md) | The EchoMuse voice link. |
| [packaging.md](packaging.md) | Building the packages. |
| [ci-rules.md](ci-rules.md) | The rules the CI enforces. |
