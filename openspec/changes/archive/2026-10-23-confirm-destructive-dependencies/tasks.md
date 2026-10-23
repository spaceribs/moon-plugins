# Tasks

## 1. Transitive confirmation

- [x] 1.1 Add `confirmed_task` to `exec.rs`: the target if tagged, else the first task tagged in a depth-first walk of `deps` (skipping unknown targets, with a visited set). Use it in `resolve`, keeping the message for the target itself and naming the dependency otherwise. Verify with unit tests for the target, a direct dependency, a dependency of a dependency, a cycle, and an unknown dependency target
- [x] 1.2 Set a button's `confirm` from `confirmed_task` in `plan_buttons`. Verify with unit tests that a button for an untagged task with a confirmed dependency (direct and transitive) requires confirmation and passes `--yes`, and that one without stays unconfirmed
- [x] 1.3 Add a WASM integration test: `exec` of a task whose dependency is tagged `tilt-confirm` fails without `--yes`, naming the dependency, and runs with it. Verify it passes

## 2. Documentation

- [x] 2.1 Update the extension README: confirmation covers task dependencies (exec-buttons and exec sections), the `env` example tags `db-flush` `tilt-confirm`, `env` values aren't secret, a button's variables reach and override other projects' task dependencies, and `generate --load` executes Tiltfiles (run it without secrets or a real `KUBECONFIG` for untrusted changes). Update **confirm** in `GLOSSARY.md`. Verify the README's `env` example matches the unit-test fixture, now with the confirmation

## 3. Verification

- [x] 3.1 With real moon 2.6.2 and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, give a host task a `tilt-confirm` dependency and a button. Verify the button asks for confirmation in Tilt's UI and runs once confirmed, and that `moon ext tilt -- exec` without `--yes` refuses it naming the dependency
- [x] 3.2 Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the workspace tests (after building the release `.wasm` files), and `openspec validate --all --strict`, and verify they pass

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-exec-buttons` spec.
- Commit as `fix(tilt): require confirmation for tasks with destructive dependencies` on `claude/sleepy-dirac-bnj3jm`, which updates PR #45.
