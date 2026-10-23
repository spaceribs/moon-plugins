# Design

## Context

- `plan_buttons` (`buttons.rs`) builds each button's `command`: `moon ext tilt -- exec <target>`, plus `--yes` for a `confirmTag` task. `buttons.tera` renders it as `cmd_button(argv = …)`, so anything in `command` is visible in the generated Tiltfile.
- `ButtonSettings` is parsed with serde (`deny_unknown_fields`). `parse_buttons` adds a non-empty check per string field, and `button_settings_errors` reports every problem before planning.
- `exec_task` (`tilt_ext.rs`) takes the arguments after `--`, and only when there are none reads `MOON_TILT_ARGS`, which Tilt sets from a button's `text_input`. Tilt passes input values only as environment variables.

## Goals / Non-Goals

**Goals:**
- What a button passes is declared next to the task's other button settings and readable in the generated Tiltfile.
- Fixed arguments and the free-text input work together on one button.

**Non-Goals:**
- Per-button environment variables. See proposal.md (Out of scope).
- Changing how the input's value is split.

## Decisions

**Fixed arguments go in the button's command, after `--`.** The button runs `moon ext tilt -- exec <target> [--yes] -- <argv…>`. `exec` already passes arguments after `--` to the task, so a terminal run of the same command behaves exactly like the button, and the arguments show up in the Tiltfile and in Tilt's log for the click. The alternative, a second environment variable (`MOON_TILT_ARGV`) set through `cmd_button(env = …)`, keeps the command short but puts the behavior back where the issue's workaround hid it.

**`exec` appends the input's arguments after `--` arguments, instead of using one or the other.** With fixed arguments in the command, the old rule ("`MOON_TILT_ARGS` only when nothing follows `--`") would drop the input on any button that has both. Appending is the order the issue asks for (fixed first, then typed). Before this change no button produced both, and Tilt only sets `MOON_TILT_ARGS` for a button with an input, so a terminal run without that variable is unchanged. Alternative: a hidden flag such as `--args-from-env` that buttons pass. It keeps the old rule intact for a case nobody hits, at the cost of another flag on every input button.

**The setting is named `argv`, as the issue proposes.** It's a list of strings, not a single string split like the input, so no quoting rules apply and `""` can be passed as an argument. The name reads as "the argument vector the button adds". It's deliberately not `args`, which is already the input's label. Tilt's `cmd_button(argv = …)` is the whole command, which contains these, so the glossary entry says which it means.

**Validation.** `argv: Option<Vec<String>>` on `ButtonSettings`, so serde rejects a non-list or a non-string element with an error `button_settings_errors` already prefixes with the project and task. `parse_buttons` adds a check that the list isn't empty, since an empty list is most likely a mistake and means the same as leaving it out. Individual elements may be empty strings.

**Template variable.** `Button` gains `argv: Vec<String>` (empty when unset). `command` already contains it, so the built-in section doesn't change. Section overrides that build their own command can use `argv`.

## Risks / Trade-offs

- [Someone runs `exec … -- <args>` from a shell where `MOON_TILT_ARGS` is exported] → The input's arguments are now appended. The variable is the extension's own and only Tilt sets it, so this needs a deliberate export. The README documents the order.
- [Fixed arguments a task doesn't understand] → The task fails on the click, with its error in Tilt's log, the same as a bad value typed into the input.

## Migration Plan

Opt-in: generation output only changes for buttons that set `argv`. A workspace using the environment-variable workaround can set `argv` and drop the variable from its manifest. Release as `feat(tilt)` (minor).
