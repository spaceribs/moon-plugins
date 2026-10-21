# Proposal

## Why

moon runs natively on Windows, but the Tilt plugins only claim Linux and macOS support. The proto tool already installs Tilt on Windows. But the extension has never been run there, CI doesn't cover it, and a few parts are known not to work:
- The toolchain's installed binary is `tilt.exe` on Windows, which the extension's lookup doesn't handle.
- The WASM integration tests only build on Unix.

Windows users of moon should be able to run their workspace in Tilt the same way Linux and macOS users do.

## What Changes

- **Core features on native Windows (x64):**
  - moon tasks as Tilt resources
  - project selection (`up`/`focus`/`ci`/`down`)
  - change detection (watching inputs, gitignore handling)
  - the generated and main Tiltfiles, and templates
  - sync
  - the toolchain's pinned Tilt (`tilt.exe`)
- **Windows CI:** the Windows runner runs the unit and WASM integration tests. It also runs an end-to-end check with real moon and Tilt on the example workspace: the generated Tiltfile loads, resources come up, and change detection re-runs only what's affected. The same check runs on Linux and macOS.
- **Container images and Kubernetes deploys** stay Linux, macOS, and WSL2 only. Image builds run a POSIX shell command, and CI can't build Linux images on Windows. The docs say so, and WSL2 is the recommended way to use containers from Windows.
- **Docs, CI comments and project context** are updated from "Linux and macOS" to the new platform matrix.

## Capabilities

### New Capabilities

- `tilt-platform-support`: which platforms each part of the Tilt plugins supports (core features on Linux, macOS, and Windows; containers on Linux and macOS, including WSL2), and the platform-specific behavior that requires, such as running `tilt.exe`.

### Modified Capabilities

None. Existing requirements are platform-neutral, and this change makes them hold on Windows too.

## Impact

- **Code:**
  - `extensions/tilt/src/toolchain.rs` and `tilt_ext.rs`: the executable name from the host OS.
  - Possibly the built-in template's path handling, if Windows paths break Tilt's watch and ignore patterns (verified in CI first).
- **Tests:** the WASM integration tests run on Windows, with a `.cmd` fake `moon`.
- **CI:** `windows-latest` joins the matrix, plus a cross-platform end-to-end job on the example workspace.
- **Docs and config:**
  - the root, extension, and toolchain READMEs, and the CI comments
  - `openspec/config.yaml`'s platform line
- **Release:** `feat(tilt)`, which ships Windows support as a minor version bump.
