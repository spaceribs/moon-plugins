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

### Pinning the version in `.prototools`

If you already pin tools in `.prototools` (for `proto install` and shells outside moon), leave `version` out and moon uses the `tilt` version from there, through moon's `versionFromPrototools` setting, on by default for every toolchain:

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

The toolchain only installs and pins Tilt. Tasks whose command is `tilt` get the pinned version on `PATH`. To run your workspace in Tilt, use the [Tilt extension](../../extensions/tilt). It manages a single Tilt instance for the whole workspace, and selects what runs in it by project:

```sh
moon ext tilt -- up web
```

The extension runs the binary this toolchain installed for the pinned version (the highest installed version that matches, for a version like `"0.37"`), whatever `tilt` is on your `PATH`. If that version isn't installed yet, the extension stops and asks you to run `moon setup`, which installs all toolchains. moon also installs it before any task whose command is `tilt`.

### A `tilt` plugin in `.prototools`

You don't need to register a `tilt` plugin in `.prototools` for moon or the extension. If you do (for example, to run `tilt` from your shell through proto's shims), it only affects `tilt` in your shell: moon's tasks and the extension use this toolchain's bundled plugin and installed binary. Both install into the same proto directory (`~/.proto/tools/tilt/<version>`), so they share installs. On proto 0.63 or newer, a `.prototools` that registers plugins must be trusted (`proto trust`), or proto's shims refuse to load it.

## Configuration

| Setting | Default | Description |
| --- | --- | --- |
| `version` | none | Tilt version to install, for example `"0.37.8"` or `"0.37"`. Without it, moon uses the `tilt` version in `.prototools`. |
| `versionFromPrototools` | `true` | moon's setting for every toolchain: `false` ignores `.prototools`, or a string reads a different tool ID from it. |

## Supported platforms

Linux (x64, arm64, armv7; glibc and musl/Alpine for x64 and arm64), macOS (x64, arm64), and Windows (x64). These match what Tilt publishes.
