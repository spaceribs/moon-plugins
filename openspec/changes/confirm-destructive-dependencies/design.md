# Design

## Context

- `exec::resolve` refuses a target tagged `confirmTag` without `yes`. `plan_buttons` sets a button's `confirm` (Tilt's `requires_confirmation`, and `--yes` in its command) from the same tag on the target only.
- `moon query tasks` reports each task's `deps` as resolved targets (`api:build`), and `moon run` runs all of them, transitively, before the task. The extension already has both queries in hand where it decides.

## Goals / Non-Goals

**Goals:**
- A task tagged `confirmTag` can't run from a button or `exec` without confirmation, however it's reached.

**Non-Goals:**
- Inspecting what a task's command or `env` does. The tag stays the one declaration of "destructive".

## Decisions

**One function finds the confirmed task: `confirmed_task(tasks, target, tag)`.** It returns the first task, in `moon run`'s dependency order, tagged `tag`: the target itself first, then a depth-first walk of `deps`, with a visited set so a cycle can't loop. `resolve` and `plan_buttons` both use it, so a button asks exactly when `exec` would refuse. A dependency target that isn't in `moon query tasks` is skipped, since moon would fail on it anyway.

**The error names the dependency.** For the target itself, the message stays as it is. For a dependency: "`api:seed` runs `api:db-flush`, tagged `tilt-confirm` (destructive): pass `--yes` to run it". So an agent or user sees why a task they didn't tag needs `--yes`.

**Not opt-in.** The project makes surprising behavior opt-in, but a safeguard someone has to turn on doesn't protect anyone. And the behavior it removes, running a confirmed task without confirmation, contradicts what the tag's author declared. Released as `fix(tilt)`, with the change called out in the changelog through the commit body.

**Documentation, not code, for the other review findings.** Secret `env` values, cross-project overrides, and `--load` executing Tiltfiles are about how users configure and run the extension. Blocking them in code would break legitimate uses (a shared variable for a dependency in another project; loading in a trusted CI job). The README says where each applies.

## Risks / Trade-offs

- [An existing button for a task with a confirmed dependency starts asking] → That's the fix. The README explains why.
- [A script running `exec` on such a task starts failing] → The error names the dependency and `--yes`, so the fix is one flag.
- [Tasks reached through other means, such as a dependency's `moon run` inside a task's own command] → Not visible in `deps`, so not covered. Out of scope (proposal.md).

## Migration Plan

Nothing to migrate. Scripts that `exec` a task with a confirmed dependency add `--yes`. Release as `fix(tilt)`.
