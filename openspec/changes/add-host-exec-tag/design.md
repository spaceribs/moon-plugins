# Design

## Context

- `plan_buttons` (`buttons.rs`) makes a button for each task whose tags contain `execTag`, attaches it to the project's first declared workload (else its first local resource, else the navigation bar), and sets `confirm` from `confirmed_task`.
- `exec::resolve` picks `Location::Kubernetes`/`Compose` from the project's first declared workload, else `Location::Host`, after the confirmation check.
- `button_settings_errors` rejects `project.tilt.buttons` entries for tasks without `execTag`.

## Goals / Non-Goals

**Goals:**
- One button mechanism for both kinds of action, with the same settings and confirmation.

**Non-Goals:**
- Running a task in a workload other than the first, or choosing per click.

## Decisions

**A second tag, as the issue proposes.** A per-task `project.tilt.buttons.<task>.host: true` would also work, but settings live per project, while tags can come from a shared task file (`.moon/tasks/*.yml`) that many projects inherit. The tag also shows up in `moon query tasks`, like `confirmTag`.

**The location is decided in `resolve`, from the tag.** `resolve` checks `hostExecTag` before looking at workloads, so a terminal run (`moon ext tilt -- exec api:rollout-env`) and a button click agree. The button's command doesn't change: it stays `moon ext tilt -- exec <target> …`, and a terminal user can't accidentally run a host task in the pod.

**Host buttons attach like any other button.** Its project's first workload is still where the action is about, so the button sits on that resource. Its location affects only where `exec` runs it.

**Both tags: host wins.** A task tagged both is most likely a shared task file adding `tilt-exec-host` to a task a project already tagged `tilt-exec`. Running on the host is the more specific declaration. One button is generated either way, since buttons are keyed by target.

**Validation accepts either tag** in `button_settings_errors`.

## Risks / Trade-offs

- [A host task assumes tools or credentials the developer's machine has] → That's the point of the tag. The README says host tasks run with the developer's PATH, kubeconfig, and credentials, so they act with the developer's permissions.
- [`hostExecTag` collides with an existing tag in a workspace] → It's configurable, like `execTag`.

## Migration Plan

Opt-in. A workspace with hand-written host `cmd_button`s tags those tasks `tilt-exec-host` and deletes its own buttons. Release as `feat(tilt)` (minor).
