# Spec Delta

## MODIFIED Requirements

### Requirement: Resources run tasks through moon
Each resource SHALL be named after the task's target and SHALL run `moon exec <target> --upstream none --on-failure bail --ignore-ci-checks` from the workspace root, with `MOON_SKIP_SYNC_WORKSPACE=true`, so moon handles caching, toolchains, and environment while Tilt handles ordering, in CI as well. Persistent tasks SHALL run as `serve_cmd`; other tasks as `cmd` with `allow_parallel = True`. Each resource SHALL be labelled with its project ID.

#### Scenario: Persistent task
- **WHEN** `web:dev` is a persistent task
- **THEN** its resource is `local_resource("web:dev", serve_cmd = ["moon", "exec", "web:dev", "--upstream", "none", "--on-failure", "bail", "--ignore-ci-checks"], ...)` labelled `web`

#### Scenario: Persistent task in CI
- **WHEN** `CI` is set, `db:serve` is a persistent task (so moon's `runInCI` defaults to `false`), and `moon ext tilt -- ci` brings it up
- **THEN** the `db:serve` resource runs the task instead of failing with `No tasks found`

### Requirement: Parameterized dependencies run as moon would
When a task dependency declares `args` or `env`, the extension SHALL create a dedicated resource for that parameterization, named `<target>[<args> <KEY=value>...]`, running the resource's moon command for `<target>` followed by `-- <args>`, with the env added. The dependent SHALL depend on it, and the plain resource SHALL only exist if something runs the target without parameters.

#### Scenario: Dependency with args and env
- **WHEN** `web:dev` depends on `codegen` with `args: ["--prod"]` and `env: { MODE: "x" }`
- **THEN** a resource `web:codegen[--prod MODE=x]` runs `moon exec web:codegen --upstream none --on-failure bail --ignore-ci-checks -- --prod` with `MODE=x`
- **AND** `web:dev` depends on it
- **AND** no plain `web:codegen` resource exists unless another task depends on it without parameters
