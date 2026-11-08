# Development

How to build, run and test HA-Panel on your own machine, without
installing a package and without touching the system's configuration.

## What you need

| Tool | Version | Why |
| --- | --- | --- |
| Rust | 1.90 or newer | The application. Tauri's minimum. |
| Node.js | 24 | The frontend build. |
| Go | 1.24 | The sidecar. |
| GTK and WebKitGTK headers | 4.1 | The webview. |

On Debian or Ubuntu:

```sh
sudo apt-get install -y --no-install-recommends \
  libwebkit2gtk-4.1-dev libgtk-3-dev libssl-dev pkg-config \
  nodejs npm golang-go
```

On Alpine:

```sh
sudo apk add --no-cache \
  build-base ca-certificates cargo go nodejs npm pkgconf \
  rustup linux-headers webkit2gtk-4.1-dev gtk+3.0-dev
```

Alpine's Rust is 1.91, which is fine. The Alpine build uses `rustup`
rather than Alpine's `rust` package so that both builds use the same
compiler — see [packaging.md](packaging.md).

## Running it locally

There are two ways to run the app, and they differ in where the frontend
comes from.

### The quick way — the embedded frontend

This is the one to use. It is what the shipped application does, so what
you see is what a device shows.

```sh
# 1. The frontend.
cd ui
npm install
npm run build
cd ..

# 2. The sidecar.
cd sidecar
CGO_ENABLED=0 go build -trimpath -ldflags="-s -w" \
  -o ../src-tauri/binaries/ha-panel-sidecar-$(rustc -vV | sed -n 's/^host: //p') .
cd ..

# 3. The application.
cd src-tauri
cargo build --features tauri/custom-protocol
```

Then run it:

```sh
./target/debug/ha-panel
```

> **The `tauri/custom-protocol` feature matters.** Without it, the app
> runs in development mode and loads its frontend from the Vite server on
> `localhost:5173` — which is not running, so the drawer and the keyboard
> will be blank. With the feature, the frontend is embedded in the binary
> and everything works on its own.

The frontend has to be built before the Rust build, because the Rust build
embeds `ui/dist`. If you change the frontend, rebuild it and then rebuild
the Rust application.

### The development way — the Vite server

Use this when you are working on the frontend and want the changes to
appear without a rebuild.

```sh
# Terminal one: the frontend, with hot reload.
cd ui
npm run dev
```

```sh
# Terminal two: the application.
cd src-tauri
cargo run
```

Now the app loads its frontend from the Vite server, and a change to a
Svelte component appears as soon as you save it. Only the frontend is
reloaded — the Rust side needs `cargo run` again.

### Keeping the device's configuration out of it

The app reads `/etc/ha-panel/` and writes to `/var/lib/ha-panel/` by
default. On a development machine those are the wrong places: the first
one needs root to create, and the second one is the device's.

Two environment variables move them:

| Variable | What it changes | Default |
| --- | --- | --- |
| `HA_PANEL_CONFIG_DIR` | Where `config.json` and `secrets.json` live | `/etc/ha-panel` |
| `HA_PANEL_STATE_DIR` | Where the state lives — the EchoMuse credentials, the sidecar's socket | `/var/lib/ha-panel` |
| `HA_PANEL_LOG` | The lowest level that reaches stderr: `error`, `warn`, `info`, `debug`, `trace` | `warn` |

So a complete local run, with nothing written outside your home
directory:

```sh
mkdir -p ~/.config/ha-panel/state

cat > ~/.config/ha-panel/config.json <<'EOF'
{
  "ha": {
    "url": "http://homeassistant.local:8123",
    "dashboard_path": "/lovelace",
    "fullscreen": true
  },
  "kiosk": { "show_handle": true, "on_screen_keyboard": true },
  "audio": { "backend": "auto", "volume": 40, "muted": false },
  "display": { "screen_power": "auto" },
  "echomuse": { "enabled": false }
}
EOF

HA_PANEL_CONFIG_DIR=~/.config/ha-panel \
HA_PANEL_STATE_DIR=~/.config/ha-panel/state \
HA_PANEL_LOG=info \
  ./src-tauri/target/debug/ha-panel
```

With `HA_PANEL_LOG=info` the app tells you where it read its
configuration from and which display server it found:

```
[INFO ] ha_panel_lib: ha-panel starting
[INFO ] ha_panel_lib: configuration from /home/you/.config/ha-panel/config.json; display server is wayland
```

That first line is the quickest way to confirm the override worked.

### The whole thing in one command

`scripts/build.sh` does the frontend, the sidecar and the Rust
application, in that order, and works out the sidecar's name for your
target by itself:

```sh
./scripts/build.sh
```

Add `--target aarch64-unknown-linux-musl` to build for another
architecture, and `--debug` for an unoptimised build.

## Running the sidecar on its own

The sidecar is a separate program and it can be run without the app.
That is the quickest way to see what it is doing.

```sh
cd sidecar
go build -o /tmp/ha-panel-sidecar .

# Ask it a question over its standard input.
echo '{"jsonrpc":"2.0","method":"ping","id":1}' \
  | /tmp/ha-panel-sidecar --socket ""
```

```json
{"jsonrpc":"2.0","result":{"pong":true},"id":1}
```

To use its Unix socket instead — which is what the other tools on a
device use — give it a path and keep its standard input open:

```sh
# The sidecar stops when its standard input closes, so hold it open.
sleep 30 | /tmp/ha-panel-sidecar --socket /tmp/sidecar.sock &

# Then ask it something. `nc -q 1` closes the connection after a
# second, rather than waiting for more input that will never come.
echo '{"jsonrpc":"2.0","method":"state","id":1}' | nc -q 1 -U /tmp/sidecar.sock
```

```json
{"jsonrpc":"2.0","result":{"connected":false,"approved":false,"phase":"pending",...},"id":1}
```

The socket outlives a connection that closes, so a health check that
connects once does not take it away from the next tool. The sidecar
stops when its standard input closes, unless it is serving a socket.

The methods are listed in [sidecar.md](sidecar.md).

## The tests

Each part has its own tests, and each can be run on its own.

```sh
# The Rust application.
cd src-tauri && cargo test

# The sidecar.
cd sidecar && go test ./...

# The frontend.
cd ui && npm run test

# The CI guard.
python3 -m unittest discover -s tests
```

The sidecar's tests include one that builds the binary and starts it, so
`go test ./...` needs a Go toolchain on the path — which it does.

The CI guard's tests matter more than they look: a guard that does not
catch anything passes every build and looks like a check. See
[ci-rules.md](ci-rules.md).

## Everything the CI checks

The CI runs all of this, plus the linters and the packaging checks:

```sh
# The Rust formatting and the linter.
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# The Go formatting and the linter.
cd ../sidecar
gofmt -l .
go vet ./...

# The frontend's types.
cd ../ui
npm run check

# The CI rules.
cd ..
python3 tools/check_workflows.py

# The workflow files themselves.
actionlint
```

`actionlint` is not installed by default. The CI downloads it from its
release tarball and checks it against a pinned sha256, which is the way
to run it here too:

```sh
VERSION=1.7.12
SHA256=8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8
curl -fsSLo actionlint.tgz \
  "https://github.com/rhysd/actionlint/releases/download/v${VERSION}/actionlint_${VERSION}_linux_amd64.tar.gz"
echo "${SHA256}  actionlint.tgz" | sha256sum -c -
tar xzf actionlint.tgz actionlint
./actionlint
```

## The code style

**No comments in the code.** The project's rule, and
`tools/strip_comments.py` removes them. It is string-aware, so it leaves
alone anything that merely looks like a comment — a URL in a string, a
Rust raw string, a shebang, a `# shellcheck` directive.

Run it after adding code:

```sh
python3 tools/strip_comments.py .
```

Then run `cargo fmt` and `gofmt -w`, because removing a comment leaves a
blank line behind and the formatters will want it gone.

**The Rust and the Go are formatted by their own tools.** `cargo fmt` and
`gofmt -w` are not optional.

**The frontend is formatted by hand.** There is no Prettier configured;
match the surrounding code.

## How the pieces fit

```
ui/                    the frontend: the drawer, the keyboard, the handle
  └─ src/lib/api.ts    every call into the Rust side, typed

src-tauri/             the Rust application
  ├─ src/lib.rs        the commands, and the setup
  ├─ src/app.rs        the windows
  ├─ src/config.rs     the configuration and the secrets
  ├─ src/logging.rs    the in-memory log
  ├─ src/ha.rs         the token injection
  ├─ src/keyboard.rs   typing into the page
  ├─ src/audio.rs      volume, mute, endpoints
  ├─ src/display.rs    screen power, brightness
  ├─ src/network.rs    Wi-Fi
  └─ src/sidecar/      the sidecar's process and its protocol

sidecar/               the Go sidecar
  ├─ main.go           the flags and the transports
  └─ internal/
     ├─ rpc/           the JSON-RPC layer
     └─ voice/         the device role
```

[architecture.md](architecture.md) explains why it is shaped this way.

## The vendored SDK

`sidecar/third_party/echomuse-sdk` is a copy of the EchoMuse SDK's Go
client, pinned to a commit. It is somebody else's code and it is not
modified.

`sidecar/third_party/echomuse-sdk/PROVENANCE.md` records the commit and
how to refresh the copy.

## Things that will trip you up

**The frontend has to be built before the Rust application.** The Rust
build embeds `ui/dist`. A stale `ui/dist` gives you an old frontend
with no error.

**`cargo run` and `cargo build --features tauri/custom-protocol` behave
differently.** The first loads the frontend from Vite; the second
embeds it. See above.

**The app needs a display.** It opens four windows. Without a display
server it will fail to start, and the error goes to stderr rather than
to the log — the log is in memory and only reaches stderr at `warn` and
above.

**`cargo fmt` and `gofmt` after stripping.** The stripper leaves blank
lines.

**The `ha-panel` user does not exist on your machine.** The app does not
need it; only the installed service does.

**The sidecar's binary name carries the target triple.** The Tauri
bundler picks the right one per architecture by that name, so the name
has to match:

```sh
ls src-tauri/binaries/
# ha-panel-sidecar-x86_64-unknown-linux-gnu
```

## Where the state goes on a device

| Path | What |
| --- | --- |
| `/etc/ha-panel/config.json` | The configuration. Group-writable, so the kiosk user can save its settings. |
| `/etc/ha-panel/secrets.json` | The long-lived token. Owner-only. |
| `/var/lib/ha-panel/echomuse/` | The EchoMuse link credentials. |
| `/run/ha-panel/sidecar.sock` | The sidecar's socket. |
| `/var/cache/ha-panel/` | The WebKitGTK cache. |

Nothing else is written. The app's own log is in memory and reaches the
disk only if something fails; systemd's journal is what usually holds it.
