# Proposal

## Why

When the Tilt toolchain is configured, the extension is meant to run the pinned Tilt version, but it only does so by accident. moon's plugin host looks up a bare `tilt` on the user's own `PATH`, so the extension gets whatever that finds:
- With Tilt only installed by the toolchain, `tilt` isn't found at all.
- With a typical proto setup, the user's `PATH` finds proto's `tilt` shim first. The shim fails (`tilt is not a built-in plugin`) unless `.prototools` registers a `tilt` plugin, which proto 0.63+ also requires the user to trust.
- Only workspaces that register (and trust) a `tilt` plugin in `.prototools` work, and then they run Tilt through *that* plugin, not the toolchain's.

Issue #3 asks which plugin wins in that last case, and for `versionFromPrototools`. Testing with moon 2.6.2 showed `versionFromPrototools` already works for Tilt, because moon provides it for every toolchain plugin. It just isn't documented.

## What Changes

- When the toolchain is configured, the extension runs the Tilt binary the toolchain installed, by absolute path: the highest installed version that satisfies the configured version (or the version moon inherited from `.prototools`). It no longer depends on the user's `PATH`, proto's shims, or a `tilt` plugin in `.prototools`.
- If no installed version satisfies it, the extension fails with an error naming the version and telling the user to run `moon setup`, instead of a proto or "command not found" error.
- Without the toolchain, the extension still runs `tilt` from `PATH` (unchanged).
- Documentation for the toolchain and extension:
  - pinning Tilt in `.prototools` (moon's `versionFromPrototools`, and that `version` in `.moon/toolchains.yml` takes precedence)
  - how a `tilt` plugin registered in `.prototools` interacts: it only affects `tilt` in your shell, installs are shared, and proto 0.63+ requires trusting it

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-toolchain`:
  - "The extension uses the pinned version" changes from running proto's shims with `PROTO_TILT_VERSION` to running the installed binary, and adds the not-installed error.
  - A new requirement covers inheriting the version from `.prototools`.

## Impact

- **Code**: `extensions/tilt/src/tilt_ext.rs` (how the Tilt command is built), plus a pure, unit-tested version-resolution helper. The extension gains a direct `version_spec` dependency, which moon's crates already compile into the module.
- **Behavior**: workspaces with the toolchain run the pinned Tilt even when `tilt` isn't on `PATH` or `.prototools` registers no `tilt` plugin. Workspaces that register their own `tilt` plugin in `.prototools` now run the binary at the same install path, so no difference in practice. Released as `fix(tilt)`, patch release.
- **Docs**: `toolchains/tilt/README.md` and the extension README. Resolves issue #3.
