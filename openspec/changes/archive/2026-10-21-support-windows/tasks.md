# Tasks

## 1. Windows-specific code

- [x] 1.1 Pass the Tilt binary name to `tilt_executable` (`tilt.exe` when warpgate's `get_host_environment()` reports Windows, `tilt` otherwise) so the toolchain lookup runs `/proto/tools/tilt/<version>/tilt.exe` on Windows; verify with unit tests for both names
- [x] 1.2 Make the WASM integration tests run on Windows with a `fake-moon.cmd` (keeping only the permission-setting Unix-only), and verify they compile and pass on Linux; Windows is verified by CI in 2.1

## 2. Windows CI

- [x] 2.1 Add `windows-latest` to the CI matrix (updating the "Windows isn't supported" comment), and verify on a pushed branch that lint, unit, and WASM integration tests pass on all three OSes
- [x] 2.2 Add `scripts/e2e-tilt.sh` and a CI job running it on Linux, macOS, and Windows, against `examples/tilt-demo`:
  - build the plugins, then run `moon setup`, `moon ext tilt -- generate` and `moon ext tilt -- ci web`
  - start `moon ext tilt -- up web` and check build counts: editing `web/schema.json` re-runs `web:codegen`, while editing gitignored `web/generated/` or a non-input file re-runs nothing

  Verify it passes on Linux and macOS, and that the Windows run uses the toolchain's `tilt.exe`
- [x] 2.3 If the Windows e2e shows ignore patterns failing with backslash paths, switch `_moon_path()` to forward-slash paths on every OS, update the render unit tests, and verify the e2e passes on all three OSes; otherwise record in this task that native paths were confirmed. **Confirmed:** CI run 39544369607 on windows-latest passed the e2e with native paths (non-input and gitignored edits re-ran nothing, an input edit re-ran `web:codegen`), so no path change was needed.

## 3. Documentation

- [x] 3.1 Update the platform statements in the root README, extension README (requirements, and that image builds need Linux, macOS, or WSL2), toolchain README, and `openspec/config.yaml`'s project context, and verify no doc still says the extension is Linux/macOS-only

## 4. Verification

- [x] 4.1 Run `moon run :lint :test root:format root:openspec` locally and verify it passes, with the change validating in strict mode, and that the PR's CI (including the Windows jobs) is green

## Workflow follow-up

- Rebase `feat/windows-support` onto `main` once PR #10 merges (it builds on `toolchain.rs`).
- Archive the change (`/opsx:archive`), creating `openspec/specs/tilt-platform-support`.
- Commit as `feat(tilt): support native Windows for the core extension and toolchain`, and open a PR.
