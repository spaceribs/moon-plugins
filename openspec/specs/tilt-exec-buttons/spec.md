# tilt-exec-buttons Specification

## Purpose
Turns moon tasks tagged for it into Tilt buttons that run the task where its project runs (in its first declared workload, or on the host), with confirmation and passthrough args declared in moon.

## Requirements

### Requirement: Tagged tasks get a button
Each task tagged with `execTag` (default `tilt-exec`) SHALL get a Tilt button named after its target that runs `moon ext tilt -- exec <target>`. Tasks without the tag MUST NOT get one. The tag MUST NOT expose the task as a local resource by itself.

#### Scenario: Migration task
- **WHEN** task `api:db-migrate` is tagged `tilt-exec`
- **THEN** the generated Tiltfile has a button `api:db-migrate` that runs `moon ext tilt -- exec api:db-migrate`, and no `api:db-migrate` local resource

#### Scenario: Untagged task
- **WHEN** no task in the workspace carries `tilt-exec`
- **THEN** the generated Tiltfile defines no buttons and doesn't load Tilt's button extension

### Requirement: Buttons attach to where the project runs
A project's buttons SHALL attach to its first declared workload (Kubernetes before Compose, as declared), or, without workloads, to its first planned local resource. A project with neither SHALL get its buttons in Tilt's top navigation bar.

#### Scenario: Containerized project
- **WHEN** `api` declares `workloads: { k8s: ["api", "api-worker"] }` and has a `tilt-exec` task
- **THEN** the button appears on the `api` resource

#### Scenario: Host project
- **WHEN** `web` declares no workloads, has local resources `web:codegen` and `web:dev`, and has a `tilt-exec` task
- **THEN** the button appears on `web:codegen`

#### Scenario: Project with nothing in Tilt
- **WHEN** `tooling` has a `tilt-exec` task but no workloads or local resources
- **THEN** the button appears in Tilt's navigation bar

### Requirement: exec runs the task in the project's workload
`moon ext tilt -- exec <project>:<task>` SHALL run `moon run <project>:<task>` inside the project's first declared workload, as the running Tilt instance deployed it: in a running pod of a Kubernetes workload, or in the container of a Compose service. It MUST fail, naming the workload, when Tilt isn't running or the workload has no running pod or container.

#### Scenario: Kubernetes workload
- **WHEN** Tilt runs `api`'s `api` workload and the user runs `moon ext tilt -- exec api:db-migrate`
- **THEN** `moon run api:db-migrate` runs in a running pod of the `api` workload, in its namespace, and its output and exit code are the command's

#### Scenario: Compose service
- **WHEN** `api` declares `workloads: { compose: ["api"] }` and Tilt runs it
- **THEN** `moon run api:db-migrate` runs in the `api` service's container

#### Scenario: Workload not running
- **WHEN** Tilt isn't running, or the `api` workload has no running pod
- **THEN** the command fails with an error naming the `api` workload

### Requirement: exec runs host projects on the host
For a project without declared workloads, `exec` SHALL run `moon run <project>:<task>` on the host, from the workspace root, whether or not Tilt is running.

#### Scenario: Host task
- **WHEN** `web` declares no workloads and the user runs `moon ext tilt -- exec web:seed`
- **THEN** `moon run web:seed` runs on the host

### Requirement: Passthrough args
Arguments after `--` SHALL be passed to the task after `--`. A button whose `project.tilt.buttons.<task>.args` is set SHALL show a text input, labelled with that value, whose contents are split into arguments the same way on every platform and passed after any arguments that follow `--`.

#### Scenario: Args from the command line
- **WHEN** the user runs `moon ext tilt -- exec api:db-migrate -- --noinput`
- **THEN** the task runs as `moon run api:db-migrate -- --noinput`

#### Scenario: Args from the button
- **WHEN** `project.tilt.buttons.db-migrate.args` is `"Flags"` and the user enters `--noinput --plan` in the button's input
- **THEN** the task runs with the arguments `--noinput` and `--plan`

#### Scenario: Args from both
- **WHEN** `MOON_TILT_ARGS` is `--plan` and the user runs `moon ext tilt -- exec api:db-migrate -- --noinput`
- **THEN** the task runs as `moon run api:db-migrate -- --noinput --plan`

#### Scenario: Button without args
- **WHEN** a `tilt-exec` task has no `args` or `argv` setting
- **THEN** its button has no input and passes no arguments

### Requirement: Fixed button arguments
A button whose `project.tilt.buttons.<task>.argv` is set SHALL pass those arguments to its task after `--`, in order, every time it runs, ahead of any arguments from its `args` input. They SHALL appear in the button's command in the generated Tiltfile. Running the task any other way, such as `moon run` or `exec` from a terminal, MUST NOT receive them.

#### Scenario: Non-interactive button
- **WHEN** `project.tilt.buttons.db-flush.argv` is `["--noinput"]` and the user clicks the `api:db-flush` button
- **THEN** the task runs as `moon run api:db-flush -- --noinput`, and its button's command is `moon ext tilt -- exec api:db-flush -- --noinput`

#### Scenario: Host run keeps prompting
- **WHEN** the same task runs as `moon run api:db-flush` on the host
- **THEN** it gets no `--noinput` and prompts as before

#### Scenario: Fixed arguments and an input
- **WHEN** `project.tilt.buttons.db-flush` is `{ argv: ["--noinput"], args: "Flags" }` and the user enters `--database replica` in the button's input
- **THEN** the task runs with the arguments `--noinput`, `--database`, and `replica`

#### Scenario: Confirmed button with fixed arguments
- **WHEN** `api:db-flush` is also tagged `tilt-confirm`
- **THEN** its button's command is `moon ext tilt -- exec api:db-flush --yes -- --noinput`

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

### Requirement: Button text and icon
A button's label SHALL be `project.tilt.buttons.<task>.text`, else the task's `description`, else the task ID. `project.tilt.buttons.<task>.icon` SHALL set its Material icon.

#### Scenario: Defaults
- **WHEN** `api:db-migrate` has description `Apply database migrations` and no button settings
- **THEN** its button is labelled `Apply database migrations`

#### Scenario: Custom text and icon
- **WHEN** `project.tilt.buttons.db-migrate` is `{ text: "Migrate", icon: "storage" }`
- **THEN** the button is labelled `Migrate` with the `storage` icon

### Requirement: Invalid button settings fail generation
Generation SHALL fail, naming the project and the setting, when `project.tilt.buttons` isn't an object of objects, an entry has a key other than `text`, `icon`, `args`, `argv`, or `env`, a value breaks the rules for button setting values, or an entry names a task the project doesn't have or that isn't tagged with `execTag`.

#### Scenario: Settings for an untagged task
- **WHEN** `project.tilt.buttons.db-migrate` is set but `api:db-migrate` isn't tagged `tilt-exec`
- **THEN** generation fails, naming `api` and `db-migrate`

#### Scenario: Unknown key
- **WHEN** `project.tilt.buttons.db-migrate` is `{ confirm: true }`
- **THEN** generation fails, naming `api` and `confirm`

### Requirement: Button setting values
In `project.tilt.buttons.<task>`, `text`, `icon`, and `args` SHALL be non-empty strings, `argv` a non-empty list of strings, and `env` a non-empty object of strings whose names are letters, digits, and underscores, not starting with a digit, and not `MOON_TILT_ARGS`. Any other value fails generation.

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
