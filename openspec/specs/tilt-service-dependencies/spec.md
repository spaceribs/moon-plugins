# tilt-service-dependencies Specification

## Purpose
How the Tilt extension uses moon's project graph to decide which services wait on which, and to bring up the services a selected service needs.

## Requirements

### Requirement: Services wait on upstream services
When a service's project depends on another project (explicit `dependsOn` or toolchain-inferred), the service SHALL get `resource_deps` on that project's services. Non-persistent resources SHALL only follow their task dependencies.

#### Scenario: Direct project dependency
- **WHEN** project `api` depends on project `db`, and both have services
- **THEN** `api:serve` has `resource_deps` on `db:serve`

### Requirement: Libraries are walked through
When a depended-on project has no services, the extension SHALL continue to that project's own dependencies until it reaches projects that have services.

#### Scenario: Service behind a library
- **WHEN** `web` depends on `api-client` (no services), which depends on `api`
- **THEN** `web:dev` has `resource_deps` on `api:serve`

### Requirement: Upstream services are included
By default, the extension SHALL expose the upstream services of every exposed service even when they aren't tagged. With `includeUpstreamServices: false`, it SHALL only link to services that are already exposed.

#### Scenario: Untagged upstream service
- **WHEN** `api:serve` is exposed and depends on the untagged project `db`
- **THEN** `db:serve` is exposed and `api:serve` waits on it

#### Scenario: Inclusion disabled
- **WHEN** `includeUpstreamServices` is `false` and `db:serve` is not otherwise exposed
- **THEN** `db:serve` is not a resource and `api:serve` has no dependency on it

### Requirement: Only runtime dependency scopes link services
The extension SHALL follow only project dependencies whose scope is in `dependencyScopes` (default `production`, `development`, `peer`).

#### Scenario: Build-scoped dependency
- **WHEN** `web` depends on `tooling` with scope `build`, and `tooling` has a persistent task
- **THEN** that task is not exposed or linked from `web:dev`

### Requirement: Linking can be disabled
With `linkProjectDependencies: false`, the extension MUST NOT add project-graph `resource_deps` or include upstream services.

#### Scenario: Linking disabled
- **WHEN** `linkProjectDependencies` is `false`
- **THEN** `web:dev` depends only on its task dependencies

### Requirement: Runtime-only project dependencies
A project SHALL be able to declare runtime-only dependencies on other projects in `project.tilt.dependsOn`. Each SHALL link the project's services and declared workloads to the dependency's services and workloads, and include them, exactly as a moon project dependency in `dependencyScopes` would. They MUST NOT affect image builds. An ID that isn't a moon project MUST fail generation with an error naming the project and the ID.

#### Scenario: Frontend that talks to an API without a moon dependency
- **WHEN** `web` sets `project.tilt.dependsOn: ["api"]`, has no moon dependency on `api`, and `api` has a service `api:serve`
- **THEN** `web:dev` has `resource_deps` on `api:serve`, and `moon ext tilt -- up web` also brings up `api:serve`

#### Scenario: Runtime dependency on a containerized project
- **WHEN** `web` sets `project.tilt.dependsOn: ["api"]`, and `api` declares workload `api`
- **THEN** `web`'s services and declared workloads wait on the `api` workload

#### Scenario: Images ignore runtime dependencies
- **WHEN** `web` has an image and sets `project.tilt.dependsOn: ["api"]`
- **THEN** `web`'s image neither watches nor syncs `api`'s directory

#### Scenario: Unknown project
- **WHEN** `web` sets `project.tilt.dependsOn: ["apii"]` and there's no project `apii`
- **THEN** `moon ext tilt -- generate` fails with an error naming `web` and `apii`

#### Scenario: Linking disabled
- **WHEN** `linkProjectDependencies` is `false`
- **THEN** `project.tilt.dependsOn` adds no `resource_deps`, as with moon's own dependencies
