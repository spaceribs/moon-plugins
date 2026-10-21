# Design

## Context

What's already platform-neutral:
- **The plugins themselves** run as WASM inside moon, and moon's queries return workspace-relative paths with `/`.
- **The generated Tiltfile** runs every task as an argv list (`["moon", "run", "<target>", "--upstream", "none"]`), so local resources need no shell.
- **The proto tool** already downloads Windows builds and exposes `tilt.exe`.

What isn't:
- **The toolchain lookup** (`toolchain.rs`, from `run-installed-tilt`) runs `/proto/tools/tilt/<version>/tilt`. On Windows the file is `tilt.exe`.
- **Paths in the generated Tiltfile.** `_moon_path()` uses Starlark's `os.path.join`, which is Go's `filepath.Join` and produces `\` on Windows. That's fine for `deps`. It's unverified for `ignore` and `watch_settings(ignore=...)` patterns, which Tilt matches with dockerignore-style globs (`**/node_modules`, `!` negations).
- **The WASM integration tests** are `#[cfg(unix)]`, because they fake `moon` with a `#!/bin/sh` script.
- **Image builds** use a shell string (`moon docker file … && docker build -t $EXPECTED_REF …`), which Tilt runs through `cmd` on Windows, where `$EXPECTED_REF` isn't expanded.
- **CI** runs Ubuntu and macOS only.

There's no Windows machine available. Everything is verified through GitHub's `windows-latest` runners, which have Git Bash, Node, and Docker for Windows containers only.

## Goals / Non-Goals

**Goals:**
- Core features work natively on Windows x64, proven by CI with real moon and Tilt, including change detection.

**Non-Goals:**
- Image builds and container workloads on native Windows. CI can't build Linux images there, and a cmd-compatible command would ship unverified. WSL2 covers them.
- Windows arm64, which Tilt doesn't publish.
- Developing *this repository* on Windows: the `commit-msg` hook runs `sh`, which Git for Windows provides, but it isn't tested.

## Decisions

**Pick the executable name from the host OS.** The extension asks warpgate's `get_host_environment()` for the OS. The pure `tilt_executable` takes the binary name (`tilt` or `tilt.exe`) as a parameter, so both are unit-tested on any platform. Alternative: probe for `tilt.exe` in the install directory, which costs a file check per call and hides the intent.

**Keep native paths in the Tiltfile, and prove them with an end-to-end check.** Starlark's `os.path.join` gives Tilt the paths it expects on each OS, and Tilt's own Windows support is built on native paths. The end-to-end check edits:
- an input file, which must re-run
- a gitignored output, which must not
- a file that isn't an input, which must not

That's on Windows. If ignore patterns don't match with `\`, `_moon_path()` switches to forward slashes on every OS (`_MOON_ROOT.replace("\\", "/") + "/" + path`), which Tilt accepts on Windows. The check runs again to confirm. The template doesn't branch on the platform either way.

**One end-to-end script, run on all three OSes.** The check runs in a bash script (`scripts/e2e-tilt.sh`), using Git Bash on Windows, against `examples/tilt-demo`, which already has the cases needed:
- `web:codegen`, with inputs and gitignored outputs
- tagged and untagged projects
- the Tilt toolchain, which also tests the `tilt.exe` lookup on Windows

It:
1. Builds the plugins.
2. Runs `moon setup`, then `moon ext tilt -- generate`.
3. Runs `moon ext tilt -- ci web` to check the resources come up.
4. Starts `moon ext tilt -- up web` in the background.
5. Edits the three kinds of file and checks each resource's build count with `tilt get uiresource`, using the toolchain's installed binary.

Running the same script on Linux and macOS keeps platforms comparable and catches regressions everywhere.

**The WASM integration tests get a Windows fake `moon`.** That's a `fake-moon.cmd` that prints the captured JSON, used as `moonBin` on Windows. Only the permission-setting stays Unix-only. Rust's `Command` runs `.cmd` files directly.

**Containers are documented, not changed.** The READMEs say that image builds need Linux, macOS or WSL2, and the Windows CI job doesn't configure images.

## Risks / Trade-offs

- [The end-to-end job is slow or flaky (Tilt startup, file-watch timing)] → It polls with timeouts, not fixed sleeps, and reports which edit failed with the resource's build history.
- [Tilt's Windows path handling differs from what CI shows for paths with spaces or long paths] → Out of scope for the first pass. The runner's workspace path is typical.
- [Release builds run on Ubuntu only] → The WASM module is platform-independent, so nothing changes there.

## Migration Plan

Nothing to migrate. Release as `feat(tilt)` (minor bump), with the platform change in the changelog.
