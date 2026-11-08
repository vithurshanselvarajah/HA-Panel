# The EchoMuse sidecar

The sidecar is a Go program that joins the panel to an EchoMuse controller.
It turns the device into a voice endpoint: a wake word, a microphone, a
speaker, and a status surface.

## What it is

The EchoMuse SDK is a protocol client, not a framework. It opens the three
WebSocket planes the contract defines and hands the application callbacks
for the things a controller asks of a device. The sidecar is that
application.

The sidecar is a separate process from the Rust app, and it is a separate
process on purpose:

- The SDK is Go and the app is Rust. A sidecar is the boundary between
  them.
- The Go runtime's memory is not shared with the app's. A panel with 2 GB
  of RAM cannot afford to have a Go heap and a Rust heap and a WebKitGTK
  heap all growing at once.
- The sidecar does not start at all unless the voice link is configured, so
  the default installation is two processes and not three.

## The protocol

JSON-RPC 2.0, over two transports.

**Over stdio.** The app owns the sidecar's lifetime. It writes requests to
the sidecar's standard input and reads responses from its standard output,
one JSON object per line. Closing the sidecar's input is how the app asks
it to stop.

**Over a Unix domain socket.** `/run/ha-panel/sidecar.sock` by default.
This is for other tools on the device: a health check, a provisioning
script, a debugging shell. It speaks the same protocol.

## The methods

| Method | Parameters | Result |
| --- | --- | --- |
| `ping` | none | `{"pong": true}` |
| `state` | none | The device state (below). |
| `capabilities` | none | `{"capabilities": ["mic", "speaker", ...]}` |
| `reconnect` | none | Asks the sidecar to redial the controller. |
| `startListening` | none | Opens the microphone. |
| `stopListening` | none | Closes the microphone. |
| `setVolume` | `{"level": 50}` | Sets the playback volume. |
| `setMuted` | `{"muted": true}` | Mutes or unmutes the playback. |
| `ring` | none | What the controller's ring is showing. |

### The state

````json
{
  "connected": true,
  "approved": true,
  "phase": "ready",
  "phase_reason": "joined the controller",
  "listening": false,
  "speaking": false,
  "thinking": false,
  "muted": false,
  "volume": 50,
  "capabilities": ["mic", "speaker", "leds"],
  "last_error": ""
}
````

### The error codes

| Code | Meaning |
| --- | --- |
| `-32601` | No such method. |
| `-32602` | The parameters were wrong. |
| `-32603` | The method exists but the call failed. |
| `-32000` | The sidecar is not in a state where the call makes sense. |

## An example

````sh
# Ask the sidecar what it is doing.
printf '{"jsonrpc":"2.0","method":"state","id":1}\n' | ha-panel-sidecar --socket ""

# Or through the socket, while the app owns the sidecar.
printf '{"jsonrpc":"2.0","method":"ping","id":1}\n' | nc -U /run/ha-panel/sidecar.sock
````

## What the sidecar announces

The sidecar announces the capabilities it actually implements:

| Capability | Meaning |
| --- | --- |
| `mic` | The device has a microphone. |
| `speaker` | The device has a speaker. |
| `leds` | The device can show the controller's ring animations. |
| `led_anim` | The device renders the animations itself. |
| `buttons` | The device has a button. |
| `pairing` | The device can ask to be admitted. |
| `wake_cue` | The device plays the wake-word cue. |
| `volume_cue` | The device plays the volume cue. |

It does **not** announce `oww_local_only`. The SDK's local wake word
scoring needs an ONNX Runtime library that is distributed separately, and
the sidecar does not require it. The device therefore streams the
microphone to the controller and the controller scores the wake word.

That is a deliberate choice, and it is the one the SDK's own rules point to:
a capability that is announced but not implemented produces a device that
scores, stays silent and looks broken. If the ONNX Runtime library is
added later, `oww_local_only` can be announced with it.

## The device identity

The device id must be stable across reboots, because the controller keys
the device's row, its configuration and its credentials on it. The sidecar
derives it from `/etc/machine-id`, falling back to the hostname. Set
`device_id` in the configuration to override it.

## The credentials

The link credential the controller issues is written to
`credentials_dir`, which defaults to `/var/lib/ha-panel/echomuse`. It is
written by the SDK's own atomic file writer.

The directory belongs to the `ha-panel` user and is not readable by
anything else.

## The vendored SDK

The SDK is copied into `sidecar/third_party/echomuse-sdk` rather than
fetched at build time. A kiosk image is built once and then runs for months
on a device that may never see the network again, and pinning the exact
source it was tested against is worth more than the ability to pull a
newer version at build time.

`sidecar/third_party/echomuse-sdk/PROVENANCE.md` records the commit the
copy came from and how to update it.

## Building it

````sh
cd sidecar
go build -o ../src-tauri/binaries/ha-panel-sidecar-x86_64-unknown-linux-gnu .
````

The sidecar builds without cgo. The SDK's ALSA backend loads `libasound.so.2`
at runtime rather than linking it, so the same binary runs on a glibc
system and on a musl one. The trade is that a sidecar built without cgo
has no ALSA backend at all — it reports "ALSA unavailable" and the voice
link carries no audio. A sidecar that needs audio is built with
`CGO_ENABLED=1`.

## Alpine

On Alpine the sidecar is built without cgo, so it has no ALSA backend. The
device's microphone and speaker are then driven by the Rust app's own
device-control layer rather than by the sidecar, and the voice link on
Alpine is limited to the ring animations and the controller's
configuration.

This is a real limitation and it is written down rather than hidden. The
way to lift it is to build the sidecar against Alpine's `alsa-lib-dev` with
cgo enabled, which the APKBUILD's `makedepends` already allows for.
