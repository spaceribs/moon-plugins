# Spec Delta

## ADDED Requirements

### Requirement: Host exec tag
Each task tagged with `hostExecTag` (default `tilt-exec-host`) SHALL get a button as a task tagged with `execTag` does: attached to where its project runs, with the same confirmation and `project.tilt.buttons` settings. `exec` SHALL run such a task on the host, from the workspace root, whether or not its project declares workloads and whether or not Tilt is running. A task tagged with both tags SHALL run on the host.

#### Scenario: Host action on a containerized project
- **WHEN** `api` declares `workloads: { k8s: ["api"] }` and its task `api:rollout-env` is tagged `tilt-exec-host`
- **THEN** the generated Tiltfile has an `api:rollout-env` button on the `api` resource, and clicking it runs `moon run api:rollout-env` on the host, not in the `api` pod

#### Scenario: Host tag from a terminal
- **WHEN** the user runs `moon ext tilt -- exec api:rollout-env` while Tilt isn't running
- **THEN** `moon run api:rollout-env` runs on the host

#### Scenario: Both tags
- **WHEN** a task of a containerized project is tagged `tilt-exec` and `tilt-exec-host`
- **THEN** it gets one button, and `exec` runs it on the host

## MODIFIED Requirements

### Requirement: exec runs the task in the project's workload
`moon ext tilt -- exec <project>:<task>` SHALL run `moon run <project>:<task>` inside the project's first declared workload, as the running Tilt instance deployed it, for a task not tagged with `hostExecTag`: in a running pod of a Kubernetes workload, or in the container of a Compose service. It MUST fail, naming the workload, when Tilt isn't running or the workload has no running pod or container.

#### Scenario: Kubernetes workload
- **WHEN** Tilt runs `api`'s `api` workload and the user runs `moon ext tilt -- exec api:db-migrate`
- **THEN** `moon run api:db-migrate` runs in a running pod of the `api` workload, in its namespace, and its output and exit code are the command's

#### Scenario: Compose service
- **WHEN** `api` declares `workloads: { compose: ["api"] }` and Tilt runs it
- **THEN** `moon run api:db-migrate` runs in the `api` service's container

#### Scenario: Workload not running
- **WHEN** Tilt isn't running, or the `api` workload has no running pod
- **THEN** the command fails with an error naming the `api` workload

### Requirement: Invalid button settings fail generation
Generation SHALL fail, naming the project and the setting, when `project.tilt.buttons` isn't an object of objects, an entry has a key other than `text`, `icon`, `args`, `argv`, or `env`, a value breaks the rules for button setting values, or an entry names a task the project doesn't have or that isn't tagged with `execTag` or `hostExecTag`.

#### Scenario: Settings for an untagged task
- **WHEN** `project.tilt.buttons.db-migrate` is set but `api:db-migrate` isn't tagged `tilt-exec` or `tilt-exec-host`
- **THEN** generation fails, naming `api` and `db-migrate`

#### Scenario: Unknown key
- **WHEN** `project.tilt.buttons.db-migrate` is `{ confirm: true }`
- **THEN** generation fails, naming `api` and `confirm`

#### Scenario: Settings for a host button
- **WHEN** `project.tilt.buttons.rollout-env` is `{ text: "Roll out env" }` and `api:rollout-env` is tagged only `tilt-exec-host`
- **THEN** generation succeeds and the button is labelled `Roll out env`
