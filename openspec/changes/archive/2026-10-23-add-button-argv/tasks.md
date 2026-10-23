# Tasks

## 1. The `argv` setting

- [x] 1.1 Add `argv: Option<Vec<String>>` to `ButtonSettings` and a check in `parse_buttons` that a set list isn't empty, so a non-list, a non-string element, or `[]` fails generation naming the project and `<task>.argv`. Verify with unit tests in `buttons.rs` for a string value, a number element, and an empty list, and that `["--noinput", ""]` is accepted
- [x] 1.2 Add `argv: Vec<String>` to `Button` and append `--` plus the fixed arguments to its `command` (after `--yes` for a confirmed task) in `plan_buttons`. Verify with unit tests that a button without `argv` keeps today's command, one with `argv` gets `… exec <target> -- --noinput`, and a confirmed one gets `… exec <target> --yes -- --noinput`, and with a render test that the generated `cmd_button` `argv` contains them
- [x] 1.3 Document `argv` in the extension README (the exec-buttons section with a `--noinput` example next to a task that prompts on the host, the project-settings table, and `argv` in the `buttons` template variable) and add **button argv** to `GLOSSARY.md`'s exec-buttons table, distinguishing it from Tilt's `cmd_button` `argv`. Verify the README example matches a unit-test fixture

## 2. Combining arguments in `exec`

- [x] 2.1 In `exec_task`, pass the arguments after `--` first, then `MOON_TILT_ARGS` split into arguments when it's set, for host, Kubernetes, and Compose alike. Move the combination into a pure function in `exec.rs` and verify with unit tests: only `--` args, only the variable, both (in that order), an empty variable, and an invalid variable still failing with its error
- [x] 2.2 Update the README's "Running a task where Tilt runs it" section and the `exec` command's `args` help text to say input arguments come after arguments given after `--`. Verify the documented invocation runs as written with a fake moon in a WASM integration test (`exec web:<task> -- a` with `MOON_TILT_ARGS=b` runs `moon run web:<task> -- a b`)

## 3. Verification

- [x] 3.1 With real moon and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, give a host task and a Kubernetes-containerized task that read stdin (or print their args) `argv: ["--noinput"]` and an `args` input. Verify the Tiltfile loads (`tilt alpha tiltfile-result`), clicking each button in Tilt's UI runs the task with `--noinput` followed by the typed arguments, and `moon run` of the same task on the host gets neither (done with a Compose workload in place of Kubernetes, since kind couldn't start pods in the session's container; `exec` combines arguments before choosing Kubernetes or Compose)
- [x] 3.2 Run `moon run :lint :test root:format root:openspec` and verify it passes, with the change validating in strict mode

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-exec-buttons` spec.
- Commit as `feat(tilt): add fixed argv to exec buttons` and open a PR that closes #43.
