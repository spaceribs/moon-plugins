# Spec Delta

## ADDED Requirements

### Requirement: Button environment variables
A button whose `project.tilt.buttons.<task>.env` is set SHALL run its task with those environment variables set, for the task and every task dependency moon runs with it, wherever the task runs. They SHALL appear in the button's command in the generated Tiltfile. Running the task any other way, such as `moon run` on the host, MUST NOT receive them.

#### Scenario: Non-interactive dependency
- **WHEN** `api:seed` has `deps: ["db-flush"]`, `db-flush` runs `python manage.py flush $FLUSH_FLAGS`, `project.tilt.buttons.seed.env` is `{ FLUSH_FLAGS: "--noinput" }`, and the user clicks the `api:seed` button
- **THEN** `db-flush` runs with `FLUSH_FLAGS` set to `--noinput`, and doesn't prompt

#### Scenario: Host run keeps prompting
- **WHEN** the user runs `moon run api:seed` on the host
- **THEN** `FLUSH_FLAGS` is unset, and `db-flush` prompts as before

#### Scenario: Visible in the command
- **WHEN** `project.tilt.buttons.seed` is `{ env: { FLUSH_FLAGS: "--noinput", LOG: "1" }, argv: ["--quick"] }` and `api:seed` is tagged `tilt-confirm`
- **THEN** its button's command is `moon ext tilt -- exec api:seed --yes --env FLUSH_FLAGS=--noinput --env LOG=1 -- --quick`

### Requirement: exec sets environment variables
`moon ext tilt -- exec <target> --env KEY=VALUE`, repeatable, SHALL run the task with each variable set in its environment: in the running pod of a Kubernetes workload, in a Compose service's container, or in the process environment on the host. It MUST NOT change the environment of anything else running in the workload. It SHALL fail without running the task, naming the flag's value, when a value has no `=` or its name isn't valid.

#### Scenario: Kubernetes workload
- **WHEN** the user runs `moon ext tilt -- exec api:seed --env FLUSH_FLAGS=--noinput` and `api` runs as a Kubernetes workload
- **THEN** `moon run api:seed` runs in a running pod of `api` with `FLUSH_FLAGS` set to `--noinput`

#### Scenario: Compose service
- **WHEN** `api` runs as a Compose service
- **THEN** `moon run api:seed` runs in the service's container with `FLUSH_FLAGS` set to `--noinput`

#### Scenario: Host project
- **WHEN** `web` declares no workloads and the user runs `moon ext tilt -- exec web:seed --env FLUSH_FLAGS=--noinput`
- **THEN** `moon run web:seed` runs on the host with `FLUSH_FLAGS` set to `--noinput`

#### Scenario: Malformed variable
- **WHEN** the user runs `moon ext tilt -- exec web:seed --env FLUSH_FLAGS`
- **THEN** the command fails without running the task, naming `FLUSH_FLAGS`

## MODIFIED Requirements

### Requirement: Invalid button settings fail generation
Generation SHALL fail, naming the project and the setting, when `project.tilt.buttons` isn't an object of objects, an entry has a key other than `text`, `icon`, `args`, `argv`, or `env`, a `text`, `icon`, or `args` value isn't a non-empty string, an `argv` value isn't a non-empty list of strings, an `env` value isn't a non-empty object of strings named with letters, digits, and underscores (not starting with a digit, and not `MOON_TILT_ARGS`), or an entry names a task the project doesn't have or that isn't tagged with `execTag`.

#### Scenario: Settings for an untagged task
- **WHEN** `project.tilt.buttons.db-migrate` is set but `api:db-migrate` isn't tagged `tilt-exec`
- **THEN** generation fails, naming `api` and `db-migrate`

#### Scenario: Unknown key
- **WHEN** `project.tilt.buttons.db-migrate` is `{ confirm: true }`
- **THEN** generation fails, naming `api` and `confirm`

#### Scenario: Fixed arguments as a string
- **WHEN** `project.tilt.buttons.db-flush.argv` is `"--noinput"`
- **THEN** generation fails, naming `api` and `db-flush.argv`

#### Scenario: Invalid variable name
- **WHEN** `project.tilt.buttons.seed.env` is `{ "FLUSH-FLAGS": "--noinput" }`
- **THEN** generation fails, naming `api` and `seed.env.FLUSH-FLAGS`

#### Scenario: Variable the input owns
- **WHEN** `project.tilt.buttons.seed.env` is `{ MOON_TILT_ARGS: "--plan" }`
- **THEN** generation fails, naming `api` and `seed.env.MOON_TILT_ARGS`

### Requirement: Buttons are a template section
Buttons SHALL be rendered by their own section template, overridable like the others, with each button's target, project, label, icon, resource (or none, for the navigation bar), confirmation, args label, fixed arguments, environment variables, and command available to it.

#### Scenario: Overriding the buttons section
- **WHEN** `templateDir/buttons.tera` extends `moon/sections/buttons.tera` and overrides its `footer` block
- **THEN** the buttons file is the built-in section plus the footer content
