# moon-plugins

Custom [WASM plugins](https://moonrepo.dev/docs/guides/wasm-plugins) for [moon](https://moonrepo.dev/moon) and [proto](https://moonrepo.dev/proto), built, tested, and released from a moon workspace.

| Plugin | Kind | Crate | Description |
| --- | --- | --- | --- |
| [Tilt](./extensions/tilt) | moon extension | `tilt_extension` | Exposes the moon task graph as Tilt resources (`moon ext tilt -- up`). |
| [Tilt](./toolchains/tilt) | moon toolchain | `tilt_toolchain` | Installs and pins Tilt for tasks and the extension. |
| [Tilt](./tools/tilt) | proto tool | `tilt_tool` | `proto install tilt`. Also bundled into `tilt_toolchain`. |

## Using the plugins

Reference a plugin with a `github://` locator. moon downloads its `.wasm` from this repo's GitHub releases:

```yaml
# .moon/extensions.yml
tilt:
  plugin: "github://spaceribs/moon-plugins/tilt_extension"                               # latest
  # plugin: "github://spaceribs/moon-plugins/tilt_extension@tilt_extension-v0.1.0"       # pinned (full release tag)
```

Each plugin's README covers setup: start with the [Tilt extension's quick start](./extensions/tilt#quick-start).

**Supported platforms:** Linux and macOS. The proto plugin (`tilt_tool`) also installs Tilt on Windows, but the extension's container builds need `sh`, and CI doesn't run on Windows.

## Layout

```
extensions/<name>/   moon extensions   (moon ext <id>)
toolchains/<name>/   moon toolchains   (.moon/toolchains.yml)
tools/<name>/        proto tools       (.prototools [plugins.tools])
examples/<name>/     standalone moon workspaces for manual end-to-end testing
```

Each plugin is a `cdylib` crate compiled to `wasm32-wasip1`. Pure logic lives in modules that compile natively. The WASM entry points are behind the default `wasm` feature.

## Development

You need [proto](https://moonrepo.dev/docs/proto/install) **0.60 or newer** (moon 2.6 requires it) and rustup. moon and Rust are pinned in `.prototools` and `rust-toolchain.toml`.

```sh
proto install                     # moon + Rust (incl. the wasm32-wasip1 target)
moon run :build                   # release .wasm builds (used by tests and dogfooding)
moon run :test                    # native unit tests + WASM integration tests
moon run :lint root:format        # clippy (-D warnings) + rustfmt
```

Integration tests load the built `.wasm` into a sandboxed host via `moon_pdk_test_utils` / `proto_pdk_test_utils`, which is why `test` depends on `build`. The harness looks in `target/wasm32-wasip1/release` before `debug`. So if you run `cargo test` by hand, rebuild with `--release` first, or a stale release build will be tested. Some tool tests download real Tilt releases.

### Trying a plugin locally

Point a workspace at the built file with a `file://` locator. For example, [`examples/tilt-demo`](./examples/tilt-demo) does this:

```yaml
# .moon/extensions.yml
tilt:
  plugin: "file://../../../target/wasm32-wasip1/release/tilt_extension.wasm"
```

`file://` paths are for local development only. They must point inside the consuming workspace if moon also runs inside container builds (see the Tilt extension's container docs). moon caches the project graph, so after rebuilding a plugin, delete `.moon/cache` in the consuming workspace.

### Adding a plugin

1. Create `extensions/<name>`, `toolchains/<name>`, or `tools/<name>`. Copy the Tilt crate as a template: `crate-type = ["cdylib", "lib"]` and a `wasm` default feature.
2. Register it in `.moon/workspace.yml`. Cargo picks it up automatically through the workspace globs.
3. Add `CHANGELOG.md` with an `## Unreleased` heading.

## Releasing

Releases are per plugin and driven by tags:

1. Bump `version` in the crate's `Cargo.toml` and rename `## Unreleased` in its `CHANGELOG.md`.
2. Tag `<crate>-v<version>` and push the tag:
   ```sh
   git tag tilt_extension-v0.1.0 && git push origin tilt_extension-v0.1.0
   ```
3. [`release.yml`](./.github/workflows/release.yml) uses [`moonrepo/build-wasm-plugin`](https://github.com/moonrepo/build-wasm-plugin) to build, `wasm-opt`, strip, and checksum the plugin. It attaches the result to a GitHub release and publishes it to ghcr.io.

Consumers then reference it with a `github://` locator ([see above](#using-the-plugins)). A pinned locator uses the full release tag (`tilt_extension@tilt_extension-v0.1.0`), because moon looks the tag up verbatim.
