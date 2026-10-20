# tilt-project-selection Specification

## Purpose
One Tilt instance per workspace, where what runs is chosen by moon project ID from moon, the Tilt CLI, or the Tilt UI.

## Requirements

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

### Requirement: The regenerate resource stays enabled
When any selection is active, the generated Tiltfile SHALL keep the `(moon)` regenerate resource enabled.

#### Scenario: Narrow selection
- **WHEN** Tilt runs with args `api`
- **THEN** `(moon)` is still enabled

### Requirement: up adds to the running instance
`moon ext tilt -- up [projects]` SHALL start Tilt with the given projects when no instance is running on `TILT_PORT`, and SHALL add them to the running instance's selection otherwise, without starting a second instance. With no projects, it SHALL start Tilt with everything.

#### Scenario: Instance running
- **WHEN** Tilt is running with `web` selected and the user runs `moon ext tilt -- up docs`
- **THEN** the running instance's args become `web docs` and no new Tilt process starts

### Requirement: focus replaces the selection
`moon ext tilt -- focus <projects>` SHALL replace the running instance's selection with the given projects, keeping its `--to-edit` choice unless new edit flags are given, and SHALL start Tilt if none is running.

#### Scenario: Focus on one project
- **WHEN** Tilt runs `web` and `docs`, and the user runs `moon ext tilt -- focus api`
- **THEN** only `api` and its dependencies remain enabled
