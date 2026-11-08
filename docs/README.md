# The documentation

| | |
| --- | --- |
| [getting-started.md](getting-started.md) | What the panel is, and the five minute version. |
| [installation.md](installation.md) | Installing on Debian and Alpine, provisioning, upgrading, troubleshooting. |
| [configuration.md](configuration.md) | Every setting, and what it does. |
| [development.md](development.md) | Running it on your own machine, without installing anything. |
| [architecture.md](architecture.md) | How it is built, and why it is built that way. |
| [packaging.md](packaging.md) | Building the `.deb` and the `.apk`. |
| [sidecar.md](sidecar.md) | The EchoMuse voice link. |
| [ci-rules.md](ci-rules.md) | The rules the CI enforces. |

## Which one do I want?

**I want to put it on a panel.** → [getting-started.md](getting-started.md),
then [installation.md](installation.md).

**I want to change the code.** → [development.md](development.md). It has
the local run instructions, including how to do it without touching
`/etc` or needing root.

**I want to understand how it works.** → [architecture.md](architecture.md).

**Something is wrong on a device.** → the troubleshooting section of
[installation.md](installation.md).

**I want to add a setting.** → [configuration.md](configuration.md) for
what the settings mean, and [development.md](development.md) for how to
run it while you are working on it.
