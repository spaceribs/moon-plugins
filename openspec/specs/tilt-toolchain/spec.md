# tilt-toolchain Specification

## Purpose
How the Tilt toolchain installs and pins a Tilt version for a moon workspace, and makes it available to tasks and to the Tilt extension.

## Requirements

### Requirement: Install and pin Tilt
The toolchain SHALL install the configured Tilt `version` through proto, by bundling the Tilt proto tool, and SHALL put it on `PATH` for tasks whose command is `tilt`.

#### Scenario: Task runs tilt
- **WHEN** `.moon/toolchains.yml` configures `tilt: { version: "0.37.8" }` and a task's command is `tilt ci`
- **THEN** moon installs Tilt 0.37.8 before the task and runs that version

### Requirement: Projects aren't Tilt-aware by their Tiltfile
The toolchain MUST NOT detect projects by their `Tiltfile`, and MUST NOT infer per-project tasks, so tasks that don't run Tilt never require installing it.

#### Scenario: Project with a Tiltfile and other tasks
- **WHEN** a project has a `Tiltfile` and an `echo` task
- **THEN** the `echo` task doesn't use the Tilt toolchain

### Requirement: The extension uses the pinned version
When the Tilt toolchain is configured, the Tilt extension SHALL run Tilt through proto's shims with `PROTO_TILT_VERSION` set to the configured version. When it isn't configured, the extension SHALL use `tilt` from `PATH` and MUST NOT fail.

#### Scenario: Toolchain not configured
- **WHEN** `.moon/toolchains.yml` has no `tilt` entry
- **THEN** `moon ext tilt -- up` runs `tilt` from `PATH`
