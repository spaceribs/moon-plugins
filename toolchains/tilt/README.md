# Tilt toolchain

A moon toolchain that installs and pins [Tilt](https://tilt.dev). It bundles the [Tilt proto tool](../../tools/tilt), so moon downloads the configured version through proto, verifies its checksum, and puts it on `PATH` for any task whose command is `tilt`. The [Tilt extension](../../extensions/tilt) runs the same pinned version.

Requires moon 2.6 or newer (tested with 2.6.2) and proto 0.60 or newer, which moon 2.6 needs to install tools.

## Setup

```yaml
# .moon/toolchains.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_toolchain"
  version: "0.37.8"
```

### Pinning the version in `.prototools`

If you already pin tools in `.prototools`, leave `version` out. moon then reads the `tilt` version from there, through its `versionFromPrototools` setting, which is on by default:

```toml
# .prototools
tilt = "0.37.8"
```

```yaml
# .moon/toolchains.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_toolchain"
  # versionFromPrototools: false   # to ignore .prototools
```

A `version` in `.moon/toolchains.yml` takes precedence over `.prototools`.

## Running Tilt

The toolchain only installs and pins Tilt. To run your workspace in Tilt, use the [Tilt extension](../../extensions/tilt), which manages a single Tilt instance and selects what runs in it by project:

```sh
moon ext tilt -- up web
```

The extension runs the binary this toolchain installed, whatever `tilt` is on your `PATH`. For a partial version like `"0.37"`, it picks the highest installed match. If none is installed, it asks you to run `moon setup`, which installs all toolchains. moon also installs Tilt before any task whose command is `tilt`.

### A `tilt` plugin in `.prototools`

moon and the extension don't need a `tilt` plugin in `.prototools`. If you register one, for example to run `tilt` through proto's shims in your shell, it only affects your shell. moon's tasks and the extension use this toolchain's bundled plugin. Both install into `~/.proto/tools/tilt/<version>`, so they share installs. On proto 0.63 or newer, a `.prototools` that registers plugins must be trusted with `proto trust`, or the shims refuse to load it.

## In container images

moon loads every toolchain configured in `.moon/toolchains.*` whenever it runs, including inside images built from `moon docker file`, where no Tilt runs. In a container capped at 1 CPU, loading this plugin took about 4.5 seconds of each cold `moon run` start. That's moon behaviour and this plugin can't skip it. In image-heavy workspaces, allow for it in liveness probes. Setting `MOON_SKIP_SYNC_WORKSPACE=true` in the image's start stage at least avoids loading the Tilt extension there too (see the extension's [Plugins inside the image build](../../extensions/tilt/README.md#plugins-inside-the-image-build)).

## Configuration

| Setting | Default | Description |
| --- | --- | --- |
| `version` | none | Tilt version to install, for example `"0.37.8"` or `"0.37"`. Without it, moon uses the `tilt` version in `.prototools`. |
| `versionFromPrototools` | `true` | moon's setting for every toolchain: `false` ignores `.prototools`, or a string reads a different tool ID from it. |

## Supported platforms

Linux (x64, arm64, armv7; glibc and musl/Alpine for x64 and arm64), macOS (x64, arm64), and Windows (x64). These match what Tilt publishes.
