# Proposal

## Why

`project.tilt.buttons.<task>.argv` only reaches the button's own task. When the prompt comes from one of its task dependencies (a `seed` task with `deps: [db-flush, db-migrate]`), there's no way to make that dependency non-interactive for the button while it stays interactive for `moon run` on the host. Putting `--noinput` on the dependency in `deps` makes the host path non-interactive too. Environment variables do reach task dependencies, but the only way to set one for a button today is the workload's manifest, where it also applies to everything else run in the container.

## What Changes

- A new per-button setting, `project.tilt.buttons.<task>.env`: an object of environment variables set only for that button's run, for the task and every task dependency moon runs with it. A task can then take `$KEY` as an argument, which moon's default shell expands to nothing when the variable is unset, so `moon run` on the host is unchanged.
- `moon ext tilt -- exec` gains a repeatable `--env KEY=VALUE` flag that sets the variable wherever the task runs: in the running pod of a Kubernetes workload, in a Compose service's container, or in the process environment on the host. Buttons pass their `env` this way, so the variables show up in the button's command in the generated Tiltfile, next to `argv`. The flag also works from a terminal.
- Invalid `env` settings fail generation, naming the project and the setting: anything other than an object with at least one entry, a name that isn't letters, digits, and underscores (not starting with a digit), `MOON_TILT_ARGS` (which a button's input sets), or a value that isn't a string.
- The `buttons` template variable gains each button's `env`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-exec-buttons`: buttons can set environment variables for their run; `exec` sets environment variables in the task's environment wherever it runs; validation of the new setting; the template variable's fields.

## Impact

- **Code**: `extensions/tilt/src/buttons.rs` (setting, validation, command), `exec.rs` (the flag, and the variables in the `kubectl`, `docker`, and host commands), `tilt_ext.rs` (the flag), the README's exec-buttons, exec, project-settings, and template-variables sections, and `GLOSSARY.md` (button env).
- **Behavior**: opt-in. Buttons without `env` and `exec` without `--env` run exactly as before. Released as `feat(tilt)` (minor).
- **Dependencies**: on Kubernetes, the variables are set with `env`, so the workload's container needs an `env` executable, as any image with a POSIX shell has.
- **Out of scope**:
  - per-variable text inputs (the one `args` input covers free-form values)
  - a task's own `env` in `moon.yml` keeps its precedence over inherited variables, as moon decides
  - Windows host shells, where moon's default shell isn't POSIX and `$KEY` doesn't expand the same way (the README notes it)
