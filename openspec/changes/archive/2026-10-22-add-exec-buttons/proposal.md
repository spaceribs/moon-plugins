# Proposal

## Why

Issue #15: workspaces hand-write Tilt `cmd_button`s that run a moon task inside a deployed workload, e.g. `kubectl exec deploy/api -- moon run api:db-migrate -- --noinput`. One workspace has about 150 lines of them in a single Tiltfile. Each one repeats the task, the workload, and how to reach it, and whether it's destructive is only recorded in Tilt's `requires_confirmation`, which only Tilt's UI enforces.

## What Changes

- A task tagged `tilt-exec` gets a Tilt button that runs it where its project runs:
  - in the project's first declared workload, for projects with `project.tilt.workloads` (Kubernetes or Compose)
  - on the host otherwise
- A task also tagged `tilt-confirm` gets a button that asks for confirmation. The "destructive" marker lives on the moon task, where `moon query tasks` and other tooling can read it.
- Optional per-button settings under `project.tilt.buttons.<task>`:
  - `text`: the button label, defaulting to the task's `description`, then its ID
  - `icon`: a Material icon name
  - `args`: a text input whose value is passed to the task after `--`
- A new command, `moon ext tilt -- exec <project>:<task> [-- <args>]`, runs a task where Tilt runs its project. The buttons call it.
  - It refuses `tilt-confirm` tasks unless given `--yes`, so agents and scripts get the same safeguard Tilt's UI gives.
  - It works on Windows too, since the buttons don't need a POSIX shell to reach a workload.
- Invalid `project.tilt.buttons` settings fail generation, naming the project:
  - unknown keys
  - a task that doesn't exist or isn't tagged `tilt-exec`
- Workspace settings `execTag` (default `tilt-exec`) and `confirmTag` (default `tilt-confirm`) rename the tags, like `tag` does for resources.

## Capabilities

### New Capabilities

- `tilt-exec-buttons`: buttons generated from tagged moon tasks, the `exec` command they call, where a task runs (workload or host), confirmation, and per-button settings.

### Modified Capabilities

None.

## Impact

- **Code**:
  - `extensions/tilt/src`: planning the buttons, the `exec` command, settings validation, and the new workspace settings in `config.rs`
  - a new `buttons` section template, which loads Tilt's `uibutton` extension
  - README sections on buttons, project settings, workspace settings, and template variables
- **Behavior**: opt-in. Nothing changes until a task carries the tag. Released as `feat(tilt)` (minor).
- **Dependencies**: the generated Tiltfile loads `ext://uibutton` from GitHub when a button exists, as it already does for `ext://restart_process`. `exec` calls `kubectl` or `docker compose` for workloads, which are already required to deploy them.
- **Out of scope**:
  - multiple named inputs mapped to task flags (one free-form `args` input covers passthrough)
  - choosing a workload other than the first per button
  - running a button's task in a specific container of a multi-container pod (the pod's default container is used)
  - interactive tasks (buttons run without a TTY)
