# Tasks

## 1. The `env` setting

- [x] 1.1 Add `env: Option<BTreeMap<String, String>>` to `ButtonSettings`, and checks in `parse_buttons` that a set object isn't empty and that each name matches `[A-Za-z_][A-Za-z0-9_]*` and isn't `MOON_TILT_ARGS`, each error naming the project and `<task>.env.<NAME>` (or `<task>.env` for an empty object). Share the name rule with `exec` as one function in `exec.rs`. Verify with unit tests in `buttons.rs` for a list value, a number value, an empty object, `FLUSH-FLAGS`, `1KEY`, and `MOON_TILT_ARGS`, and that `{ FLUSH_FLAGS: "--noinput", EMPTY: "" }` is accepted
- [x] 1.2 Add `env: BTreeMap<String, String>` to `Button` and append `--env KEY=VALUE` per variable, sorted by name, after `--yes` and before `-- <argv>`, in `plan_buttons`. Verify with unit tests that a button without `env` keeps today's command and that a confirmed one with `env` and `argv` gets `… exec api:seed --yes --env FLUSH_FLAGS=--noinput --env LOG=1 -- --quick`, and with a render test that the generated `cmd_button` `argv` contains them

## 2. `exec --env`

- [x] 2.1 Add `parse_env` to `exec.rs` (`KEY=VALUE` strings to an ordered list, splitting on the first `=`, failing on a missing `=` or an invalid name with an error naming the value) and an `env` parameter to `kubectl_args` (`… -- env KEY=VALUE … moon run …`, with no `env` when the list is empty) and `docker_args` (`exec -e KEY=VALUE … <container> moon run …`). Verify with unit tests on the existing Tilt fixtures for both commands with and without variables, a value containing `=` and spaces, and each parse error
- [x] 2.2 Add a repeatable `--env` to the `exec` command, parse it before resolving anything, and pass it to the Kubernetes and Compose commands and, on the host, to `ExecCommandInput.env`. Verify with WASM integration tests using the fake moon, extended to record a variable's value: `exec web:lint --env FAKE_ENV=a` runs with `FAKE_ENV=a`, and `exec web:lint --env FAKE_ENV` fails naming it without running moon
- [x] 2.3 Document `env` in the extension README and `GLOSSARY.md`: the exec-buttons section (an example where a task dependency takes `$FLUSH_FLAGS`, the advice to turn caching off or list the variable as an input, and the Windows host shell note), the exec section (`--env`, and that Kubernetes needs `env` in the image), the project-settings table, the `buttons` template variable, and **button env** in the glossary's exec-buttons table. Verify the README example matches a unit-test fixture

## 3. Verification

- [x] 3.1 With real moon 2.6.2 and Tilt 0.37 on a scratch copy of `examples/tilt-demo`, give a host project's `seed` task a dependency whose command prints `$FLUSH_FLAGS` (with `cache: false`) and a button with `env: { FLUSH_FLAGS: "--noinput" }`, and do the same for a containerized project (Kubernetes if a cluster can run here, else Compose). Verify that clicking each button in Tilt's UI runs the dependency with `--noinput`, that `moon run` of `seed` on the host prints nothing for it, and what moon does when the dependency also sets `FLUSH_FLAGS` in its own `env` (record the result in the README). Done with Compose, since kind can't start pods in the session's container: both buttons ran from Tilt's UI with the variables set, the Compose container's own environment stayed unset, and moon 2.6.2 let the inherited variable win over the task's own `env`
- [x] 3.2 Run `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, the workspace tests (`cargo test --workspace --no-default-features`, after building the release `.wasm` files), and `openspec validate --all --strict`, and verify they pass

## Workflow follow-up

- Archive the change (`/opsx:archive`), updating the `tilt-exec-buttons` spec.
- Commit as `feat(tilt): add env to exec buttons` on `claude/sleepy-dirac-bnj3jm`, which updates PR #45.
