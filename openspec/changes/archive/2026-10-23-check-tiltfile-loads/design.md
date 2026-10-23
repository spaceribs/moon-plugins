# Design

## Context

- `generate` (`tilt_ext.rs`) either writes the output (`sync_files`) or, with `--check`, renders it and compares it to disk without writing. Neither runs Tilt.
- `tilt_command` builds a `tilt` invocation from `tiltBin` or the Tilt toolchain's pinned binary, mapped to its host path. `run_tilt` streams it; `running_tiltfile` pipes it (`ExecCommandInput::pipe`), capturing stdout and stderr.
- `tilt alpha tiltfile-result --file <tiltfile>` (checked against Tilt 0.37.0) evaluates the Tiltfile without a cluster: `k8s_yaml()` of inline YAML loads with no kubeconfig. Its documented exit codes are 0 (loaded, result JSON on stdout), 1 (setup failure, logs on stderr), and 5 (evaluation error, logs on stderr). On exit 5, stderr is a `Loading Tiltfile at: …` line followed by the error with its location, for example `Tiltfile:1:13: k8s_resource specified unknown resource "nope". known k8s resources: `. It takes Tiltfile args after `--`, like `tilt up`.
- The host aborts the call uncatchably when exec'ing a missing command, so a binary that may be missing is checked first (`require_command`).

## Goals / Non-Goals

**Goals:**
- One command CI can run to know the Tiltfile loads, with quiet success and Tilt's own error on failure.
- The same Tilt binary as `up` and `ci`, so the check matches what developers run.

**Non-Goals:**
- Parsing the result JSON for further checks (it's an unstable API).
- Checking against a cluster or running anything Tilt would deploy.

## Decisions

**A `--load` flag on `generate`, not a new `check` command.** `generate` already owns the staleness check, and the issue's two spellings differ only in where the flag lives. As a flag it composes: `--load` alone generates then loads (the fresh-CI case, where `.moon/tilt` is gitignored and absent), and `--check --load` verifies the committed state without writing. A separate `check` command would need its own `--stale`-style switch to cover both. Alternative considered: making `--check` always load. It would make an existing CI step slower and dependent on `tilt`, which existing users didn't opt into.

**Run `tilt alpha tiltfile-result --file <tiltfile>` piped, and discard stdout.** Piping keeps the JSON out of the log, and nothing reads it. On exit 0 the command prints one line saying the Tiltfile loads. On exit 5 it fails with `<tiltfile> doesn't load in Tilt:` and Tilt's stderr, minus the `Loading Tiltfile at:` line. Any other exit code fails with the code and Tilt's output, since it's a setup problem rather than the Tiltfile's. The issue's workaround (`--builtin-timings --dur-threshold 1h`) prints nothing too, but depends on a timing threshold never being hit, and the flag's meaning is unrelated. Piping depends only on exit codes, which Tilt documents in the command's help.

**Load the main Tiltfile (`tiltfile`) with no Tiltfile args.** That's what `tilt up` loads. With no args, the generated selection enables everything, so every declared workload's decoration runs and a misnamed workload fails. The main Tiltfile's `local("moon ext tilt -- generate")` line runs again during the load; it rewrites nothing, since the output is current.

**Check for `tilt` before running it.** For a bare `tiltBin` resolved from `PATH`, `generate --load` checks `command_exists` and fails naming the binary and the `tilt` toolchain, instead of letting the host abort. A toolchain binary is resolved by path, as `tilt_command` already does.

## Risks / Trade-offs

- [`tilt alpha` commands can change between Tilt releases] → Only the command name and its documented exit codes are relied on. Verified against Tilt 0.37, the minimum supported version.
- [Loading runs Tiltfile side effects: `local()`, `docker compose config`, `helm template`, `ext://` fetches] → The same happens on every `tilt up`. The README says CI needs those tools and network access for the check. Workspaces whose Tiltfiles need a cluster at load time (`k8s_context()`-dependent logic, `helm_resource` lookups) may fail in CI for reasons that aren't a bug; the error is Tilt's, so it says why.
- [Tilt's stderr on exit 5 may include more than the error (warnings, `print()` output)] → It's shown as-is apart from the `Loading Tiltfile at:` line, which is Tilt's own framing.

## Migration Plan

Opt-in: nothing changes until a CI step passes `--load`. Workspaces using the `tiltfile-result --builtin-timings` workaround can replace it with `moon ext tilt -- generate --load`. Release as `feat(tilt)` (minor).
