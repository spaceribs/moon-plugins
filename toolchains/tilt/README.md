# Tilt toolchain

A moon toolchain that installs and pins [Tilt](https://tilt.dev). It bundles the [Tilt proto tool](../../tools/tilt), so moon downloads the configured version through proto (with checksum verification) and puts it on `PATH` for any task whose command is `tilt`. When this toolchain is configured, the Tilt extension also runs that pinned version.

Requires moon 2.6 or newer (tested with 2.6.2), with proto 0.60 or newer installed (moon 2.6 needs it to install tools).

## Setup

```yaml
# .moon/toolchains.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_toolchain"
  version: "0.37.8"
```

## Running Tilt

The toolchain only installs and pins Tilt. Tasks whose command is `tilt` get the pinned version on `PATH`. To run your workspace in Tilt, use the [Tilt extension](../../extensions/tilt). It manages a single Tilt instance for the whole workspace, and selects what runs in it by project:

```sh
moon ext tilt -- up web
```

## Configuration

| Setting | Default | Description |
| --- | --- | --- |
| `version` | none | Tilt version to install, for example `"0.37.8"` or `"0.37"`. |

## Supported platforms

Linux (x64, arm64, armv7; glibc and musl/Alpine for x64 and arm64), macOS (x64, arm64), and Windows (x64). These match what Tilt publishes.
