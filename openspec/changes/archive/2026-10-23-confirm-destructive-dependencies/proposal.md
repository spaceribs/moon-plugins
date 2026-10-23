# Proposal

## Why

Confirmation (`confirmTag`, and `--yes` for `exec`) is checked only on the task a button or `exec` targets, never on the task dependencies `moon run` runs with it. A task tagged `tilt-confirm` has always been reachable, unconfirmed, through a button for a task that depends on it. Button `env` (`add-button-env`) makes that worse. A destructive dependency's interactive prompt used to fail safely on a button, which has no terminal. Now `env: { FLUSH_FLAGS: "--noinput" }` on an unconfirmed `seed` button flushes the database in one click.

The security review of PR #45 also found three things users need to be told: button `env` values aren't secret, a button's variables override other projects' task dependencies, and `generate --load` runs a PR's Tiltfiles.

## What Changes

- Confirmation becomes transitive: a task needs confirmation when it, or any task dependency `moon run` runs with it, is tagged `confirmTag`. Its button asks for confirmation in Tilt's UI and passes `--yes`, and `exec` refuses it without `--yes`, naming the confirmed dependency.
- The README's `env` example tags the destructive dependency `tilt-confirm`, so the button asks first.
- README notes:
  - `env` values are stored in `moon.yml` and the generated Tiltfile, printed in Tilt's log, and visible in `ps`, so they're for flags, not credentials.
  - A button's variables reach task dependencies in other projects and override their own `env`.
  - `generate --load` executes the Tiltfiles it loads, so CI should run it without secrets or a real `KUBECONFIG` for untrusted changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `tilt-exec-buttons`: confirmation covers a task's task dependencies.

## Impact

- **Code**: `extensions/tilt/src/exec.rs` (finding a confirmed task in the dependency tree, and `resolve`'s check), `buttons.rs` (a button's `confirm`), the README's exec-buttons, exec, and generated-Tiltfiles sections, and `GLOSSARY.md` (confirm).
- **Behavior**: not opt-in, as an exception to the project's rule, because an opt-in safeguard doesn't protect anyone. A button whose task depends on a `tilt-confirm` task now asks before running, and a script that runs `exec` on such a task needs `--yes`. Both were ways around the confirmation the dependency's author asked for. Released as `fix(tilt)`.
- **Out of scope**:
  - confirmation for tasks a `moon run` reaches any other way (moon's own `moon run` doesn't know the tag)
  - pinning `ext://` Tilt extensions to a version (`tilt up` already fetches them unpinned)
