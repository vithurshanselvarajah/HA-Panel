# HA-Panel

A lightweight Home Assistant kiosk for wall-mounted displays.

It shows the Home Assistant web UI fullscreen, and gives you a control drawer
for the things a kiosk panel actually needs: volume, mute, screen power,
brightness, Wi-Fi, and the audio input and output endpoints.

The device may also join an EchoMuse controller as a voice endpoint (wake word,
microphone, speaker). That part lives in a small Go sidecar.

## Design goals

| Goal | How it is met |
| --- | --- |
| Small RAM | One Rust process plus one Go sidecar. The system WebKitGTK runtime is shared, not bundled. No embedded browser. |
| Small storage | ~12 MB installed. The frontend is a few tens of kilobytes of JS with no framework runtime beyond Svelte. |
| Almost no writes | Settings are written only when they change. Logs live in a memory ring buffer and reach the disk only if something failed. |
| Runs with or without a desktop | Two installation variants: one that starts its own wlroots compositor, one that attaches to a session that already exists. |
| Debian and Alpine | `.deb` for Debian/Ubuntu, `.apk` for Alpine (musl, static). |

## Documentation

**Using it**

- [docs/getting-started.md](docs/getting-started.md) — the five minute version.
- [docs/installation.md](docs/installation.md) — installing, provisioning, upgrading, troubleshooting.
- [docs/configuration.md](docs/configuration.md) — every setting, and what it does.

**Building it**

- [docs/development.md](docs/development.md) — running it locally, without installing anything.
- [docs/packaging.md](docs/packaging.md) — building the `.deb` and the `.apk`.
- [docs/architecture.md](docs/architecture.md) — how the pieces fit and why.
- [docs/sidecar.md](docs/sidecar.md) — the EchoMuse voice link.
- [docs/ci-rules.md](docs/ci-rules.md) — the rules the CI enforces.

## Building

```sh
# Frontend
npm install
npm run build

# Rust app (also builds the Go sidecar from vendored source)
cargo build --manifest-path src-tauri/Cargo.toml --release
```

The Tauri CLI drives the bundle step:

```sh
npx tauri build --config src-tauri/tauri.conf.json
```

The scripts do all three parts together:

```sh
./scripts/build.sh                      # everything for this machine
./scripts/build-alpine.sh --arch arm64  # the Alpine package, in a container
```

The CI builds every installer on every push and attaches them to the run,
so a build failure is debuggable from the pull request that caused it.
See [docs/packaging.md](docs/packaging.md).

## Repository layout

```
src-tauri/     Rust application: windows, device control, sidecar supervisor
ui/            Svelte 5 frontend: drawer, settings, on-screen keyboard
sidecar/       Go EchoMuse adapter, plus the vendored SDK it builds against
packaging/     Debian and Alpine packaging, systemd units, install scripts
scripts/       Build scripts, including the Alpine container build
tools/         The CI guard
tests/         The CI guard's own tests
ci/            The pinned vulnerability scanner
docs/          Architecture and configuration notes
```
