# Spec Delta

## Purpose

Turns moon tasks tagged for it into Tilt buttons that run the task where its project runs (in its first declared workload, or on the host), with confirmation and passthrough args declared in moon.

## ADDED Requirements

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
Arguments after `--` SHALL be passed to the task after `--`. A button whose `project.tilt.buttons.<task>.args` is set SHALL show a text input, labelled with that value, whose contents are split into arguments the same way on every platform and passed the same way.

#### Scenario: Args from the command line
- **WHEN** the user runs `moon ext tilt -- exec api:db-migrate -- --noinput`
- **THEN** the task runs as `moon run api:db-migrate -- --noinput`

#### Scenario: Args from the button
- **WHEN** `project.tilt.buttons.db-migrate.args` is `"Flags"` and the user enters `--noinput --plan` in the button's input
- **THEN** the task runs with the arguments `--noinput` and `--plan`

#### Scenario: Button without args
- **WHEN** a `tilt-exec` task has no `args` setting
- **THEN** its button has no input and passes no arguments

### Requirement: Confirmation is declared on the task
A task tagged with `confirmTag` (default `tilt-confirm`) SHALL get a button that requires confirmation in Tilt's UI. `exec` SHALL refuse to run such a task unless given `--yes`, with an error naming the task and the flag, and its buttons SHALL pass `--yes` once Tilt has confirmed.

#### Scenario: Destructive task from a script
- **WHEN** `api:db-reset` is tagged `tilt-exec` and `tilt-confirm`, and an agent runs `moon ext tilt -- exec api:db-reset`
- **THEN** the command fails without running the task, naming `api:db-reset` and `--yes`

#### Scenario: Destructive task from Tilt
- **WHEN** the user clicks the `api:db-reset` button and confirms
- **THEN** the task runs

#### Scenario: Confirmation tag alone
- **WHEN** a task is tagged `tilt-confirm` but not `tilt-exec`
- **THEN** it gets no button, and `exec` still requires `--yes` for it

### Requirement: Button text and icon
A button's label SHALL be `project.tilt.buttons.<task>.text`, else the task's `description`, else the task ID. `project.tilt.buttons.<task>.icon` SHALL set its Material icon.

#### Scenario: Defaults
- **WHEN** `api:db-migrate` has description `Apply database migrations` and no button settings
- **THEN** its button is labelled `Apply database migrations`

#### Scenario: Custom text and icon
- **WHEN** `project.tilt.buttons.db-migrate` is `{ text: "Migrate", icon: "storage" }`
- **THEN** the button is labelled `Migrate` with the `storage` icon

### Requirement: Invalid button settings fail generation
Generation SHALL fail, naming the project and the setting, when `project.tilt.buttons` isn't an object of objects, an entry has a key other than `text`, `icon`, or `args`, a value isn't a non-empty string, or an entry names a task the project doesn't have or that isn't tagged with `execTag`.

#### Scenario: Settings for an untagged task
- **WHEN** `project.tilt.buttons.db-migrate` is set but `api:db-migrate` isn't tagged `tilt-exec`
- **THEN** generation fails, naming `api` and `db-migrate`

#### Scenario: Unknown key
- **WHEN** `project.tilt.buttons.db-migrate` is `{ confirm: true }`
- **THEN** generation fails, naming `api` and `confirm`

### Requirement: Buttons are a template section
Buttons SHALL be rendered by their own section template, overridable like the others, with each button's target, project, label, icon, resource (or none, for the navigation bar), confirmation, args label, and command available to it.

#### Scenario: Overriding the buttons section
- **WHEN** `templateDir/buttons.tera` extends `moon/sections/buttons.tera` and overrides its `footer` block
- **THEN** the buttons file is the built-in section plus the footer content
