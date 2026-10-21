# Design

## Context

Today the extension runs `tiltBin` (default `tilt`), adds `/proto/shims` and `/proto/bin` to `paths`, and sets `PROTO_TILT_VERSION` when the toolchain is configured. Verified with moon 2.6.2 and proto 0.62/0.63 in a scratch workspace, using a scratch `PROTO_HOME`:

- **The host ignores `paths` when looking up the command.** warpgate's `exec_command` resolves a bare command with `find_command_on_path` against moon's own `PATH`, and only puts `input.paths` on the child process's `PATH`. With Tilt installed only by the toolchain: `Command or script tilt does not exist`.
- **proto's `tilt` shim needs a plugin registered in `.prototools`.** moon installs Tilt through the toolchain's bundled plugin, but proto doesn't record it anywhere (`shims/registry.json` is `"tilt": {}`). So the shim fails with `tilt is not a built-in plugin` unless `.prototools` registers one. proto 0.63+ also requires that `.prototools` to be trusted (`proto::tool::untrusted_plugin`).
- **`moon bin tilt` can't replace this.** It shells out to the `proto` CLI and fails the same way without a `.prototools` plugin, even for an exact, installed version.
- **Absolute paths work.** `tiltBin: "/proto/bin/tilt"` ran real Tilt. The host maps `/proto` to `PROTO_HOME`, and the toolchain installs the binary at `/proto/tools/tilt/<version>/tilt`, recording installed versions in `/proto/tools/tilt/manifest.json` (`installed_versions`).
- **`versionFromPrototools` already works.** moon fills in the toolchain's `version` from `.prototools` when it's unset, for every toolchain plugin (`ToolchainsConfigExt::inherit_proto_versions_for_plugins`, on by default). Tasks used the inherited version, and `load_toolchain_config("tilt")` returned it to the extension.
- **A `.prototools` plugin shares installs.** With the appthrust TOML plugin registered and trusted, the shim ran the 0.37.8 that the toolchain's plugin had installed. Both use the same `tools/tilt/<version>/tilt` layout.

## Goals / Non-Goals

**Goals:**
- Run the toolchain's pinned Tilt regardless of the user's `PATH`, shims, or `.prototools` plugins.

**Non-Goals:**
- Installing Tilt from the extension. moon installs toolchains (`moon setup`, or before a task that uses the toolchain), and the extension has no host function for it.
- Warning about a different `tilt` plugin in `.prototools`, as #3 suggests. Once the extension no longer uses the shim, that plugin can't affect it, so documenting it is enough.
- Windows (unsupported by the extension).

## Decisions

**Resolve the binary in the extension, from proto's install directory.** The extension reads the toolchain's version (as today), reads `installed_versions` from `/proto/tools/tilt/manifest.json`, and picks the highest installed version that satisfies it. Then it runs `/proto/tools/tilt/<version>/tilt` by absolute path. Alternatives:
- *`moon bin tilt`*: fails without a `.prototools` plugin (see Context).
- *Keep shims, document registering a plugin*: makes every user register and trust a second locator for a tool the toolchain already installs.
- *`PROTO_TILT_VERSION` with `/proto/bin/tilt`*: `bin/tilt` links one version and ignores the variable.

**Match versions with `version_spec`.** It already handles the forms the toolchain accepts:
- exact versions (`0.37.8`)
- partial versions and requirements (`0.37`, `~0.37`, `>=0.36`)
- ranges

Aliases (`latest`, `stable`) resolve to the highest installed version; canary isn't supported and is reported as not installed. `version_spec` is already compiled into the module through moon's crates, so it becomes a direct dependency without new code. The matching is a pure function, unit-tested natively.

**Precedence:**
1. An explicit `tiltBin` other than `tilt` is used as-is. That keeps an escape hatch for users who run a custom Tilt.
2. Otherwise, a configured toolchain with a version uses the installed binary.
3. Otherwise, `tilt` comes from `PATH`, as today.

`PROTO_TILT_VERSION` and the shim paths are dropped. The resolved version's directory is put first on the child's `PATH`, so anything Tilt starts that calls `tilt` gets the same version.

**Fail clearly when the version isn't installed.** The error names the requested version, the installed versions, and `moon setup`. A silent fallback to `PATH` would run a different Tilt than the one pinned.

## Risks / Trade-offs

- [proto changes its install layout or manifest] → Both are long-standing proto conventions used by every tool. The error message names the expected path, and a unit test pins the manifest shape.
- [Users relied on the shim to auto-install a missing version] → With a `.prototools` plugin and proto's `auto-install`, the shim could install on demand. Now the extension asks for `moon setup` instead. That's one command, and it's the documented way to install toolchains.
- [A custom `tiltBin` of exactly `tilt` can't override the toolchain] → It's indistinguishable from the default. Setting the absolute path works.

## Migration Plan

Nothing to migrate. Release as `fix(tilt)` (patch). Workspaces that registered a `tilt` plugin in `.prototools` only for the extension can remove it.
