# Design

## Context

- A button runs `moon ext tilt -- exec <target> [--yes] [-- <argv…>]` (`plan_buttons` in `buttons.rs`). `cmd_button(env = …)` already sets the generated `_MOON_ENV` variables and the input's `MOON_TILT_ARGS`, on that `moon ext tilt` process rather than on the task.
- `exec` runs the task in one of three ways (`exec.rs`, `exec_task` in `tilt_ext.rs`):
  - Kubernetes: `kubectl [--kubeconfig …] [--context …] exec -n <ns> <pod> -- moon run …`
  - Compose: `docker exec <container> moon run …`
  - Host: `moon run …` through the PDK's `exec`, whose `ExecCommandInput.env` adds variables to the inherited environment. A name ending in `?` or `!` means something else there (set-if-unset, remove).
- moon runs a target's task dependencies in the same `moon run` process, and tasks inherit that process's environment. So a variable set on `moon run` reaches the dependencies too, which `argv` can't do.
- moon decides whether a task runs from its hash. Inherited variables aren't part of it unless the task lists them as inputs (`inputs: ["$FLUSH_FLAGS"]`), and a cached task replays its output instead of running. Tasks behind buttons usually have side effects and no outputs, so the README recommends `options: { cache: false }` (or the variable as an input) for tasks whose behavior an `env` changes.

## Goals / Non-Goals

**Goals:**
- What a button sets is declared next to its other settings and readable in the generated Tiltfile.
- The same variables reach the task in any location, without changing the workload's environment for anything else.

**Non-Goals:**
- Inputs per variable. See proposal.md (Out of scope).
- Changing moon's precedence: an inherited variable wins over the same variable in a task's own `env` (checked with moon 2.6.2), so a button's variable wins too.

## Decisions

**Buttons pass variables to `exec` as `--env KEY=VALUE` flags, not through `cmd_button(env = …)`.** The flag puts the variables in the button's command, next to `argv`, and a terminal can run the same command. `exec` needs to know which variables to forward into a container anyway. Through `cmd_button(env = …)`, they'd reach the `moon ext tilt` process, and `exec` would need a second variable listing which ones to forward (`MOON_TILT_ENV=FLUSH_FLAGS,LOG`), which hides them again. The flags come after `--yes` and before `-- <argv…>`, sorted by name so the command is stable.

**Each location sets the variables its own way, scoped to the one command:**
- Kubernetes: `kubectl exec … -- env KEY=VALUE … moon run …`. `kubectl exec` has no environment option, and `env` is POSIX and present in any image with a shell. The alternative, `sh -c 'KEY=… moon run …'`, needs quoting that changes the arguments' meaning.
- Compose: `docker exec -e KEY=VALUE … <container> moon run …`, Docker's own option.
- Host: `ExecCommandInput.env`, added to the inherited environment.

None of them touches the container's or pod's environment for other processes.

**Names are restricted to `[A-Za-z_][A-Za-z0-9_]*`.** That's what POSIX shells can expand as `$KEY`, it's the same rule for all three locations, and it keeps the PDK's `?`/`!` suffixes and `=` out of names. `MOON_TILT_ARGS` is rejected, since the button's input sets it for `exec` itself. Values can be any string, including an empty one. Generation validates `project.tilt.buttons.<task>.env` (serde rejects non-objects and non-string values; `parse_buttons` checks the names and that the object isn't empty, as for `argv`), and `exec` validates `--env` with the same rule before running anything.

**`--env` parsing and the commands are pure functions in `exec.rs`.** `parse_env` turns `KEY=VALUE` strings into an ordered list, and `kubectl_args`/`docker_args` take it, so each command is unit-tested without a cluster. The host path passes the same list to `ExecCommandInput.env`.

**Template variable.** `Button` gains `env: BTreeMap<String, String>` (empty when unset). `command` already contains the flags, so the built-in section doesn't change.

## Risks / Trade-offs

- [An image without `env`, such as a distroless one] → `kubectl exec` fails with the container's "executable file not found" error, and only for buttons that set `env`. Such an image wouldn't have the shell moon's tasks use anyway.
- [A cached task dependency doesn't run again, so its variable has no effect] → moon's caching, not the extension's. The README recommends `cache: false` or the variable as an input. Checked with moon 2.6.2: a cached dependency replayed its output under `--env`.
- [A task declares the same variable in its own `env`] → moon 2.6.2 lets the inherited variable win, so the button's value applies. The README states it.
- [Windows hosts] → moon's default shell there isn't POSIX, so `$KEY` in a host task's command doesn't expand the same way. The variable is still set, and the README says so. Containers are Linux.

## Migration Plan

Opt-in: generation output only changes for buttons that set `env`. A workspace that set the variable in its manifest can move it to the button and drop it from the manifest. Release as `feat(tilt)` (minor).
