# Design

## Context

What moon 2.6.2 does, verified against its source:
- **CI detection.** moon treats any environment with `CI`, `CI_NAME`, or `AZURE_PIPELINES` set as CI (`moon_common::is_ci`).
- **The skip.** In CI, the action graph skips tasks whose `runInCI` is `false` (`action_graph_builder.rs`), and that's the default for persistent tasks. With nothing left to run, `moon run` prints `No tasks found` and exits non-zero.
- **`moon run` is a wrapper.** It's `moon exec` with `on_failure = Bail` plus the optional affected arguments. `moon exec` has `--ignore-ci-checks` ("Ignore 'run in CI' task checks"), which `moon run` doesn't expose.

Verified locally:
- `CI=true moon run db:serve --upstream none` fails with `No tasks found`.
- `CI=true moon exec db:serve --upstream none --ignore-ci-checks` runs the server.

## Goals / Non-Goals

**Goals:**
- Resources run their task whether or not `CI` is set, with no other change in behavior.

**Non-Goals:**
- The Live Update restart entrypoint (`moon run <project>:<startTask>`) and moon's generated Dockerfile `CMD`. Both run inside containers, where the host's `CI` isn't passed through.
- Changing which tasks are *selected*. Selection already ignores `runInCI`.

## Decisions

**Run `moon exec … --on-failure bail --ignore-ci-checks`.** It reproduces `moon run` exactly, minus the CI filter:
- `--on-failure bail` matches what `moon run` sets.
- `--upstream none` keeps Tilt in charge of ordering.
- Passthrough args still follow `--`.

Alternatives:
- *`moon run --force`*: also bypasses moon's cache and affected checks, so every resource re-runs its task in full.
- *Hide `CI` from resources*: Tilt can't unset variables for a `local_resource`, and tasks that legitimately read `CI` (test runners, for example) would change behavior.
- *Ask users to set `runInCI: true` on persistent tasks*: puts the burden on every workspace, and changes how those tasks behave in `moon ci`.

**Tasks with `runInCI: "only"` run under Tilt outside CI too.** Tilt's selection already includes them, and today they'd fail to run outside CI. With `--ignore-ci-checks`, a selected task runs wherever Tilt runs, which is the consistent rule: what Tilt selects, runs.

## Risks / Trade-offs

- [`moon exec` is described as a "low-level" command, and could change independently of `moon run`] → The extension already pins moon 2.6+. A unit test asserts the exact command, and the CI end-to-end check runs it with real moon on every PR.
- [Generated Tiltfiles change for every workspace] → The change is mechanical, and the generated file is regenerated automatically on the next sync.

## Migration Plan

Nothing to migrate. Release as `fix(tilt)` (patch). Workspaces pick up the new command on their next `moon sync` or `moon ext tilt -- generate`.
