# Tasks

## 1. The tag

- [x] 1.1 Add the workspace setting `hostExecTag` (default `tilt-exec-host`) to `config.rs`, with the README's workspace-settings table. Verify with a unit test that the default deserializes and can be overridden
- [x] 1.2 Plan buttons for tasks tagged `execTag` or `hostExecTag` in `plan_buttons` (one per task), and accept either tag in `button_settings_errors`. Verify with unit tests that a host-tagged task in a containerized project gets a button on its first workload, that a task with both tags gets one, and that settings for a host-tagged task are accepted
- [x] 1.3 In `exec::resolve`, return `Location::Host` for a task tagged `hostExecTag`, before looking at workloads, after the confirmation check. Verify with unit tests for a containerized project's host-tagged task, a task with both tags, and that an untagged task still resolves to its workload
- [x] 1.4 Add a WASM integration test: `exec` of a host-tagged task in a project with declared workloads runs `moon run` on the host with the fake moon, without querying Tilt. Verify it passes
- [x] 1.5 Document the tag in the extension README (exec-buttons and exec sections, including that host tasks act with the developer's tools and credentials) and add **host exec button** to `GLOSSARY.md`'s exec-buttons table and the translation table's tags row. Verify the README example matches a unit-test fixture

## 2. Verification

- [x] 2.1 With real moon 2.6.2 and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, give a Compose-containerized project a `tilt-exec-host` task that prints its hostname and a `tilt-exec` task that does the same. Verify both buttons sit on the workload, the host one runs on the host and the other in the container, when clicked in Tilt's UI
- [x] 2.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the workspace tests (after building the release `.wasm` files), and `openspec validate --all --strict`, and verify they pass

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-exec-buttons` spec.
- Commit as `feat(tilt): add a host exec tag for buttons that run on the host`, closing #47.
