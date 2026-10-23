# Proposal

## Why

Issue #44: `generate` succeeding doesn't mean Tilt can load the result. A `project.tilt.workloads.k8s` name that no Tiltfile deploys only fails at `tilt up`, with `k8s_resource specified unknown resource`. `tilt alpha tiltfile-result` catches it in a few seconds without a cluster, but on success it prints the Tiltfile result as JSON, thousands of lines for a real workspace. Workspaces silence it with `--builtin-timings --dur-threshold 1h`, a trick that has nothing to do with what they want to check.

## What Changes

- A new flag, `moon ext tilt -- generate --load`, that loads the main Tiltfile the way Tilt does (`tilt alpha tiltfile-result`), without a cluster or a running Tilt, after generating. It prints nothing from Tilt when the Tiltfile loads, and fails with Tilt's error when it doesn't.
- `--load` combines with `--check`: `generate --check --load` fails if the output is stale, and otherwise loads it without writing anything.
- The README's CI notes recommend `generate --load` for catching Tiltfiles that fail to load before anyone runs `tilt up`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-generated-tiltfile`: adds a load check next to the staleness check.

## Impact

- **Code**: `extensions/tilt/src/tilt_ext.rs` (the flag, running `tilt alpha tiltfile-result` through the same `tilt` the other commands use), the README's usage and generated-Tiltfiles sections.
- **Behavior**: opt-in. `generate` and `generate --check` are unchanged without the flag. Released as `feat(tilt)` (minor).
- **Dependencies**: `--load` needs the `tilt` binary (`tiltBin`, or the toolchain's), like `up` and `ci`. Loading runs whatever the Tiltfiles run at load time: `local()` calls, `docker compose config` for `docker_compose()`, Helm for `helm()`, and fetching `ext://` Tilt extensions from GitHub.
- **Out of scope**:
  - a separate `check` command (the issue's alternative): `--load` sits next to `--check` on the command that already owns validation
  - passing Tiltfile args, so a load check covers only a selection; with none, everything is defined
  - checking against a live cluster
