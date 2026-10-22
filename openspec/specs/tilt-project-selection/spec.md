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

### Requirement: up and focus re-apply an unchanged selection
When `up` or `focus` leaves the running instance's Tiltfile args unchanged, they SHALL still make Tilt re-apply the selection, so edits to the generated selection (such as `project.tilt.dependsOn`) take effect. They SHALL do this by first setting the args with a `(reapply)` marker, which the generated Tiltfile drops where it parses its args (so neither the selection nor `MOON_CFG` sees it, and the same resources stay enabled in between), waiting for Tilt to start loading the Tiltfile, and then setting the real args. Changed args SHALL be set once, without the marker. A marker left in the running args (say, if setting the real args failed) SHALL NOT be treated as a project.

#### Scenario: Dependency added while running
- **WHEN** Tilt runs with `web` selected, `web` gains `project.tilt.dependsOn: ["ingress"]`, and the user runs `moon ext tilt -- focus web`
- **THEN** `ingress` is enabled and no resource is disabled in between

#### Scenario: Marker is not a selection
- **WHEN** Tilt runs with args `(reapply) web`
- **THEN** the selection is the same as for `web`, and `MOON_CFG["to-run"]` is `["web"]`

#### Scenario: Leftover marker
- **WHEN** Tilt runs with args `(reapply) web` and the user runs `moon ext tilt -- up docs`
- **THEN** the running instance's args become `web docs`

### Requirement: focus replaces the selection
`moon ext tilt -- focus <projects>` SHALL replace the running instance's selection with the given projects, keeping its `--to-edit` choice unless new edit flags are given, and SHALL start Tilt if none is running.

#### Scenario: Focus on one project
- **WHEN** Tilt runs `web` and `docs`, and the user runs `moon ext tilt -- focus api`
- **THEN** only `api` and its dependencies remain enabled
