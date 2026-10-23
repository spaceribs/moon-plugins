# Spec Delta

## MODIFIED Requirements

### Requirement: Confirmation is declared on the task
A task tagged with `confirmTag` (default `tilt-confirm`), or depending on one directly or through other task dependencies, SHALL get a button that requires confirmation in Tilt's UI. `exec` SHALL refuse to run such a task unless given `--yes`, with an error naming the task, the tagged task when it's a dependency, and the flag, and its buttons SHALL pass `--yes` once Tilt has confirmed.

#### Scenario: Destructive task from a script
- **WHEN** `api:db-reset` is tagged `tilt-exec` and `tilt-confirm`, and an agent runs `moon ext tilt -- exec api:db-reset`
- **THEN** the command fails without running the task, naming `api:db-reset` and `--yes`

#### Scenario: Destructive task from Tilt
- **WHEN** the user clicks the `api:db-reset` button and confirms
- **THEN** the task runs

#### Scenario: Confirmation tag alone
- **WHEN** a task is tagged `tilt-confirm` but not `tilt-exec`
- **THEN** it gets no button, and `exec` still requires `--yes` for it

#### Scenario: Destructive dependency
- **WHEN** `api:seed` is tagged `tilt-exec` but not `tilt-confirm`, and has `deps: ["db-flush"]`, where `api:db-flush` is tagged `tilt-confirm`
- **THEN** the `api:seed` button requires confirmation and passes `--yes`, and `moon ext tilt -- exec api:seed` without `--yes` fails without running anything, naming `api:seed`, `api:db-flush`, and `--yes`

#### Scenario: Destructive dependency of a dependency
- **WHEN** `api:seed` depends on `api:prepare`, which depends on `db:flush`, tagged `tilt-confirm`
- **THEN** the `api:seed` button requires confirmation, naming `db:flush` when `exec` refuses it
