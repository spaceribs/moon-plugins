# Spec Delta

## ADDED Requirements

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
