# Provenance

This directory is a plain copy of the EchoMuse SDK's Go client, vendored so
that HA-Panel builds without a network fetch and without a submodule.

## Source

| | |
| --- | --- |
| Repository | `https://github.com/wilbowes/EchoMuse-SDK.git` |
| Path in the repository | `clients/go` |
| Commit | `6865f81a77a2d1f3b7ba7f06bcbf6538bfbcf7a4` |
| Committed on | 2026-11-03 |
| Commit subject | `chore: update docs` |

## How it was copied

```sh
cp -r <checkout>/clients/go sidecar/vendor/echomuse-sdk
rm -rf sidecar/vendor/echomuse-sdk/.git
```

Nothing in the SDK was modified. The `go.mod` module path is unchanged, so
the SDK's own tests run as they did upstream.

## Updating it

```sh
cd <checkout-of-EchoMuse-SDK>
git checkout <new-commit>
cd -
rm -rf sidecar/vendor/echomuse-sdk
cp -r <checkout-of-EchoMuse-SDK>/clients/go sidecar/vendor/echomuse-sdk
rm -rf sidecar/vendor/echomuse-sdk/.git
```

Then update the commit hash in this file and run the sidecar's tests.

## Why a copy and not a dependency

The SDK is the other half of a protocol contract. A kiosk image is built
once and then runs for months on a device that may never see the network
again. Pinning the exact source it was tested against is worth more than
the ability to pull a newer version at build time.

The cost is that the copy has to be refreshed by hand. `PROVENANCE.md` is
the record of what was copied and when, so a refresh is a deliberate act
rather than something that happens by accident.

## License

The SDK is MIT licensed. Its `NOTICE.md` and `LICENSE` are kept alongside
this file.
