# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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

### Requirement: Invalid button settings fail generation
Generation SHALL fail, naming the project and the setting, when `project.tilt.buttons` isn't an object of objects, an entry has a key other than `text`, `icon`, `args`, or `argv`, a `text`, `icon`, or `args` value isn't a non-empty string, an `argv` value isn't a non-empty list of strings, or an entry names a task the project doesn't have or that isn't tagged with `execTag`.

#### Scenario: Settings for an untagged task
- **WHEN** `project.tilt.buttons.db-migrate` is set but `api:db-migrate` isn't tagged `tilt-exec`
- **THEN** generation fails, naming `api` and `db-migrate`

#### Scenario: Unknown key
- **WHEN** `project.tilt.buttons.db-migrate` is `{ confirm: true }`
- **THEN** generation fails, naming `api` and `confirm`

#### Scenario: Fixed arguments as a string
- **WHEN** `project.tilt.buttons.db-flush.argv` is `"--noinput"`
- **THEN** generation fails, naming `api` and `db-flush.argv`

### Requirement: Buttons are a template section
Buttons SHALL be rendered by their own section template, overridable like the others, with each button's target, project, label, icon, resource (or none, for the navigation bar), confirmation, args label, fixed arguments, and command available to it.

#### Scenario: Overriding the buttons section
- **WHEN** `templateDir/buttons.tera` extends `moon/sections/buttons.tera` and overrides its `footer` block
- **THEN** the buttons file is the built-in section plus the footer content
