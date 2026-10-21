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
When the Tilt toolchain is configured, the Tilt extension SHALL run the Tilt binary the toolchain installed: the highest installed version that satisfies the toolchain's version, whether set in `.moon/toolchains.yml` or inherited from `.prototools`. It MUST NOT depend on `tilt` being on `PATH`, on proto's shims, or on a `tilt` plugin registered in `.prototools`. When no installed version satisfies it, the extension MUST fail with an error naming the version and `moon setup`.

#### Scenario: Toolchain not configured
- **WHEN** `.moon/toolchains.yml` has no `tilt` entry
- **THEN** `moon ext tilt -- up` runs `tilt` from `PATH`

#### Scenario: Tilt only installed by the toolchain
- **WHEN** the toolchain pins `0.37.8`, moon has installed it, `tilt` isn't on `PATH`, and `.prototools` registers no `tilt` plugin
- **THEN** `moon ext tilt -- up` runs Tilt 0.37.8

#### Scenario: Partial version
- **WHEN** the toolchain pins `0.37`, and versions `0.37.7` and `0.37.8` are installed
- **THEN** the extension runs Tilt 0.37.8

#### Scenario: Pinned version not installed
- **WHEN** the toolchain pins `0.37.8` and no installed version satisfies it
- **THEN** `moon ext tilt -- up` fails with an error naming `0.37.8` and `moon setup`

#### Scenario: .prototools registers its own Tilt plugin
- **WHEN** `.prototools` registers a different `tilt` plugin locator
- **THEN** the extension still runs the binary the toolchain installed, and the `.prototools` plugin only affects `tilt` run from the user's shell

### Requirement: Without a pinned version, the extension uses tilt from PATH
When the Tilt toolchain isn't configured, or is configured without any version, the extension SHALL use `tilt` from `PATH` and MUST NOT fail. An explicitly configured `tiltBin` other than `tilt` SHALL take precedence over the toolchain.

#### Scenario: Toolchain without a version
- **WHEN** `.moon/toolchains.yml` configures `tilt` without a `version`, and `.prototools` doesn't pin `tilt`
- **THEN** `moon ext tilt -- up` runs `tilt` from `PATH`

#### Scenario: Custom tiltBin
- **WHEN** the toolchain is configured and the extension sets `tiltBin: "/opt/tilt/bin/tilt"`
- **THEN** the extension runs `/opt/tilt/bin/tilt`

### Requirement: The version can be pinned in .prototools
When `.moon/toolchains.yml` configures the Tilt toolchain without a `version`, the toolchain SHALL use the `tilt` version from the workspace's `.prototools`, for tasks and for the extension, unless `versionFromPrototools` is `false`. A `version` in `.moon/toolchains.yml` SHALL take precedence.

#### Scenario: Version only in .prototools
- **WHEN** `.prototools` sets `tilt = "0.37.8"` and `.moon/toolchains.yml` configures `tilt` with only a `plugin`
- **THEN** a task whose command is `tilt version` runs Tilt 0.37.8, and the extension runs the same version

#### Scenario: Both set
- **WHEN** `.prototools` sets `tilt = "0.37.7"` and `.moon/toolchains.yml` sets `version: "0.37.8"`
- **THEN** Tilt 0.37.8 is used
