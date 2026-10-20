# Design

## Context

The planner treats a task as running Tilt when its command is `tilt` or it uses the `tilt` toolchain. It skips such tasks when selecting, and when walking task dependencies. But a selected task's resource copies *every* task dependency into `resource_deps`, Tilt ones included.

Verified against Tilt 0.37.8: a `local_resource` whose `resource_deps` names a resource that doesn't exist loads without an error or warning (exit 0), and the dependency is simply dropped. So a task that follows `tilt up` starts immediately and unordered. See proposal.md for the user impact.

## Goals / Non-Goals

**Goals:**
- Leave out every task whose dependency chain reaches a task that runs Tilt, and make that visible in the generated file.

**Non-Goals:**
- Detecting Tilt-adjacent tasks by what they do (e.g. a command that opens `localhost:10350`). Only the dependency graph is used.
- Excluding tasks that depend on *interactive* tasks. That's a separate case that hasn't been reported.
- An opt-out setting. See the decision below.

## Decisions

**Exclusion follows dependencies transitively, not just direct ones.** Issue #4 proposes direct dependencies only. But then `notify` → `tilt-open` → `tilt-up` would keep `notify`, with a `resource_deps` entry on the now-excluded `tilt-open`: the same silent drop, one level down. The rule is "a task is excluded if any dependency runs Tilt or is itself excluded for this reason", computed once over the task graph before selection.

**Excluded even when tagged, like tasks that run Tilt.** A task that depends on Tilt running is meant to run alongside `tilt up` from moon, not as a resource inside Tilt, whatever selected it. Keeping tagged ones would preserve exactly the reported bug for anyone who tags the follow-up task.

**Not opt-in, despite the project rule for surprising behavior.** The current behavior is the defect: the dependency is dropped silently and the task runs out of order. No working setup depends on that. The generated comment names every task excluded this way, so the change is visible in the file moon already shows in `git diff`. It ships as `fix(tilt)`.

**The generated file lists only tasks that would otherwise have been resources.** Listing every Tilt-adjacent task in the workspace would be noise. Only tasks that the selection (tags, `includePersistent`, or a dependency of a selected task) would have exposed are listed. Each entry names the task and the dependency that caused it, e.g. `web:tilt-open (depends on web:tilt-up, which runs Tilt)`. The list is a comment in the `resources` block, and is exposed to templates as a new `excluded` variable (`target`, `dependency`). Adding a variable doesn't break custom templates.

## Risks / Trade-offs

- [A task with a stray dependency on a Tilt task disappears from Tilt] → The generated comment names it and the dependency. Removing that dependency in `moon.yml` brings it back.
- [Large task graphs] → The exclusion set is computed once, in linear time over task dependencies, with no effect on generation time in practice.

## Migration Plan

Nothing to migrate. Release as `fix(tilt)` (patch). Workspaces with follow-up tasks see them leave Tilt on the next generation, listed in the generated file.
