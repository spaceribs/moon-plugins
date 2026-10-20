# Tilt proto plugin

A [proto](https://moonrepo.dev/proto) WASM plugin for [Tilt](https://tilt.dev).

```toml
# .prototools
tilt = "0.37.8"

[plugins.tools]
tilt = "github://spaceribs/moon-plugins/tilt_tool"
```

```sh
proto install tilt
```

- Versions come from `tilt-dev/tilt` git tags.
- Prebuilt binaries come from GitHub releases and are verified against `checksums.txt`.
- On musl hosts (Alpine), the `linux-alpine` builds are used.

Supported platforms: Linux x64/arm64/armv7, Linux musl x64/arm64, macOS x64/arm64, and Windows x64.

Requires proto 0.50 or newer. Tested with proto 0.62.

moon users should use the [Tilt toolchain](../../toolchains/tilt) instead. It bundles this plugin.
