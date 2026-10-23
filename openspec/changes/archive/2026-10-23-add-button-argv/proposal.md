# Proposal

## Why

Issue #43: an exec button runs `moon ext tilt -- exec <target>` with no arguments except what someone types into its optional `args` input. Buttons have no terminal, so a task that prompts (Django's `flush`) fails from a button unless the task itself is always non-interactive. Building `--noinput` into the task also removes the prompt from `moon run` on the host, which is the safeguard a destructive task should keep. Today's workaround, an environment variable only the workload's container sets, works but hides what the button does in a manifest.

## What Changes

- A new per-button setting, `project.tilt.buttons.<task>.argv`: a list of fixed arguments the button always passes to its task after `--`, ahead of anything typed into its `args` input. The button's command in the generated Tiltfile shows them, so what a button runs stays visible next to the task.
- `exec` passes arguments given after `--` first, then the button input's arguments (`MOON_TILT_ARGS`) after them, instead of ignoring the input whenever arguments follow `--`. Only buttons with an `args` input set that variable, so a terminal run is unchanged.
- `argv` that isn't a list of strings fails generation, naming the project and the setting, like the other button settings.
- The `buttons` template variable gains each button's `argv`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-exec-buttons`: buttons can carry fixed arguments; how `exec` combines arguments after `--` with a button input's arguments; validation of the new setting; the template variable's fields.

## Impact

- **Code**: `extensions/tilt/src/buttons.rs` (setting, validation, command), `exec.rs`/`tilt_ext.rs` (combining arguments), the README's exec-buttons, project-settings, and template-variables sections, and `GLOSSARY.md` (button argv).
- **Behavior**: opt-in. A button without `argv` renders exactly as before. The one change in `exec` only matters when arguments follow `--` and `MOON_TILT_ARGS` is set, which no button produced before this change. Released as `feat(tilt)` (minor).
- **Out of scope**:
  - fixed environment variables per button (`env`, the issue's alternative): reaching a workload's process would need `kubectl exec`/`docker exec` to carry them, and `argv` covers the issue's case
  - per-input named flags
