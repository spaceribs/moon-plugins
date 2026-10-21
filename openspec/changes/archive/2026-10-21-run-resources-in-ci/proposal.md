# Proposal

## Why

Every Tilt resource runs its task with `moon run <target> --upstream none`. When `CI` is set, moon skips tasks whose `runInCI` is `false`, which is the default for persistent tasks (`preset: server`). The run then fails with `No tasks found. Unable to execute action pipeline.`

So `moon ext tilt -- ci`, the command meant for running Tilt in CI, can't bring up any server in CI, on any OS. The new end-to-end CI check found this on PR #11 (`db:serve` failed on Ubuntu and macOS), and it reproduces locally with `CI=true`.

Tilt has already decided which tasks run, so moon's CI filter shouldn't second-guess it.

## What Changes

- Resources run `moon exec <target> --upstream none --on-failure bail --ignore-ci-checks`. In moon 2.6.2, `moon run` is `moon exec` with `--on-failure bail` (`commands/run.rs`). So this behaves exactly as before (caching, toolchains, environment, exit codes), minus the CI skip.
- Parameterized dependency resources keep passing their args after `--`, which `moon exec` supports.
- The README's description of how resources run is updated.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-resources`: "Resources run tasks through moon" and "Parameterized dependencies run as moon would" change the command they specify. The first also gains a CI scenario.

## Impact

- **Code**: `extensions/tilt/src/tiltfile.rs` (the resource command), and the unit and render tests that assert it.
- **Behavior**: generated Tiltfiles change their resource commands. Outside CI, nothing behaves differently. In CI, persistent tasks now run under Tilt. Released as `fix(tilt)` (patch).
- **Docs**: extension README.
- **Unblocks**: the Windows PR (#11), whose end-to-end check runs `moon ext tilt -- ci` in CI.
