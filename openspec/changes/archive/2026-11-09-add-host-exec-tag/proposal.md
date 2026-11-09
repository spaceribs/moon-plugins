# Proposal

## Why

Issue #47: `exec` decides where a task runs per project, in its first declared workload or on the host when it has none. Some actions on a containerized project belong on the host because they act on the cluster from outside: patching a Deployment and waiting for its rollout, sending fixture requests through the ingress, reading a queue emulator with the AWS SDK. Tagged `tilt-exec`, they run in the project's pod, where the CLI, its kubeconfig, and the fixtures aren't. The workaround is hand-written `cmd_button`s next to the generated ones: two button mechanisms in one project.

## What Changes

- A workspace setting, `hostExecTag` (default `tilt-exec-host`). A task with that tag gets a button exactly like an `execTag` one (attached to where its project runs, with confirmation and `project.tilt.buttons` settings), but `exec` always runs it on the host, from the workspace root, whether or not the project declares workloads.
- A task with both tags runs on the host.
- `project.tilt.buttons` settings may configure a task tagged with either tag.
- A tag, not a per-task setting, so a shared task file (`.moon/tasks/`) can decide once for every project that inherits the task.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-exec-buttons`: a second tag for buttons that run on the host; `exec` runs tasks with it on the host; button settings accept either tag.

## Impact

- **Code**: `extensions/tilt/src/config.rs` (the setting), `buttons.rs` (planning, validation), `exec.rs` (`resolve`), the README's exec-buttons, exec, and workspace-settings sections, and `GLOSSARY.md` (host exec button).
- **Behavior**: opt-in. Nothing changes until a task carries the new tag. Released as `feat(tilt)` (minor).
- **Out of scope**: choosing a specific workload per task, other than the host.
