# Tasks

## 1. The `--load` flag

- [x] 1.1 Add `--load` to `generate`, and a function that runs `tilt alpha tiltfile-result --file <tiltfile>` piped through `tilt_command`, after a `command_exists` check for a bare `tiltBin`. Map its result in a pure function (exit 0: loaded; exit 5: Tilt's stderr without the `Loading Tiltfile at:` line; other codes: the code and Tilt's output). Verify with unit tests on stderr captured from Tilt 0.37 for each exit code
- [x] 1.2 Wire `--load` into `generate`: after writing without `--check`, and only after the staleness check passes with `--check`, writing nothing. Verify with WASM integration tests using a fake `tilt` on `PATH` that `generate --load` fails with the fake's exit-5 error, succeeds quietly on exit 0, and that `generate --check --load` with stale output fails naming the file without running the fake
- [x] 1.3 Document `--load` in the extension README: the usage block (`generate [--check] [--load]`), the generated-Tiltfiles notes, and a CI recommendation (`moon ext tilt -- generate --load`), including that loading runs the Tiltfiles' load-time commands and fetches `ext://` extensions. Add **load check** to `GLOSSARY.md`'s generated-files table. Verify the documented commands run as written against real Tilt

## 2. Verification

- [x] 2.1 With real moon and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, with no cluster configured (`KUBECONFIG` pointing at an empty file): verify `generate --load` succeeds and prints no JSON, then rename a declared workload in a `moon.yml` and verify it fails with Tilt's `k8s_resource specified unknown resource` error and a non-zero exit code
- [x] 2.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-generated-tiltfile` spec.
- Commit as `feat(tilt): add generate --load to check the Tiltfile loads` and open a PR that closes #44.
