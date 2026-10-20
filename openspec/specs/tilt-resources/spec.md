# tilt-resources Specification

## Purpose
How the Tilt extension turns moon tasks into Tilt resources: which tasks are exposed, how each one runs, and how task dependencies order them.

## Requirements

### Requirement: Task selection by tag
The extension SHALL expose a task as a Tilt resource when the task has the configured tag (default `tilt`), or when it is a persistent task in a project that has the tag. With `includePersistent: true`, it SHALL expose every persistent task regardless of tags.

#### Scenario: Tagged project exposes its persistent tasks
- **WHEN** project `web` is tagged `tilt` and has a persistent task `dev` and a non-persistent task `lint`
- **THEN** `web:dev` becomes a resource
- **AND** `web:lint` does not

#### Scenario: Individually tagged task
- **WHEN** project `api` is not tagged but its task `serve` is tagged `tilt`
- **THEN** `api:serve` becomes a resource

### Requirement: Tasks that cannot run under Tilt are excluded
The extension MUST NOT expose interactive tasks, or tasks that run Tilt itself (command `tilt`, or the `tilt` toolchain), even when tagged.

#### Scenario: Interactive task is tagged
- **WHEN** an interactive task is tagged `tilt`
- **THEN** it is not a resource

#### Scenario: Task runs Tilt
- **WHEN** a persistent task's command is `tilt up`
- **THEN** it is not a resource, and no other resource depends on it

### Requirement: Resources run tasks through moon
Each resource SHALL be named after the task's target and SHALL run `moon run <target> --upstream none` from the workspace root, with `MOON_SKIP_SYNC_WORKSPACE=true`, so moon handles caching, toolchains, and environment while Tilt handles ordering. Persistent tasks SHALL run as `serve_cmd`; other tasks as `cmd` with `allow_parallel = True`. Each resource SHALL be labelled with its project ID.

#### Scenario: Persistent task
- **WHEN** `web:dev` is a persistent task
- **THEN** its resource is `local_resource("web:dev", serve_cmd = ["moon", "run", "web:dev", "--upstream", "none"], ...)` labelled `web`

### Requirement: Task dependencies become resource dependencies
A resource SHALL depend (`resource_deps`) on the resources of its task dependencies, and those dependencies SHALL be exposed transitively even when not tagged.

#### Scenario: Untagged dependency is pulled in
- **WHEN** `web:dev` is exposed and depends on the untagged task `web:codegen`
- **THEN** `web:codegen` is also a resource
- **AND** `web:dev` has `resource_deps` on `web:codegen`

### Requirement: Parameterized dependencies run as moon would
When a task dependency declares `args` or `env`, the extension SHALL create a dedicated resource for that parameterization, named `<target>[<args> <KEY=value>...]`, running `moon run <target> --upstream none -- <args>` with the env added. The dependent SHALL depend on it, and the plain resource SHALL only exist if something runs the target without parameters.

#### Scenario: Dependency with args and env
- **WHEN** `web:dev` depends on `codegen` with `args: ["--prod"]` and `env: { MODE: "x" }`
- **THEN** a resource `web:codegen[--prod MODE=x]` runs `moon run web:codegen --upstream none -- --prod` with `MODE=x`
- **AND** `web:dev` depends on it
- **AND** no plain `web:codegen` resource exists unless another task depends on it without parameters
