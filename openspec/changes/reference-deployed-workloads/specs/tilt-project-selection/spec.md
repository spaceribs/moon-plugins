# Spec Delta

## MODIFIED Requirements

### Requirement: Project IDs are Tiltfile args
With `selectByProject` enabled (default), the generated Tiltfile SHALL accept moon project IDs as positional Tiltfile args (`to-run`): a project ID selects that project's resources (its local resources, its declared workloads, and any `project.tilt.resources`), other values pass through as Tilt resource names, and Tilt enables the selected resources' dependencies. With no args, everything SHALL be enabled.

#### Scenario: Select one project
- **WHEN** Tilt runs with args `web`
- **THEN** `web`'s resources and their dependencies are enabled, and everything else is disabled

#### Scenario: Project's own Tiltfile resources
- **WHEN** project `docs` sets `project.tilt.resources: ["docs-site"]`
- **THEN** selecting `docs` also enables `docs-site`

#### Scenario: Project with declared workloads
- **WHEN** project `api` declares `workloads: { k8s: ["api", "api-worker"] }`
- **THEN** selecting `api` enables both workloads, and their dependencies

#### Scenario: Selection disabled
- **WHEN** `selectByProject` is `false`
- **THEN** the generated Tiltfile doesn't call `config.parse()`
